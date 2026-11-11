// optional compositor binding for desktops that prefer it over the built-in
// evdev hotkey. the daemon already listens for Ctrl+Shift+D on its own.
pub fn setup_gnome(key: &str) {
    let path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/typebird/";
    let _ = std::process::Command::new("gsettings")
        .args([
            "set",
            &format!(
                "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}",
                path
            ),
            "name",
            "Typebird",
        ])
        .output();
    let _ = std::process::Command::new("gsettings")
        .args([
            "set",
            &format!(
                "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}",
                path
            ),
            "command",
            "typebird toggle",
        ])
        .output();
    let _ = std::process::Command::new("gsettings")
        .args([
            "set",
            &format!(
                "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}",
                path
            ),
            "binding",
            key,
        ])
        .output();
    println!("bound {} to 'typebird toggle' (gnome)", key);
}
