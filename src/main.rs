// full pipeline: record 3s -> wav -> whisper-cli -> type the result.
// milestone 1: record, transcribe, type. all in one binary, no daemon yet.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AttributeSet, EventType, Key};
use hound::{WavSpec, WavWriter};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn char_to_key(c: char) -> Option<Key> {
    match c {
        'a' => Some(Key::KEY_A), 'b' => Some(Key::KEY_B), 'c' => Some(Key::KEY_C),
        'd' => Some(Key::KEY_D), 'e' => Some(Key::KEY_E), 'f' => Some(Key::KEY_F),
        'g' => Some(Key::KEY_G), 'h' => Some(Key::KEY_H), 'i' => Some(Key::KEY_I),
        'j' => Some(Key::KEY_J), 'k' => Some(Key::KEY_K), 'l' => Some(Key::KEY_L),
        'm' => Some(Key::KEY_M), 'n' => Some(Key::KEY_N), 'o' => Some(Key::KEY_O),
        'p' => Some(Key::KEY_P), 'q' => Some(Key::KEY_Q), 'r' => Some(Key::KEY_R),
        's' => Some(Key::KEY_S), 't' => Some(Key::KEY_T), 'u' => Some(Key::KEY_U),
        'v' => Some(Key::KEY_V), 'w' => Some(Key::KEY_W), 'x' => Some(Key::KEY_X),
        'y' => Some(Key::KEY_Y), 'z' => Some(Key::KEY_Z),
        ' ' => Some(Key::KEY_SPACE),
        _ => None,
    }
}

fn type_text(dev: &mut VirtualDevice, text: &str) {
    for c in text.chars() {
        if let Some(key) = char_to_key(c.to_ascii_lowercase()) {
            let shift = c.is_uppercase();
            if shift {
                dev.emit(&[evdev::InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 1)]).unwrap();
            }
            dev.emit(&[evdev::InputEvent::new(EventType::KEY, key.0, 1)]).unwrap();
            dev.emit(&[evdev::InputEvent::new(EventType::KEY, key.0, 0)]).unwrap();
            if shift {
                dev.emit(&[evdev::InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 0)]).unwrap();
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    dev.emit(&[evdev::InputEvent::new(EventType::SYNCHRONIZATION, 0, 0)]).unwrap();
}

fn record_and_transcribe() -> String {
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
    println!("recording 3s... speak now");
    thread::sleep(Duration::from_secs(3));
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

    let model = "/home/sujal/.local/share/voxd/models/ggml-base.en.bin";
    let out = Command::new("whisper-cli")
        .args(["-m", model, "-f", wav_path, "-l", "en", "-t", "4", "-nt"])
        .output()
        .expect("failed to run whisper-cli");

    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn main() {
    let text = record_and_transcribe();
    println!("transcribed: {}", text);

    if text.is_empty() || text == "[BLANK_AUDIO]" {
        println!("nothing to type");
        return;
    }

    let mut keys = AttributeSet::<Key>::new();
    for k in 0..256u16 {
        keys.insert(Key(k));
    }
    let mut dev = VirtualDeviceBuilder::new()
        .unwrap()
        .name("typebird-kbd")
        .with_keys(&keys)
        .unwrap()
        .build()
        .unwrap();

    println!("typing in 2s...");
    thread::sleep(Duration::from_secs(2));
    type_text(&mut dev, &text);
    println!("done");
}
