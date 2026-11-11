#!/usr/bin/env bash
# remove typebird. keeps your models unless --purge is passed.
set -euo pipefail

PURGE=0
for arg in "$@"; do
    case "$arg" in
        --purge) PURGE=1 ;;
        -h|--help)
            cat <<'EOF'
usage: uninstall.sh [--purge]

removes the typebird binaries, autostart entry, and uinput udev rule.
keeps ~/.local/share/typebird/models unless --purge is passed.
EOF
            exit 0
            ;;
        *) echo "unknown option: $arg" >&2; exit 1 ;;
    esac
done

DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/typebird"
BIN_DIR="$HOME/.local/bin"
SOCKET="${XDG_RUNTIME_DIR:-/tmp}/typebird.sock"

pkill -x typebird-pill 2>/dev/null || true
pkill -x typebird 2>/dev/null || true

rm -f "$BIN_DIR/typebird" "$BIN_DIR/typebird-pill"
rm -f "$HOME/.config/autostart/typebird.desktop"
rm -f "$SOCKET"
rm -rf "$DATA_DIR/bin"
rm -f "$DATA_DIR/uninstall.sh"

if [ "$PURGE" -eq 1 ]; then
    rm -rf "$DATA_DIR"
    echo "removed typebird and its models"
else
    echo "removed typebird"
    echo "models kept in $DATA_DIR/models (run with --purge to remove them)"
fi

if [ -f /etc/udev/rules.d/99-typebird.rules ]; then
    if command -v sudo >/dev/null 2>&1; then
        sudo rm -f /etc/udev/rules.d/99-typebird.rules
        sudo udevadm control --reload-rules 2>/dev/null || true
        echo "removed the uinput udev rule"
    else
        echo "remove /etc/udev/rules.d/99-typebird.rules manually"
    fi
fi

echo "you can remove yourself from the input group if you no longer need it:"
echo "  sudo gpasswd -d $(id -un) input"
