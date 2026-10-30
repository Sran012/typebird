// uinput typing. virtual keyboard, us qwerty.
use evdev::uinput::VirtualDeviceBuilder;
use evdev::{AttributeSet, EventType, Key};
use std::thread;
use std::time::Duration;

fn char_to_key(c: char) -> Option<Key> {
    match c {
        'a' => Some(Key::KEY_A), 'b' => Some(Key::KEY_B), 'c' => Some(Key::KEY_C),
        'd' => Some(Key::KEY_D), 'e' => Some(Key::KEY_E), 'f' => Some(Key::KEY_F),
        'g' => Some(Key::KEY_G), 'h' => Some(Key::KEY_H), 'i' => Some(Key::KEY_I),
        'j' => Some(Key::KEY_J), 'k' => Some(Key::KEY_K), 'l' => Some(Key::KEY_L),
        'm' => Some(Key::KEY_M), 'n' => Some(Key::KEY_N), 'o' => Some(Key::KEY_O),
        'p' => Some(Key::KEY_P), 'q' => Some(Key::KEY_Q), 'r' => Some(Key::KEY_R),
        's' => Some(Key::KEY_S), 't' => Some(Key::KEY_T), 'u' => Some(Key::KEY_U),
        'v' => Some(Key::KEY_V), 'w' => Some(Key::KEY_W), 'x' => Some(Key::KEY_X),
        'y' => Some(Key::KEY_Y), 'z' => Some(Key::KEY_Z),
        ' ' => Some(Key::KEY_SPACE),
        _ => None,
    }
}

pub fn type_text(text: &str) {
    if text.is_empty() || text == "[BLANK_AUDIO]" {
        return;
    }
    let mut keys = AttributeSet::<Key>::new();
    for k in 0..256u16 {
        keys.insert(Key(k));
    }

    let builder = match VirtualDeviceBuilder::new() {
        Ok(b) => b,
        Err(e) => { eprintln!("uinput open failed: {}", e); return; }
    };
    let builder = builder.name("typebird-kbd");
    let builder = match builder.with_keys(&keys) {
        Ok(b) => b,
        Err(e) => { eprintln!("with_keys failed: {}", e); return; }
    };
    let mut dev = match builder.build() {
        Ok(d) => d,
        Err(e) => { eprintln!("build failed: {}", e); return; }
    };

    for c in text.chars() {
        if let Some(key) = char_to_key(c.to_ascii_lowercase()) {
            let shift = c.is_uppercase();
            if shift {
                let _ = dev.emit(&[evdev::InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 1)]);
            }
            let _ = dev.emit(&[evdev::InputEvent::new(EventType::KEY, key.0, 1)]);
            let _ = dev.emit(&[evdev::InputEvent::new(EventType::KEY, key.0, 0)]);
            if shift {
                let _ = dev.emit(&[evdev::InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 0)]);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    let _ = dev.emit(&[evdev::InputEvent::new(EventType::SYNCHRONIZATION, 0, 0)]);
}
