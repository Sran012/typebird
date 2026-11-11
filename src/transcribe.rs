// shell out to whisper-cli. returns transcribed text.
//
// VAD (when the silero model is present) drops chunks that contain no speech.
// without it whisper turns room noise into "Hello, hello, how are you".
use std::process::Command;

pub fn transcribe(wav_path: &str) -> String {
    let model = crate::paths::model_path().to_string_lossy().to_string();
    let mut cmd = Command::new("whisper-cli");
    cmd.arg("-m")
        .arg(&model)
        .arg("-f")
        .arg(wav_path)
        .args(["-l", "en", "-t", "4", "-nt", "-sns", "-mc", "0"]);

    if let Some(vad) = crate::paths::vad_model_path() {
        cmd.arg("--vad")
            .arg("-vm")
            .arg(vad)
            .args(["-vt", "0.6", "-vspd", "300"]);
    }

    match cmd.output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(e) => {
            eprintln!("whisper-cli failed: {}", e);
            String::new()
        }
    }
}
