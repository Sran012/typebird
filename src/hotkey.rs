// hotkey binding. gnome only for now via gsettings.
// we don't grab keys ourselves - we let the desktop bind "typebird toggle"
// to a key. simpler and works on wayland.
use std::process::Command;

pub fn bind_gnome(key: &str) {
    // check if already bound
    let out = Command::new("gsettings")
        .args(["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
        .output();
    // just set it, gnome handles conflicts by ignoring
    let path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/typebird/";
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/typebird/", "name", "Typebird"])
        .output();
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/typebird/", "command", "typebird toggle"])
        .output();
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/typebird/", "binding", key])
        .output();
    println!("bound {} to 'typebird toggle' (gnome)", key);
}

pub fn setup(key: &str) {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if desktop.to_lowercase().contains("gnome") || desktop.to_lowercase().contains("zorin") {
        bind_gnome(key);
    } else {
        println!("hotkey: desktop '{}' not supported yet. bind 'typebird toggle' manually.", desktop);
    }
}
