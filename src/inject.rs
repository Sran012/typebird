// uinput typing. one virtual keyboard, kept alive for the daemon lifetime.
// creating a device per chunk raced the compositor: the device could be gone
// before libinput had registered it, so the keys were dropped on the floor.
use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AttributeSet, EventType, InputEvent, Key};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

static DEVICE: Mutex<Option<VirtualDevice>> = Mutex::new(None);

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
        ',' => Some(Key::KEY_COMMA),
        '.' => Some(Key::KEY_DOT),
        '\'' => Some(Key::KEY_APOSTROPHE),
        '-' => Some(Key::KEY_MINUS),
        '?' => Some(Key::KEY_SLASH),
        '!' => Some(Key::KEY_1),
        _ => None,
    }
}

fn build() -> std::io::Result<VirtualDevice> {
    let mut keys = AttributeSet::<Key>::new();
    for k in 0..256u16 {
        keys.insert(Key(k));
    }
    VirtualDeviceBuilder::new()?
        .name("typebird-kbd")
        .with_keys(&keys)?
        .build()
}

pub fn type_text(text: &str) {
    if text.is_empty() || text == "[BLANK_AUDIO]" {
        return;
    }
    let mut guard = match DEVICE.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if guard.is_none() {
        match build() {
            Ok(d) => {
                // give libinput/compositor a moment to register the device
                // before the first key. only paid once per daemon lifetime.
                thread::sleep(Duration::from_millis(250));
                *guard = Some(d);
            }
            Err(e) => {
                eprintln!("uinput: {}", e);
                return;
            }
        }
    }
    let dev = guard.as_mut().unwrap();

    for c in text.chars() {
        let lower = c.to_ascii_lowercase();
        let Some(key) = char_to_key(lower) else { continue };
        let shift = c.is_uppercase() || matches!(c, '?' | '!');
        let mut ev = Vec::with_capacity(5);
        if shift {
            ev.push(InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 1));
        }
        ev.push(InputEvent::new(EventType::KEY, key.0, 1));
        ev.push(InputEvent::new(EventType::KEY, key.0, 0));
        if shift {
            ev.push(InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 0));
        }
        ev.push(InputEvent::new(EventType::SYNCHRONIZATION, 0, 0));
        let _ = dev.emit(&ev);
        thread::sleep(Duration::from_millis(8));
    }
}
