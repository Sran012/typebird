# typebird

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub stars](https://img.shields.io/github/stars/Sran012/typebird?style=social)](https://github.com/Sran012/typebird/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/Sran012/typebird?style=social)](https://github.com/Sran012/typebird/network/members)

Voice typing for Linux. One binary, one model, every app.

Press a hotkey, talk, text appears in whatever has focus. Works on X11 and Wayland, GNOME and KDE. Single static binary.

## Status

Early. Daemon, streaming, uinput typing, hotkey, overlay pill, doctor, and installer all exist. Roadmap at the bottom.

## Quick start

```bash
curl -fsSL https://raw.githubusercontent.com/Sran012/typebird/main/install.sh | bash
systemctl --user enable --now typebird
typebird doctor
```

Bind `typebird toggle` to a key (Super+H by default via the installer). Press it, talk, press again. Text types into the focused window.

## How it works

```mermaid
graph TD
    A[Keybinding / Script] --> B[typebird daemon]
    C[Overlay Pill] --> B
    B --> D[Audio Capture<br/>cpal]
    B --> E[Engine Trait<br/>whisper.cpp]
    B --> F[uinput Typing<br/>virtual keyboard]
    D --> E
    E --> F
    style B fill:#4a90d9,color:#fff
```

- Audio capture via cpal (PipeWire / PulseAudio / ALSA, autodetected)
- Transcription via whisper.cpp (`whisper-cli`), English model by default
- Typing via a uinput virtual keyboard. Works identically on X11 and Wayland because it's a kernel input device, not a compositor protocol
- Recording is streamed in chunks. Silence cuts chunks, never stops recording. Text appears while you speak

## Install

See `install.sh`. It detects your distro, installs system deps, builds from source, downloads the model, sets up the systemd user service, and binds a hotkey.

Manual build:

```bash
cargo build --release
cp target/release/typebird ~/.local/bin/
cp target/release/typebird-pill ~/.local/bin/  # optional overlay
```

## Usage

```
typebird daemon          # run the daemon (or use the systemd service)
typebird toggle          # start/stop recording
typebird start           # explicit start
typebird stop            # stop, transcribe, type
typebird cancel          # abandon without transcribing
typebird state           # ready / listening / transcribing
typebird info            # engine, recorder, injector in use
typebird doctor          # what's present, what's missing
typebird models          # list models
typebird models get X    # download model X
typebird hotkey <key>    # bind key (gnome)
```

`typebird-pill` runs the floating indicator. It subscribes to the daemon and shows listening/transcribing/idle state. Never takes focus, clicks pass through.

## Text injection

By default typebird creates a uinput virtual keyboard. This works on every compositor because it's a kernel input device, not an X11 or Wayland protocol.

The cost: a keyboard emits keycodes, not characters, so characters are mapped through a US-QWERTY layout. On a different layout the keycodes are still valid but the characters differ. typebird reports characters it cannot type rather than typing something else. `wtype`/`ydotool`/`xdotool` remain available as fallbacks. Reading the active keymap is on the roadmap.

Security note: write access to `/dev/uinput` is the ability to synthesize arbitrary keystrokes. That's inherent to dictation, but it's a real capability. The shipped udev rule scopes it to the active local session via `uaccess` rather than granting it to a group permanently. Skip it with `install.sh --no-udev` and typebird falls back to an external tool.

## Streaming

Text appears while you speak, not all at once when you stop. Chunks are cut at silence points. Pauses never stop recording. Tap stop when finished.

Seam-hiding makes chunk boundaries invisible:
- strips ellipses whisper writes at chunk edges
- lowercases mid-sentence restarts
- drops false full stops on short pauses

## Audio capture

Detected, not assumed: PipeWire, then PulseAudio, then ALSA. Recording is 16 kHz mono s16, which is what the engines want.

The recorder is stopped with SIGINT rather than SIGKILL so it finishes writing a valid WAV header.

## Models

`$XDG_DATA_HOME/typebird/models`, overridable. Bare model names resolve there; absolute paths are used as given. Downloading a model once and having every tool use it is the point.

## Roadmap

- voice commands (punctuation, newline, corrections)
- model downloader that just works (`typebird models get base.en`)
- KDE and sway hotkey support (gnome only right now)
- streaming transcription improvements

---

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Sran012/typebird&type=Date)](https://star-history.com/#Sran012/typebird&Date)

## License

[MIT](LICENSE)