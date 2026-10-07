# TortOS Installer

Puts [TortOS](https://github.com/ericreinsmidt/TortOS) on a microSD card for
the TrimUI Brick or the GKD Pixel 2, or updates a card that already has it
without touching the games, music and saves on it.

## Download

Get it from the [latest release](https://github.com/ericreinsmidt/TortOS-Installer/releases/latest):

| | |
|---|---|
| **macOS** (Apple Silicon and Intel) | `TortOS-Installer-<version>-macOS.zip` |
| **Windows** 10 and 11 | `TortOS-Installer-<version>-Windows.exe` |
| **Linux** (Debian, Ubuntu, Mint) | `TortOS-Installer-<version>-amd64.deb` |
| **Linux** (Fedora) | `TortOS-Installer-<version>-x86_64.rpm` |

The first time it opens:

- **macOS:** unzip it and open the app. It isn't signed by Apple, so macOS
  stops it the first time: open *System Settings > Privacy & Security*,
  scroll down, and choose *Open Anyway*. Writing a card asks for your
  password.
- **Windows:** the exe needs nothing installed. If Windows says it protected
  your PC, choose *More info* and *Run anyway*. It asks to run as
  administrator, which writing a card needs.
- **Linux:** install the package, `sudo apt install ./TortOS-Installer-*.deb`
  or `sudo dnf install ./TortOS-Installer-*.rpm`, and open TortOS Installer
  from your apps. Writing a card asks for your password.

## Using it

Put the card in your computer. The installer finds it and reads what is on
it, then offers what fits:

- **Update** a card that already has TortOS: the newest TortOS goes on, and
  the games, music, saves and settings stay.
- **Fresh install** on any card: it is erased, and TortOS goes on. A Brick
  card is formatted FAT32, which Windows can't do itself on a card over 32 GB.

TortOS is downloaded from its latest release on GitHub and checked against
the release's checksums before anything is written, and everything written is
read back.

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
