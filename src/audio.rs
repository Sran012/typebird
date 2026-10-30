// audio capture. cpal -> 16khz mono s16 samples.
// record_until_stop blocks until the stop flag is set, then returns samples.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

pub fn record_until_stop(stop: Arc<AtomicBool>) -> Vec<i16> {
    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => { eprintln!("no input device"); return vec![]; }
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
        Err(e) => { eprintln!("build stream failed: {}", e); return vec![]; }
    };

    if stream.play().is_err() {
        eprintln!("failed to play stream");
        return vec![];
    }

    // wait until stop flag
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(stream);

    let buf = samples.lock().unwrap();
    buf.clone()
}
