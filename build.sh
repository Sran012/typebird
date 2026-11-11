#!/usr/bin/env bash
# build a release tarball locally. mirrors .github/workflows/release.yml.
set -euo pipefail
cd "$(dirname "$0")"

CARGO_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
VERSION="${VERSION:-$CARGO_VERSION}"
WHISPER_TAG="${WHISPER_TAG:-v1.9.5}"
TARGET="x86_64-unknown-linux-gnu"
NAME="typebird-v${VERSION}-${TARGET}"
WHISPER_DIR="build/whisper.cpp"
DIST="dist"

if [ "$VERSION" != "$CARGO_VERSION" ]; then
    echo "VERSION=$VERSION does not match Cargo.toml $CARGO_VERSION" >&2
    exit 1
fi

for tool in cargo cmake git; do
    command -v "$tool" >/dev/null 2>&1 || { echo "missing $tool" >&2; exit 1; }
done

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release

if [ -d "$WHISPER_DIR/.git" ]; then
    current="$(git -C "$WHISPER_DIR" describe --tags --always 2>/dev/null || true)"
    if [ "$current" != "$WHISPER_TAG" ]; then
        echo "re-cloning whisper.cpp $WHISPER_TAG (had $current)"
        rm -rf "$WHISPER_DIR"
    fi
fi
if [ ! -d "$WHISPER_DIR" ]; then
    git clone --depth 1 --branch "$WHISPER_TAG" \
        https://github.com/ggerganov/whisper.cpp.git "$WHISPER_DIR"
fi

cmake -S "$WHISPER_DIR" -B "$WHISPER_DIR/build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHARED_LIBS=OFF \
    -DGGML_NATIVE=OFF \
    -DGGML_OPENMP=OFF \
    -DWHISPER_BUILD_TESTS=OFF \
    -DWHISPER_BUILD_EXAMPLES=ON
cmake --build "$WHISPER_DIR/build" --config Release --target whisper-cli -j "$(nproc)"
"$WHISPER_DIR/build/bin/whisper-cli" --help 2>&1 | grep -q -- "--vad"

rm -rf "$DIST/$NAME" "$DIST/$NAME.tar.gz"
mkdir -p "$DIST/$NAME/THIRD_PARTY_LICENSES"
cp target/release/typebird target/release/typebird-pill "$DIST/$NAME/"
cp "$WHISPER_DIR/build/bin/whisper-cli" "$DIST/$NAME/"
cp install.sh uninstall.sh README.md LICENSE "$DIST/$NAME/"
cp "$WHISPER_DIR/LICENSE" "$DIST/$NAME/THIRD_PARTY_LICENSES/whisper.cpp-LICENSE"
tar -C "$DIST" -czf "$DIST/$NAME.tar.gz" "$NAME"
( cd "$DIST" && sha256sum "$NAME.tar.gz" > SHA256SUMS )

echo
echo "built $DIST/$NAME.tar.gz"
cat "$DIST/SHA256SUMS"
