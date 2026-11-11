#!/usr/bin/env bash
# typebird installer.
#
# run it from an extracted release tarball, or pipe it from curl:
#   curl -fsSL https://raw.githubusercontent.com/Sran012/typebird/main/install.sh | bash
#
# no rust and no whisper.cpp build required. the tarball ships both binaries
# and a static whisper-cli. this script downloads the models, grants input
# access, and sets up autostart.
set -euo pipefail

REPO="${TYPEBIRD_REPO:-Sran012/typebird}"
VERSION="${TYPEBIRD_VERSION:-latest}"
TARGET="x86_64-unknown-linux-gnu"

DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/typebird"
MODEL_DIR="$DATA_DIR/models"
BIN_DIR="$HOME/.local/bin"
CLI_DIR="$DATA_DIR/bin"
BASE_MODEL="$MODEL_DIR/ggml-base.en.bin"
VAD_MODEL="$MODEL_DIR/ggml-silero-v5.1.2.bin"
LEGACY_MODEL="${XDG_DATA_HOME:-$HOME/.local/share}/voxd/models/ggml-base.en.bin"

BASE_SHA="a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002"
VAD_SHA="29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf"
BASE_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"
VAD_URL="https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v5.1.2.bin"

say() { printf '  %s\n' "$*"; }
warn() { printf '  warning: %s\n' "$*" >&2; }
die() { printf 'typebird: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

fetch() {
    if have curl; then
        curl -fL --retry 3 --connect-timeout 15 -o "$2" "$1"
    elif have wget; then
        wget -q --show-progress -O "$2" "$1"
    else
        die "curl or wget is required"
    fi
}

[ "$(uname -s)" = "Linux" ] || die "typebird runs on linux"
[ "$(uname -m)" = "x86_64" ] || die "this build is x86_64 only, not $(uname -m)"

# find the payload: next to this script, or download the latest tarball.
SCRIPT_DIR=""
if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
    SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fi

TMP=""
cleanup() { if [ -n "$TMP" ]; then rm -rf "$TMP"; fi; }
trap cleanup EXIT

if [ -n "$SCRIPT_DIR" ] && [ -x "$SCRIPT_DIR/typebird" ] && [ -x "$SCRIPT_DIR/whisper-cli" ]; then
    STAGE="$SCRIPT_DIR"
    say "installing from $STAGE"
else
    TMP="$(mktemp -d)"
    if [ "$VERSION" = "latest" ]; then
        say "looking up the latest release"
        if have curl; then
            tag="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
                | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
        else
            tag="$(wget -qO- "https://api.github.com/repos/$REPO/releases/latest" \
                | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
        fi
        [ -n "$tag" ] || die "could not find a release, set TYPEBIRD_VERSION=v0.1.0"
    else
        tag="$VERSION"
    fi
    tarball="typebird-${tag}-${TARGET}.tar.gz"
    base="https://github.com/$REPO/releases/download/$tag"
    say "downloading $tarball"
    fetch "$base/$tarball" "$TMP/$tarball"
    fetch "$base/SHA256SUMS" "$TMP/SHA256SUMS"
    ( cd "$TMP" && grep "  $tarball\$" SHA256SUMS | sha256sum -c - >/dev/null ) \
        || die "checksum mismatch"
    tar -xzf "$TMP/$tarball" -C "$TMP"
    STAGE="$TMP/${tarball%.tar.gz}"
fi

[ -x "$STAGE/typebird" ] || die "typebird missing from the payload"
[ -x "$STAGE/typebird-pill" ] || die "typebird-pill missing from the payload"
[ -x "$STAGE/whisper-cli" ] || die "whisper-cli missing from the payload"

# the daemon links libasound at runtime. install it if it is missing.
if have ldconfig && ! ldconfig -p 2>/dev/null | grep -q "libasound.so.2"; then
    if have apt-get; then
        say "installing libasound2"
        sudo apt-get update -qq
        sudo apt-get install -y libasound2 2>/dev/null \
            || sudo apt-get install -y libasound2t64
    else
        warn "libasound.so.2 is missing, install your distro's alsa library"
    fi
fi

# stop an older copy so the new binaries take over.
pkill -x typebird-pill 2>/dev/null || true
pkill -x typebird 2>/dev/null || true
sleep 0.2

mkdir -p "$BIN_DIR" "$CLI_DIR" "$MODEL_DIR"
install -m 0755 "$STAGE/typebird" "$BIN_DIR/typebird"
install -m 0755 "$STAGE/typebird-pill" "$BIN_DIR/typebird-pill"
install -m 0755 "$STAGE/whisper-cli" "$CLI_DIR/whisper-cli"
say "binaries -> $BIN_DIR/typebird, $CLI_DIR/whisper-cli"

if [ -x "$STAGE/uninstall.sh" ]; then
    install -m 0755 "$STAGE/uninstall.sh" "$DATA_DIR/uninstall.sh"
fi

"$BIN_DIR/typebird" --version

# keep an existing voxd model instead of downloading 142 MB again.
if [ ! -f "$BASE_MODEL" ] && [ -f "$LEGACY_MODEL" ]; then
    ln -sf "$LEGACY_MODEL" "$BASE_MODEL"
    say "linked the existing model from $LEGACY_MODEL"
fi

download_model() {
    url="$1"; dest="$2"; want="$3"; label="$4"
    if [ -f "$dest" ]; then
        got="$(sha256sum "$dest" | cut -d' ' -f1)"
        if [ "$got" = "$want" ]; then
            say "$label already present"
            return
        fi
        say "$label is incomplete, downloading again"
    fi
    say "downloading $label"
    part="$dest.part"
    fetch "$url" "$part"
    got="$(sha256sum "$part" | cut -d' ' -f1)"
    if [ "$got" != "$want" ]; then
        rm -f "$part"
        die "$label checksum mismatch"
    fi
    mv "$part" "$dest"
}

download_model "$BASE_URL" "$BASE_MODEL" "$BASE_SHA" "base.en model"
download_model "$VAD_URL" "$VAD_MODEL" "$VAD_SHA" "silero VAD model"

# input access. the hotkey reads /dev/input/event*, typing writes /dev/uinput.
USER_NAME="$(id -un)"
NEED_RELOGIN=0
if ! id -nG "$USER_NAME" | grep -qw input; then
    say "adding $USER_NAME to the input group"
    if have sudo; then
        sudo usermod -aG input "$USER_NAME"
        NEED_RELOGIN=1
    else
        warn "run: sudo usermod -aG input $USER_NAME"
        NEED_RELOGIN=1
    fi
fi

if ! grep -q 'KERNEL=="uinput"' /etc/udev/rules.d/99-typebird.rules 2>/dev/null; then
    if have sudo && have udevadm; then
        echo 'KERNEL=="uinput", MODE="0660", GROUP="input", OPTIONS+="static_node=uinput"' \
            | sudo tee /etc/udev/rules.d/99-typebird.rules >/dev/null
        sudo udevadm control --reload-rules
        sudo udevadm trigger
        sudo modprobe uinput 2>/dev/null || true
        say "udev rule -> /etc/udev/rules.d/99-typebird.rules"
    else
        warn "could not install the uinput udev rule, see README"
    fi
fi

"$BIN_DIR/typebird" install

if [ "$NEED_RELOGIN" -eq 1 ]; then
    echo
    say "log out and back in once so the input group applies"
    say "after that typebird starts with your desktop, or run: typebird daemon"
else
    if ! pgrep -x typebird >/dev/null 2>&1; then
        setsid "$BIN_DIR/typebird" daemon >/dev/null 2>&1 < /dev/null &
        say "daemon started"
    fi
fi

echo
echo "typebird installed."
echo "  hotkey: Ctrl+Shift+D"
echo "  typebird doctor   # check audio, model, uinput, daemon"
