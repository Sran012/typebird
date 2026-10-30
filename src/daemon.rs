// the daemon. long-lived process, holds the socket, manages state.
// commands: toggle, start, stop, cancel, state, info, key <name>
// protocol: newline-delimited text over unix socket. response is a line.
use crate::audio;
use crate::inject;
use crate::transcribe;
use hound::WavWriter;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

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
    println!("typebird daemon listening on {}", path);

    let state = Arc::new(Mutex::new(State::Ready));
    let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let rec_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>> = Arc::new(Mutex::new(None));

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let state = state.clone();
                let samples = samples.clone();
                let stop_flag = stop_flag.clone();
                let rec_handle = rec_handle.clone();
                thread::spawn(move || handle_client(s, state, samples, stop_flag, rec_handle));
            }
            Err(e) => eprintln!("accept error: {}", e),
        }
    }
}

fn handle_client(
    mut stream: UnixStream,
    state: Arc<Mutex<State>>,
    samples: Arc<Mutex<Vec<i16>>>,
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
                samples.lock().unwrap().clear();
                stop_flag.store(false, Ordering::Relaxed);
                let stop = stop_flag.clone();
                let samples = samples.clone();
                let h = thread::spawn(move || {
                    let buf = audio::record_until_stop(stop);
                    *samples.lock().unwrap() = buf;
                });
                *rec_handle.lock().unwrap() = Some(h);
                *s = State::Listening;
                "ok".to_string()
            } else {
                "busy".to_string()
            }
        }
        "stop" => {
            let mut s = state.lock().unwrap();
            if *s == State::Listening {
                stop_flag.store(true, Ordering::Relaxed);
                // join recording thread
                if let Some(h) = rec_handle.lock().unwrap().take() {
                    let _ = h.join();
                }
                *s = State::Transcribing;
                drop(s); // release lock before transcribing

                // write wav
                let buf = samples.lock().unwrap().clone();
                let wav_path = "/tmp/typebird-test.wav";
                let spec = hound::WavSpec {
                    channels: 1, sample_rate: 16000, bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };
                if let Ok(mut w) = WavWriter::create(wav_path, spec) {
                    for &s in &buf { let _ = w.write_sample(s); }
                    let _ = w.finalize();
                }

                // transcribe
                let text = transcribe::transcribe(wav_path);
                println!("transcribed: {}", text);

                // type
                if !text.is_empty() && text != "[BLANK_AUDIO]" {
                    inject::type_text(&text);
                }

                *state.lock().unwrap() = State::Ready;
                "ok".to_string()
            } else {
                "not listening".to_string()
            }
        }
        "cancel" => {
            stop_flag.store(true, Ordering::Relaxed);
            if let Some(h) = rec_handle.lock().unwrap().take() { let _ = h.join(); }
            samples.lock().unwrap().clear();
            *state.lock().unwrap() = State::Ready;
            "ok".to_string()
        }
        "toggle" => {
            let cur = *state.lock().unwrap();
            if cur == State::Ready {
                // start
                samples.lock().unwrap().clear();
                stop_flag.store(false, Ordering::Relaxed);
                let stop = stop_flag.clone();
                let samples = samples.clone();
                let h = thread::spawn(move || {
                    let buf = audio::record_until_stop(stop);
                    *samples.lock().unwrap() = buf;
                });
                *rec_handle.lock().unwrap() = Some(h);
                *state.lock().unwrap() = State::Listening;
                "listening".to_string()
            } else if cur == State::Listening {
                // stop + transcribe + type (same as stop)
                "use stop".to_string()
            } else {
                "busy".to_string()
            }
        }
        "info" => "typebird daemon".to_string(),
        _ => format!("unknown: {}", cmd),
    };

    let _ = writeln!(stream, "{}", resp);
}
