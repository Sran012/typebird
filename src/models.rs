// model management. list, get.
use std::process::Command;

pub fn list() {
    let dir = "/home/sujal/.local/share/voxd/models";
    println!("model dir: {}", dir);
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            println!("  {}", e.file_name().to_string_lossy());
        }
    }
}

pub fn get(name: &str) {
    println!("downloading model: {}", name);
    // placeholder - real download comes with install.sh
    println!("(not implemented yet, grab it manually)");
}
