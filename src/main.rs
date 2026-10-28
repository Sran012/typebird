// audio capture. cpal gives us a stream, we grab samples into a buffer.
// 16khz mono s16 is what whisper wants, so we ask for that.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

fn main() {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("no input device");
    println!("using mic: {}", device.name().unwrap_or_default());

    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };

    let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_clone = samples.clone();

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _| {
                let mut buf = samples_clone.lock().unwrap();
                for &s in data {
                    buf.push((s * 32767.0) as i16);
                }
            },
            |err| eprintln!("stream error: {}", err),
            None,
        )
        .expect("failed to build stream");

    stream.play().expect("failed to play");
    println!("recording 3s...");
    std::thread::sleep(std::time::Duration::from_secs(3));
    drop(stream);

    let buf = samples.lock().unwrap();
    println!("captured {} samples ({}s)", buf.len(), buf.len() as f64 / 16000.0);
}
