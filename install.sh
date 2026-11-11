#!/bin/bash
# typebird installer. one command, whole pipeline.
set -e

echo "typebird: installing"

# 1. detect
DISTRO=$(grep -oP '^ID=\K.*' /etc/os-release 2>/dev/null | tr -d '"')
SESSION="x11"
[ -n "$WAYLAND_DISPLAY" ] && SESSION="wayland"
DESKTOP="${XDG_CURRENT_DESKTOP:-unknown}"
echo "  distro=$DISTRO desktop=$DESKTOP session=$SESSION"

# 2. system deps
if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get install -y pkg-config libasound2-dev 2>/dev/null || true
fi

# 3. build from source (needs cargo)
if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo not found. install rust first: https://rustup.rs"
    exit 1
fi
cargo build --release

# 4. install binary
mkdir -p ~/.local/bin
cp target/release/typebird ~/.local/bin/typebird
cp target/release/typebird-pill ~/.local/bin/typebird-pill 2>/dev/null || true
echo "  binary -> ~/.local/bin/typebird"

# 5. model dir
MODEL_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/typebird/models"
mkdir -p "$MODEL_DIR"
if [ ! -f "$MODEL_DIR/ggml-base.en.bin" ]; then
    echo "  downloading model..."
    curl -L -o "$MODEL_DIR/ggml-base.en.bin" \
        "https://github.com/ggerganov/whisper.cpp/releases/latest/download/ggml-base.en.bin" 2>/dev/null || \
        echo "  model download failed, grab it manually"
fi

# 6. udev rule for uinput
if [ ! -f /etc/udev/rules.d/99-typebird.rules ]; then
    echo 'KERNEL=="uinput", MODE="0660", GROUP="input", OPTIONS+="static_node=uinput"' | \
        sudo tee /etc/udev/rules.d/99-typebird.rules >/dev/null 2>&1 || true
    sudo udevadm control --reload-rules 2>/dev/null || true
fi

# 7. autostart: daemon and pill come up with the desktop session
~/.local/bin/typebird install

# 8. start it now so the shortcut works immediately
if ! pgrep -x typebird >/dev/null 2>&1; then
    setsid ~/.local/bin/typebird daemon >/dev/null 2>&1 < /dev/null &
fi

echo ""
echo "typebird installed."
echo "  hotkey: Ctrl+Shift+D toggles recording"
echo "  the pill sits at the bottom of the screen with start/stop"
echo "  typebird doctor   # check audio, model, uinput"
