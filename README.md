# TortOS Installer

Puts [TortOS](https://github.com/ericreinsmidt/TortOS) on a microSD card for
the TrimUI Brick or the GKD Pixel 2, or updates a card that already has it
without touching the games, music and saves on it.

Work in progress: this is a rewrite of the earlier installer, and not usable
yet.

## Building

Everything is built on a Mac: the macOS app natively, the Windows exe and the
Linux packages in Docker (OrbStack works).

1. Install Rust with rustup, add the Intel target for the macOS app
   (`rustup target add x86_64-apple-darwin`), and install the Tauri CLI
   (`cargo install tauri-cli --version "^2"`).
2. Run `tools/release.sh`. It runs the tests, then builds all four files into
   `dist/<version>/` with their `SHA256SUMS`. The first run takes a while: it
   builds the Linux image from `tools/linux.Dockerfile` and fetches the
   Windows SDK.

`tools/release.sh --publish` then tags the commit and makes the GitHub
release from those files.

The fonts in `ui/fonts` are Josefin Sans (regular, and thin italic for the wordmark), under
the SIL Open Font License (`ui/fonts/OFL.txt`).

MIT License; see `LICENSE`.
