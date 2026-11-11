<p align="center">
  <img src="assets/typebird-banner.png" alt="typebird: voice typing for Linux" width="800">
</p>

<h1 align="center">typebird</h1>

<p align="center">
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://github.com/Sran012/typebird/stargazers"><img src="https://img.shields.io/github/stars/Sran012/typebird?style=social" alt="GitHub stars"></a>
  <a href="https://github.com/Sran012/typebird/network/members"><img src="https://img.shields.io/github/forks/Sran012/typebird?style=social" alt="GitHub forks"></a>
</p>

<p align="center">
  Voice typing for Linux. Press a key, talk, and the words appear as you speak.
</p>

<p align="center">
  Works on X11 and Wayland, GNOME and KDE. Two small binaries, no webview, no cloud.
</p>

## Status

Early. Daemon, streaming, uinput typing, hotkey, overlay pill, doctor, and installer all exist. Roadmap at the bottom.

## Quick start

```bash
curl -fsSL https://raw.githubusercontent.com/Sran012/typebird/main/install.sh | bash
typebird doctor
```

The installer builds the binaries, downloads the model, adds typebird to your desktop autostart, and starts it. A small pill appears at the bottom of the screen while you record. Press **Ctrl+Shift+D**, talk, press it again or click stop. Text types into whatever window has focus.

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
- Recording is streamed in chunks. Text appears while you speak, not all at once when you stop

## Install

See `install.sh`. It detects your distro, installs system deps, builds from source, downloads the model, and adds a desktop autostart entry.

Manual build:

```bash
cargo build --release
cp target/release/typebird ~/.local/bin/
cp target/release/typebird-pill ~/.local/bin/
typebird install        # autostart for the daemon and pill
```

## Usage

```
typebird install         # autostart the daemon and pill with the desktop session
typebird daemon          # run the daemon in the foreground
typebird toggle          # start/stop recording
typebird start           # explicit start
typebird stop            # stop, transcribe, type
typebird cancel          # abandon without transcribing
typebird state           # ready / listening / transcribing
typebird info            # engine, recorder, injector in use
typebird doctor          # what's present, what's missing
typebird models          # list models
typebird models get X    # download model X
typebird hotkey <key>    # optional extra binding for GNOME
```

The daemon listens for **Ctrl+Shift+D** itself through `/dev/input`, so the shortcut works on any desktop without compositor configuration. The pill is a small always-on-top window at the bottom of the screen; it shows the daemon state and has a working stop button. It never takes keyboard focus, so text keeps going to the window you were typing in.

## Text injection

By default typebird creates a uinput virtual keyboard. This works on every compositor because it's a kernel input device, not an X11 or Wayland protocol.

The cost: a keyboard emits keycodes, not characters, so characters are mapped through a US-QWERTY layout. On a different layout the keycodes are still valid but the characters differ. typebird reports characters it cannot type rather than typing something else. `wtype`/`ydotool`/`xdotool` remain available as fallbacks. Reading the active keymap is on the roadmap.

Security note: the hotkey needs read access to your keyboards, and typing needs write access to `/dev/uinput`. Together that is the ability to read every keystroke and synthesize arbitrary ones. That is inherent to a global dictation shortcut, but it is a real capability. The installer adds you to the `input` group and installs a udev rule for `/dev/uinput`. If that is not acceptable, bind a desktop shortcut to `typebird toggle` instead and do not install the hotkey.

## Streaming

Text appears while you speak, not all at once when you stop. Chunks are cut every few seconds. Pauses never stop recording. Tap stop when finished.

Seam-hiding makes chunk boundaries invisible:
- strips ellipses whisper writes at chunk edges
- lowercases mid-sentence restarts
- drops false full stops on short pauses

## Audio capture

Detected, not assumed: PipeWire, then PulseAudio, then ALSA. Recording is 16 kHz mono s16, which is what the engines want.

The recorder is stopped cleanly so the last chunk is still transcribed.

## Models

`$XDG_DATA_HOME/typebird/models`, overridable. Bare model names resolve there; absolute paths are used as given. Downloading a model once and having every tool use it is the point.

## Roadmap

- voice commands (punctuation, newline, corrections)
- model downloader that just works (`typebird models get base.en`)
- configurable hotkey
- streaming transcription improvements

---

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Sran012/typebird&type=Date)](https://star-history.com/#Sran012/typebird&Date)

## License

[MIT](LICENSE)