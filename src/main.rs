// uinput typing test. create a virtual keyboard, type "hello", done.
// us qwerty keycode map. real layout handling comes later.
use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
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

fn type_text(dev: &mut VirtualDevice, text: &str) {
    for c in text.chars() {
        if let Some(key) = char_to_key(c.to_ascii_lowercase()) {
            let shift = c.is_uppercase();
            if shift {
                dev.emit(&[evdev::InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 1)]).unwrap();
            }
            dev.emit(&[evdev::InputEvent::new(EventType::KEY, key.0, 1)]).unwrap();
            dev.emit(&[evdev::InputEvent::new(EventType::KEY, key.0, 0)]).unwrap();
            if shift {
                dev.emit(&[evdev::InputEvent::new(EventType::KEY, Key::KEY_LEFTSHIFT.0, 0)]).unwrap();
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    // sync
    dev.emit(&[evdev::InputEvent::new(EventType::SYNCHRONIZATION, 0, 0)]).unwrap();
}

fn main() {
    let mut keys = AttributeSet::<Key>::new();
    for k in 0..256u16 {
        keys.insert(Key(k));
    }

    let mut dev = VirtualDeviceBuilder::new()
        .unwrap()
        .name("typebird-kbd")
        .with_keys(&keys)
        .unwrap()
        .build()
        .unwrap();

    println!("typing 'hello world' via uinput in 2s...");
    thread::sleep(Duration::from_secs(2));
    type_text(&mut dev, "hello world");
    println!("done");
}
