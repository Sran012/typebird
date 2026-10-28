// audio capture + transcription. record 3s, write wav, shell out to whisper-cli.
// next step: type the result via uinput.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::process::Command;
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
    println!("captured {} samples", buf.len());

    let wav_path = "/tmp/typebird-test.wav";
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = WavWriter::create(wav_path, spec).expect("wav create failed");
    for &s in buf.iter() {
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();
    println!("wrote {}", wav_path);

    // transcribe
    let model = "/home/sujal/.local/share/voxd/models/ggml-base.en.bin";
    let out = Command::new("whisper-cli")
        .args(["-m", model, "-f", wav_path, "-l", "en", "-t", "4", "-nt"])
        .output()
        .expect("failed to run whisper-cli");

    let text = String::from_utf8_lossy(&out.stdout);
    println!("--- transcription ---");
    println!("{}", text.trim());
    println!("--- end ---");
}
