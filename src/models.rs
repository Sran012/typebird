// model management. list, get.

pub fn list() {
    let dir = crate::paths::model_dir();
    println!("model dir: {}", dir.display());
    match std::fs::read_dir(&dir) {
        Ok(entries) => {
            let mut any = false;
            for e in entries.flatten() {
                println!("  {}", e.file_name().to_string_lossy());
                any = true;
            }
            if !any {
                println!("  (empty, run `typebird models get base.en`)");
            }
        }
        Err(_) => println!("  (empty, run `typebird models get base.en`)"),
    }
}

pub fn get(name: &str) {
    println!("model downloads are handled by the installer for now");
    println!("place {} in {}", name, crate::paths::model_dir().display());
}
