# Contributing

Thanks for helping. typebird is a small project maintained by one person, so small, focused changes are easiest to review.

## Before you start

- For anything larger than a small fix, open an issue first so we can agree on the direction.
- Check the roadmap in the README to avoid duplicating planned work.

## Development setup

```bash
sudo apt-get install -y pkg-config libasound2-dev libudev-dev \
    libx11-dev libxcursor-dev libxrandr-dev libxi-dev \
    libxkbcommon-dev libxkbcommon-x11-dev \
    libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libgl1-mesa-dev libegl1-mesa-dev libwayland-dev
rustup toolchain install stable
```

## Build and test

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
bash -n install.sh uninstall.sh build.sh
```

CI runs the same steps on Ubuntu 22.04.

## Run locally

```bash
cargo run -- daemon          # daemon in the foreground
cargo run --bin typebird-pill # pill by itself
```

The daemon spawns the pill from the same directory as the daemon binary. To test the installed layout, use `./build.sh` and run `./install.sh` from the extracted tarball.

## Release build

```bash
./build.sh
```

This builds the Rust binaries, builds the static `whisper-cli`, and packages `dist/typebird-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` with checksums.

## Commit style

- Small commits, one idea each.
- Imperative subject line, e.g. `install: replace the source build with a release installer`.
- Reference the issue when there is one.

## Pull requests

- Describe the problem and the fix.
- Note which desktop and session you tested on (X11 / Wayland, GNOME / KDE / other).
- Update the README if you add a command, change install behavior, or change claims.
- Keep the `cargo fmt` and clippy clean.

## Reporting bugs

Use the bug report template. Include the output of `typebird doctor`, your distribution and desktop, the session type, and `typebird --version`. Redact anything private.
