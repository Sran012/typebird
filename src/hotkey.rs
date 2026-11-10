// global hotkey via evdev. listens for a key, toggles recording.
// works on any desktop (gnome, kde, sway) because it reads /dev/input directly.
// requires user in 'input' group.
use evdev::{Device, EventType, Key};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

pub fn listen_toggle<F>(stop: Arc<AtomicBool>, callback: F)
where F: Fn() + Send + 'static {
    thread::spawn(move || {
        // find keyboard devices
        let mut devices: Vec<Device> = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/dev/input") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.to_string_lossy().contains("event") {
                    if let Ok(dev) = Device::open(&path) {
                        if dev.supported_keys().map(|k| k.contains(Key::KEY_F9)).unwrap_or(false) {
                            devices.push(dev);
                        }
                    }
                }
            }
        }

        // listen for F9 press
        for mut dev in devices {
            let stop = stop.clone();
            let cb = &callback;
            // can't easily share cb across threads, so just handle one device
            // for simplicity, handle first keyboard found
            loop {
                if stop.load(Ordering::Relaxed) { break; }
                if let Ok(events) = dev.fetch_events() {
                    for ev in events {
                        if ev.event_type() == EventType::KEY && ev.code() == Key::KEY_F9.0 && ev.value() == 1 {
                            cb();
                        }
                    }
                }
                thread::sleep(std::time::Duration::from_millis(10));
            }
            break;
        }
    });
}

pub fn setup_gnome(key: &str) {
    let _ = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/typebird/", "name", "Typebird"])
        .output();
    let _ = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/typebird/", "command", "typebird toggle"])
        .output();
    let _ = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/typebird/", "binding", key])
        .output();
    println!("bound {} to 'typebird toggle' (gnome)", key);
}