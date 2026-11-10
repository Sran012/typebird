// the daemon. long-lived process, holds the socket, manages state.
// commands: toggle, start, stop, cancel, state, info, key <name>
// protocol: newline-delimited text over unix socket. response is a line.
// streaming: records in chunks, transcribes each, types as they come.
use crate::audio;
use crate::inject;
use crate::seam;
use crate::transcribe;
use hound::WavWriter;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State { Ready, Listening, Transcribing }

impl State {
    pub fn as_str(&self) -> &'static str {
        match self { State::Ready => "ready", State::Listening => "listening", State::Transcribing => "transcribing" }
    }
}

fn socket_path() -> String {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/typebird.sock", runtime)
}

pub fn run() {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => { eprintln!("bind failed: {}", e); return; }
    };
    eprintln!("typebird daemon listening on {}", path);
    eprintln!("hotkey: press F9 to toggle recording");

    let state = Arc::new(Mutex::new(State::Ready));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let rec_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>> = Arc::new(Mutex::new(None));

    // hotkey listener: F9 toggles recording
    let hotkey_state = state.clone();
    let hotkey_stop = Arc::new(AtomicBool::new(false));
    let hotkey_rec = rec_handle.clone();
    let hotkey_samples_stop = Arc::new(AtomicBool::new(false));
    thread::spawn(move || {
        let mut devices: Vec<evdev::Device> = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/dev/input") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.to_string_lossy().contains("event") {
                    if let Ok(dev) = evdev::Device::open(&path) {
                        if dev.supported_keys().map(|k| k.contains(evdev::Key::KEY_F9)).unwrap_or(false) {
                            devices.push(dev);
                        }
                    }
                }
            }
        }
        if let Some(mut dev) = devices.into_iter().next() {
            loop {
                if hotkey_stop.load(Ordering::Relaxed) { break; }
                if let Ok(events) = dev.fetch_events() {
                    for ev in events {
                        if ev.event_type() == evdev::EventType::KEY && ev.code() == evdev::Key::KEY_F9.0 && ev.value() == 1 {
                            let cur = *hotkey_state.lock().unwrap();
                            if cur == State::Ready {
                                hotkey_samples_stop.store(false, Ordering::Relaxed);
                                let stop = hotkey_samples_stop.clone();
                                let st = hotkey_state.clone();
                                let h = thread::spawn(move || {
                                    recording_loop(stop, st);
                                });
                                *hotkey_rec.lock().unwrap() = Some(h);
                                *hotkey_state.lock().unwrap() = State::Listening;
                                eprintln!("hotkey: listening");
                            } else if cur == State::Listening {
                                hotkey_samples_stop.store(true, Ordering::Relaxed);
                                if let Some(h) = hotkey_rec.lock().unwrap().take() { let _ = h.join(); }
                                *hotkey_state.lock().unwrap() = State::Ready;
                                eprintln!("hotkey: stopped");
                            }
                        }
                    }
                }
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    });

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let state = state.clone();
                let stop_flag = stop_flag.clone();
                let rec_handle = rec_handle.clone();
                thread::spawn(move || handle_client(s, state, stop_flag, rec_handle));
            }
            Err(e) => eprintln!("accept error: {}", e),
        }
    }
}

fn handle_client(
    mut stream: UnixStream,
    state: Arc<Mutex<State>>,
    stop_flag: Arc<AtomicBool>,
    rec_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() { return; }
    let cmd = line.trim().to_string();

    let resp = match cmd.as_str() {
        "state" => {
            let s = *state.lock().unwrap();
            s.as_str().to_string()
        }
        "start" => {
            let mut s = state.lock().unwrap();
            if *s == State::Ready {
                stop_flag.store(false, Ordering::Relaxed);
                let stop = stop_flag.clone();
                let state = state.clone();
                let h = thread::spawn(move || {
                    recording_loop(stop, state);
                });
                *rec_handle.lock().unwrap() = Some(h);
                *s = State::Listening;
                "ok".to_string()
            } else {
                "busy".to_string()
            }
        }
        "stop" => {
            {
                let mut s = state.lock().unwrap();
                if *s != State::Listening {
                    drop(s);
                    let _ = writeln!(stream, "not listening");
                    return;
                }
                *s = State::Transcribing;
            }
            stop_flag.store(true, Ordering::Relaxed);
            if let Some(h) = rec_handle.lock().unwrap().take() {
                let _ = h.join();
            }
            *state.lock().unwrap() = State::Ready;
            let _ = writeln!(stream, "ok");
            return;
        }
        "cancel" => {
            stop_flag.store(true, Ordering::Relaxed);
            if let Some(h) = rec_handle.lock().unwrap().take() { let _ = h.join(); }
            *state.lock().unwrap() = State::Ready;
            "ok".to_string()
        }
        "toggle" => {
            let cur = *state.lock().unwrap();
            if cur == State::Ready {
                // start
                stop_flag.store(false, Ordering::Relaxed);
                let stop = stop_flag.clone();
                let state_clone = state.clone();
                let h = thread::spawn(move || {
                    recording_loop(stop, state_clone);
                });
                *rec_handle.lock().unwrap() = Some(h);
                *state.lock().unwrap() = State::Listening;
                "listening".to_string()
            } else if cur == State::Listening {
                // stop
                stop_flag.store(true, Ordering::Relaxed);
                if let Some(h) = rec_handle.lock().unwrap().take() { let _ = h.join(); }
                *state.lock().unwrap() = State::Ready;
                "stopped".to_string()
            } else {
                "busy".to_string()
            }
        }
        "info" => "typebird daemon".to_string(),
        _ => format!("unknown: {}", cmd),
    };

    let _ = writeln!(stream, "{}", resp);
}

// the recording loop. records in chunks, transcribes each, types as they come.
// cuts a chunk every ~5s. silence detection comes later.
fn recording_loop(stop: Arc<AtomicBool>, state: Arc<Mutex<State>>) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => { eprintln!("no input device"); return; }
    };

    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };

    let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_clone = samples.clone();

    let stream = match device.build_input_stream(
        &config,
        move |data: &[f32], _| {
            let mut buf = samples_clone.lock().unwrap();
            for &s in data {
                buf.push((s * 32767.0) as i16);
            }
        },
        |err| eprintln!("stream error: {}", err),
        None,
    ) {
        Ok(s) => s,
        Err(e) => { eprintln!("build stream failed: {}", e); return; }
    };

    if stream.play().is_err() {
        eprintln!("failed to play stream");
        return;
    }

    let chunk_duration = Duration::from_secs(5);
    let mut last_chunk = Instant::now();
    let mut prev_ended = true; // first chunk starts a sentence

    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(100));

        let now = Instant::now();
        if now.duration_since(last_chunk) >= chunk_duration {
            // cut a chunk
            let chunk: Vec<i16> = {
                let mut buf = samples.lock().unwrap();
                let c = buf.clone();
                buf.clear();
                c
            };

            if !chunk.is_empty() {
                prev_ended = process_chunk(&chunk, prev_ended, false);
            }
            last_chunk = now;
        }
    }

    // process remaining samples (last chunk, keep punctuation)
    let remaining: Vec<i16> = {
        let mut buf = samples.lock().unwrap();
        let r = buf.clone();
        buf.clear();
        r
    };
    if !remaining.is_empty() {
        let _ = process_chunk(&remaining, prev_ended, true);
    }

    drop(stream);
}

fn process_chunk(chunk: &[i16], prev_ended: bool, is_last: bool) -> bool {
    if chunk.len() < 8000 { return prev_ended; } // skip tiny chunks
    // rms check: skip silent chunks, whisper hallucinates on silence
    let rms: f64 = (chunk.iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / chunk.len() as f64).sqrt() / 32768.0;
    if rms < 0.005 {
        eprintln!("chunk: (silent, rms={:.4}, skipping)", rms);
        return prev_ended;
    }
    let wav_path = "/tmp/typebird-chunk.wav";
    let spec = hound::WavSpec {
        channels: 1, sample_rate: 16000, bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    if let Ok(mut w) = WavWriter::create(wav_path, spec) {
        for &s in chunk { let _ = w.write_sample(s); }
        let _ = w.finalize();
    }

    let raw = transcribe::transcribe(wav_path);
    if raw.is_empty() || raw == "[BLANK_AUDIO]" {
        return prev_ended;
    }
    // seam-hiding
    let mut text = seam::clean(&raw, prev_ended);
    // drop trailing punctuation on non-last chunks (short pause between chunks)
    if !is_last {
        let t = text.trim_end_matches(['.', '?', '!']).trim().to_string();
        if !t.is_empty() { text = t; }
    }
    if !text.is_empty() {
        eprintln!("chunk: {}", text);
        std::thread::sleep(std::time::Duration::from_millis(500));
        inject::type_text(&text);
    }
    seam::ended_sentence(&text)
}
