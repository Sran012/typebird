// shell out to whisper-cli. returns transcribed text.
use std::process::Command;

pub fn transcribe(wav_path: &str) -> String {
    let model = "/home/sujal/.local/share/voxd/models/ggml-base.en.bin";
    let out = Command::new("whisper-cli")
        .args(["-m", model, "-f", wav_path, "-l", "en", "-t", "4", "-nt"])
        .output();

    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(e) => { eprintln!("whisper-cli failed: {}", e); String::new() }
    }
}
