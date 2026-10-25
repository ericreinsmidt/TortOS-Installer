#!/usr/bin/env python3
"""THIRD-PARTY.md: what TortOS Installer is built from that isn't its own,
made from Cargo's records, so the list is the build's and not a hand-kept
one: every crate compiled into the macOS, Windows or Linux app, with its
version, license, repository and which apps carry it.

usage: tools/third-party.py > THIRD-PARTY.md   (run from the repository)

tools/release.sh stops when the committed file no longer matches."""
import json
import subprocess

TARGETS = [
    ("macOS", "aarch64-apple-darwin"),
    ("Windows", "x86_64-pc-windows-msvc"),
    ("Linux", "x86_64-unknown-linux-gnu"),
]


def cargo(*args):
    return subprocess.run(["cargo", *args, "--locked"], cwd="src-tauri", check=True,
                          capture_output=True, text=True).stdout


def shipped(target):
    """name@version of every crate built into the app for this target: its
    normal dependencies, not the build scripts' or the tests'"""
    out = cargo("tree", "--target", target, "-e", "normal", "--prefix", "none", "--format", "{p}")
    crates = set()
    for line in out.splitlines():
        parts = line.split()
        if len(parts) >= 2:
            crates.add(f"{parts[0]}@{parts[1].lstrip('v')}")
    return crates


def main():
    meta = json.loads(cargo("metadata", "--format-version", "1"))
    info = {f"{p['name']}@{p['version']}": p for p in meta["packages"]}
    apps = {}
    for name, target in TARGETS:
        for crate in shipped(target):
            apps.setdefault(crate, []).append(name)
    apps = {c: a for c, a in apps.items() if not c.startswith("tortos-installer@")}
    print("""# Third-party notices

TortOS Installer's own code is licensed under the GNU General Public License,
version 3 or later; see `LICENSE`. Versions up to 2.0.0 were MIT.

It is built with **Tauri** (https://tauri.app) and the Rust crates below, which
keep their own licenses. Each app shows its window through the system's own web
view, which the installer doesn't carry: WebKit on macOS, Microsoft Edge
WebView2 on Windows, WebKitGTK on Linux.

The fonts in `ui/fonts` are **Josefin Sans** (regular, and thin italic for the
wordmark), by Santiago Orozco, under the SIL Open Font License 1.1
(`ui/fonts/OFL.txt`).

## Crates

Every crate compiled into an app, from Cargo's own records
(`tools/third-party.py`; `tools/release.sh` stops when this list no longer
matches the build). The ones that only run while building it aren't listed.
""")
    print("| Crate | Version | License | Repository | In |")
    print("|---|---|---|---|---|")
    for crate in sorted(apps, key=lambda c: (c.split("@")[0].lower(), c)):
        p = info.get(crate, {})
        name, version = crate.split("@", 1)
        cells = [name, version, p.get("license") or "see its repository",
                 p.get("repository") or "", ", ".join(apps[crate])]
        print("| " + " | ".join(c.replace("|", "/") for c in cells) + " |")


main()
