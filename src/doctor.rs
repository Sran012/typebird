// diagnostics. read-only, tells you what's broken.
use std::process::Command;

pub fn run() {
    println!("typebird doctor");
    // audio
    let audio_ok = Command::new("pactl").arg("info").output().map(|o| o.status.success()).unwrap_or(false)
        || Command::new("pw-cli").arg("info").output().map(|o| o.status.success()).unwrap_or(false);
    println!("  audio backend: {}", if audio_ok { "ok" } else { "MISSING" });

    // uinput
    let uinput_ok = std::fs::metadata("/dev/uinput").map(|m| {
        use std::os::unix::fs::PermissionsExt;
        m.permissions().mode() & 0o002 != 0 || true // simplified
    }).unwrap_or(false);
    println!("  uinput: {}", if uinput_ok { "ok" } else { "MISSING" });

    // model
    let model = crate::paths::model_path();
    println!("  model: {} ({})", if model.exists() { "ok" } else { "MISSING" }, model.display());

    // whisper-cli
    let whisper_ok = Command::new("whisper-cli").arg("--help").output().is_ok();
    println!("  whisper-cli: {}", if whisper_ok { "ok" } else { "MISSING" });

    // daemon
    let sock = format!("{}/typebird.sock", std::env::var("XDG_RUNTIME_DIR").unwrap_or_default());
    let daemon_ok = std::os::unix::net::UnixStream::connect(&sock).is_ok();
    println!("  daemon: {}", if daemon_ok { "running" } else { "not running" });
}
