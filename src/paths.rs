// where models live. typebird's own dir first, then the old voxd dir so
// existing setups keep working without re-downloading.
use std::path::PathBuf;

pub fn data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp"));
            home.join(".local/share")
        })
}

pub fn model_dir() -> PathBuf {
    data_home().join("typebird/models")
}

pub fn model_path() -> PathBuf {
    let name = "ggml-base.en.bin";
    let primary = model_dir().join(name);
    if primary.exists() {
        return primary;
    }
    let legacy = data_home().join("voxd/models").join(name);
    if legacy.exists() {
        return legacy;
    }
    primary
}

// silero VAD model. optional, but without it whisper hallucinates on noise.
pub fn vad_model_path() -> Option<PathBuf> {
    let p = model_dir().join("ggml-silero-v5.1.2.bin");
    if p.exists() { Some(p) } else { None }
}

// the bundled whisper-cli lives next to typebird in the release tarball, or
// in the typebird data dir once installed. fall back to PATH for source builds.
pub fn whisper_cli() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let candidate = dir.join("whisper-cli");
        if candidate.exists() {
            return candidate;
        }
    }
    let bundled = data_home().join("typebird/bin/whisper-cli");
    if bundled.exists() {
        return bundled;
    }
    PathBuf::from("whisper-cli")
}
