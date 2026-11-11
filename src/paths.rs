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
