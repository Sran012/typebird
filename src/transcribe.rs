// shell out to whisper-cli. returns transcribed text.
use std::process::Command;

pub fn transcribe(wav_path: &str) -> String {
    let model = crate::paths::model_path().to_string_lossy().to_string();
    let out = Command::new("whisper-cli")
        .arg("-m")
        .arg(&model)
        .arg("-f")
        .arg(wav_path)
        .args(["-l", "en", "-t", "4", "-nt"])
        .output();

    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(e) => { eprintln!("whisper-cli failed: {}", e); String::new() }
    }
}
