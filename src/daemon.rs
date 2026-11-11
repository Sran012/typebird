// the daemon. long-lived process, holds the socket, manages state.
// commands: toggle, start, stop, cancel, state, info, key <name>
// protocol: newline-delimited text over unix socket. response is a line.
// streaming: records in chunks, transcribes each, types as they come.
use crate::inject;
use crate::seam;
use crate::transcribe;
use hound::WavWriter;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Ready,
    Listening,
    Transcribing,
}

impl State {
    pub fn as_str(&self) -> &'static str {
        match self {
            State::Ready => "ready",
            State::Listening => "listening",
            State::Transcribing => "transcribing",
        }
    }
}

fn socket_path() -> String {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    format!("{}/typebird.sock", runtime)
}

// the pill lives next to the daemon binary once installed.
fn pill_binary() -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let candidate = dir.join("typebird-pill");
        if candidate.exists() {
            return candidate;
        }
    }
    std::path::PathBuf::from("typebird-pill")
}

fn ensure_pill() {
    let running = std::process::Command::new("pgrep")
        .arg("-x")
        .arg("typebird-pill")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !running {
        let _ = std::process::Command::new(pill_binary()).spawn();
    }
}

fn start_recording(
    state: &Arc<Mutex<State>>,
    stop_flag: &Arc<AtomicBool>,
    discard_flag: &Arc<AtomicBool>,
    rec_handle: &Arc<Mutex<Option<thread::JoinHandle<()>>>>,
) -> bool {
    {
        let mut s = state.lock().unwrap();
        if *s != State::Ready {
            return false;
        }
        *s = State::Listening;
    }
    stop_flag.store(false, Ordering::Relaxed);
    discard_flag.store(false, Ordering::Relaxed);
    let stop = stop_flag.clone();
    let discard = discard_flag.clone();
    let st = state.clone();
    let h = thread::spawn(move || recording_loop(stop, discard, st));
    *rec_handle.lock().unwrap() = Some(h);
    ensure_pill();
    true
}

fn stop_recording(
    state: &Arc<Mutex<State>>,
    stop_flag: &Arc<AtomicBool>,
    rec_handle: &Arc<Mutex<Option<thread::JoinHandle<()>>>>,
) -> bool {
    {
        let mut s = state.lock().unwrap();
        if *s != State::Listening {
            return false;
        }
        *s = State::Transcribing;
    }
    stop_flag.store(true, Ordering::Relaxed);
    if let Some(h) = rec_handle.lock().unwrap().take() {
        let _ = h.join();
    }
    *state.lock().unwrap() = State::Ready;
    true
}

fn toggle_recording(
    state: &Arc<Mutex<State>>,
    stop_flag: &Arc<AtomicBool>,
    discard_flag: &Arc<AtomicBool>,
    rec_handle: &Arc<Mutex<Option<thread::JoinHandle<()>>>>,
) -> &'static str {
    let cur = *state.lock().unwrap();
    if cur == State::Ready {
        if start_recording(state, stop_flag, discard_flag, rec_handle) {
            "listening"
        } else {
            "busy"
        }
    } else if cur == State::Listening {
        if stop_recording(state, stop_flag, rec_handle) {
            "stopped"
        } else {
            "busy"
        }
    } else {
        "busy"
    }
}

// global hotkey via evdev: Ctrl+Shift+D toggles recording.
// listens on every real keyboard. keyd (and similar remappers) grab the
// physical device and re-emit through a virtual keyboard, so reading all of
// them is the only way to see the key on every setup.
fn spawn_hotkey_listener(
    state: Arc<Mutex<State>>,
    stop_flag: Arc<AtomicBool>,
    discard_flag: Arc<AtomicBool>,
    rec_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
) {
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/dev/input") {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_event = path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with("event"))
                .unwrap_or(false);
            if !is_event {
                continue;
            }
            let Ok(dev) = evdev::Device::open(&path) else {
                continue;
            };
            let name = dev.name().unwrap_or("").to_lowercase();
            if name.contains("ydotool") || name.contains("uinput") || name.contains("typebird") {
                continue;
            }
            let Some(keys) = dev.supported_keys() else {
                continue;
            };
            let full_keyboard = keys.contains(evdev::Key::KEY_A)
                && keys.contains(evdev::Key::KEY_Z)
                && keys.contains(evdev::Key::KEY_SPACE)
                && keys.contains(evdev::Key::KEY_D);
            if full_keyboard {
                paths.push(path);
            }
        }
    }
    eprintln!("hotkey: Ctrl+Shift+D on {} keyboard(s)", paths.len());

    for path in paths {
        let state = state.clone();
        let stop_flag = stop_flag.clone();
        let discard_flag = discard_flag.clone();
        let rec_handle = rec_handle.clone();
        thread::spawn(move || {
            let Ok(mut dev) = evdev::Device::open(&path) else {
                return;
            };
            let mut ctrl = false;
            let mut shift = false;
            loop {
                if let Ok(events) = dev.fetch_events() {
                    for ev in events {
                        if ev.event_type() != evdev::EventType::KEY {
                            continue;
                        }
                        let code = ev.code();
                        let val = ev.value();
                        if code == evdev::Key::KEY_LEFTCTRL.0 || code == evdev::Key::KEY_RIGHTCTRL.0
                        {
                            ctrl = val != 0;
                        } else if code == evdev::Key::KEY_LEFTSHIFT.0
                            || code == evdev::Key::KEY_RIGHTSHIFT.0
                        {
                            shift = val != 0;
                        } else if code == evdev::Key::KEY_D.0 && val == 1 && ctrl && shift {
                            let r =
                                toggle_recording(&state, &stop_flag, &discard_flag, &rec_handle);
                            eprintln!("hotkey: {}", r);
                        }
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
    }
}

pub fn run() {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("bind failed: {}", e);
            return;
        }
    };
    eprintln!("typebird daemon listening on {}", path);

    // keep the pill on screen for the whole life of the daemon
    ensure_pill();

    let state = Arc::new(Mutex::new(State::Ready));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let discard_flag = Arc::new(AtomicBool::new(false));
    let rec_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>> = Arc::new(Mutex::new(None));

    spawn_hotkey_listener(
        state.clone(),
        stop_flag.clone(),
        discard_flag.clone(),
        rec_handle.clone(),
    );

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let state = state.clone();
                let stop_flag = stop_flag.clone();
                let rec_handle = rec_handle.clone();
                let discard_flag = discard_flag.clone();
                thread::spawn(move || handle_client(s, state, stop_flag, discard_flag, rec_handle));
            }
            Err(e) => eprintln!("accept error: {}", e),
        }
    }
}

fn handle_client(
    mut stream: UnixStream,
    state: Arc<Mutex<State>>,
    stop_flag: Arc<AtomicBool>,
    discard_flag: Arc<AtomicBool>,
    rec_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let cmd = line.trim().to_string();

    let resp = match cmd.as_str() {
        "state" => {
            let s = *state.lock().unwrap();
            s.as_str().to_string()
        }
        "start" => {
            if start_recording(&state, &stop_flag, &discard_flag, &rec_handle) {
                "ok".to_string()
            } else {
                "busy".to_string()
            }
        }
        "stop" => {
            if stop_recording(&state, &stop_flag, &rec_handle) {
                "ok".to_string()
            } else {
                "not listening".to_string()
            }
        }
        "cancel" => {
            discard_flag.store(true, Ordering::Relaxed);
            stop_flag.store(true, Ordering::Relaxed);
            if let Some(h) = rec_handle.lock().unwrap().take() {
                let _ = h.join();
            }
            *state.lock().unwrap() = State::Ready;
            "ok".to_string()
        }
        "toggle" => toggle_recording(&state, &stop_flag, &discard_flag, &rec_handle).to_string(),
        "info" => "typebird daemon".to_string(),
        _ => format!("unknown: {}", cmd),
    };

    let _ = writeln!(stream, "{}", resp);
}

// the recording loop. records in chunks, transcribes each, types as they come.
// cuts a chunk every ~5s. silence detection comes later.
fn recording_loop(stop: Arc<AtomicBool>, discard: Arc<AtomicBool>, state: Arc<Mutex<State>>) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            eprintln!("no input device");
            return;
        }
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
        Err(e) => {
            eprintln!("build stream failed: {}", e);
            return;
        }
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
                prev_ended = process_chunk(&chunk, prev_ended, false, &discard);
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
    if !remaining.is_empty() && !discard.load(Ordering::Relaxed) {
        let _ = process_chunk(&remaining, prev_ended, true, &discard);
    }

    drop(stream);
    let _ = state; // state flips to Ready in stop_recording
}

fn process_chunk(chunk: &[i16], prev_ended: bool, is_last: bool, discard: &AtomicBool) -> bool {
    if discard.load(Ordering::Relaxed) {
        return prev_ended;
    }
    if chunk.len() < 8000 {
        return prev_ended;
    } // skip tiny chunks
    // rms check: skip silent chunks, whisper hallucinates on silence
    let rms: f64 = (chunk.iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / chunk.len() as f64)
        .sqrt()
        / 32768.0;
    if rms < 0.005 {
        eprintln!("chunk: (silent, rms={:.4}, skipping)", rms);
        return prev_ended;
    }
    let wav_path = "/tmp/typebird-chunk.wav";
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    if let Ok(mut w) = WavWriter::create(wav_path, spec) {
        for &s in chunk {
            let _ = w.write_sample(s);
        }
        let _ = w.finalize();
    }

    let raw = transcribe::transcribe(wav_path);
    if raw.is_empty() || raw == "[BLANK_AUDIO]" {
        return prev_ended;
    }
    if discard.load(Ordering::Relaxed) {
        return prev_ended;
    }
    // seam-hiding
    let mut text = seam::clean(&raw, prev_ended);
    // drop trailing punctuation on non-last chunks (short pause between chunks)
    if !is_last {
        let t = text.trim_end_matches(['.', '?', '!']).trim().to_string();
        if !t.is_empty() {
            text = t;
        }
    }
    if discard.load(Ordering::Relaxed) {
        return prev_ended;
    }
    if !text.is_empty() {
        eprintln!("chunk: {}", text);
        inject::type_text(&text);
    }
    seam::ended_sentence(&text)
}
