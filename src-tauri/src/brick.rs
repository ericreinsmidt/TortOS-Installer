// The Brick: its TortOS is a .zip of files for the card's root.
//
// A fresh install builds a FAT32 card with them in it (fat32.rs). An update
// copies them onto the card's mounted volume, no password needed, and writes
// only what the release has: what TortOS writes on the card itself (its
// settings and library in .userdata/, saves, the stock boot logo's backup in
// TortOS/, its markers) is never in the release, so it stays. Outside
// TortOS/, .tmp_update/ and trimui/, nothing on the card is replaced, only
// folders that are missing are made.
//
// Every file goes next to the old one under a temporary name first, and is
// read back. Only when all of them are there are they renamed into place, a
// moment's work, so an update cancelled or failed before that leaves the card
// as it was.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::fat32::Entry;
use crate::writer::Progress;

/// What an update replaces; anywhere else it only adds what is missing
const REPLACED: &[&str] = &["TortOS", ".tmp_update", "trimui"];
const NEW: &str = ".tortos-new";

/// Every file and folder in the release, read into memory: about 33 MB.
pub fn entries(zip_path: &Path) -> Result<Vec<Entry>, String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Opening the release: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Reading the release: {e}"))?;
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| format!("Reading the release: {e}"))?;
        let Some(path) = f.enclosed_name().and_then(|p| p.to_str().map(|s| s.replace('\\', "/"))) else {
            return Err(format!("The release has a file named {:?}, which can't go on a card", f.name()));
        };
        let time = f.last_modified().map(|t| (t.datepart() as u32) << 16 | t.timepart() as u32).unwrap_or(0);
        let data = if f.is_dir() {
            None
        } else {
            let mut data = Vec::with_capacity(f.size() as usize);
            std::io::copy(&mut f, &mut data).map_err(|e| format!("Reading {path} from the release: {e}"))?;
            Some(data)
        };
        out.push(Entry { path, time, data });
    }
    Ok(out)
}

fn replaced(path: &str) -> bool {
    REPLACED.iter().any(|r| path == *r || path.starts_with(&format!("{r}/")))
}

fn full_disk(e: &std::io::Error) -> bool {
    // ENOSPC on Linux and macOS, ERROR_DISK_FULL and ERROR_HANDLE_DISK_FULL on Windows
    matches!(e.raw_os_error(), Some(28) | Some(112) | Some(39))
}

/// macOS gives every file an app makes an attribute it won't let go of
/// (com.apple.provenance), and FAT keeps attributes in a "._" file beside it.
/// The file is whole without it; the "._" file is only clutter on the card.
fn drop_apple_double(path: &Path) {
    if cfg!(target_os = "macos") {
        if let Some(name) = path.file_name() {
            let mut dot = std::ffi::OsString::from("._");
            dot.push(name);
            let _ = fs::remove_file(path.with_file_name(dot));
        }
    }
}

/// A file written under its temporary name, flushed, and read back.
fn put(target: &Path, data: &[u8]) -> Result<PathBuf, String> {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(NEW);
    let temp = target.with_file_name(name);
    let shown = target.display();
    let write = || -> std::io::Result<()> {
        let mut f = fs::File::create(&temp)?;
        f.write_all(data)?;
        f.sync_all()
    };
    if let Err(e) = write() {
        let _ = fs::remove_file(&temp);
        drop_apple_double(&temp);
        return Err(if full_disk(&e) {
            "The card is full. Make some room on it and update again.".into()
        } else {
            format!("Writing {shown}: {e}")
        });
    }
    let mut back = Vec::with_capacity(data.len());
    fs::File::open(&temp).and_then(|mut f| f.read_to_end(&mut back)).map_err(|e| format!("Reading back {shown}: {e}"))?;
    if back != data {
        let _ = fs::remove_file(&temp);
        drop_apple_double(&temp);
        return Err(format!("{shown} didn't read back what was written. The card may be failing."));
    }
    Ok(temp)
}

/// The update, onto the card mounted at `root`.
pub fn update(
    root: &Path,
    entries: &[Entry],
    progress: &mut dyn FnMut(Progress),
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if !root.join("TortOS").is_dir() || !root.join(".tmp_update").is_dir() {
        return Err("This card doesn't have the Brick's TortOS on it to update.".into());
    }
    let total: u64 = entries.iter().filter_map(|e| e.data.as_ref()).map(|d| d.len() as u64).sum();
    let mut done = 0;
    // Each file written under its temporary name, and where it goes
    let mut ready: Vec<(PathBuf, PathBuf)> = Vec::new();
    let undo = |ready: &[(PathBuf, PathBuf)]| {
        for (temp, _) in ready {
            let _ = fs::remove_file(temp);
            drop_apple_double(temp);
        }
    };
    for entry in entries {
        if cancelled() {
            undo(&ready);
            return Err("cancelled".into());
        }
        let target = root.join(&entry.path);
        let result = match &entry.data {
            None => fs::create_dir_all(&target).map_err(|e| format!("Making {}: {e}", target.display())),
            // Outside TortOS's own folders, only what isn't there yet
            Some(_) if !replaced(&entry.path) && target.exists() => Ok(()),
            Some(data) => {
                let made = match target.parent() {
                    Some(p) => fs::create_dir_all(p).map_err(|e| format!("Making {}: {e}", p.display())),
                    None => Ok(()),
                };
                made.and_then(|_| put(&target, data)).map(|temp| {
                    ready.push((temp, target));
                    done += data.len() as u64;
                    progress(Progress::Writing { done, total });
                })
            }
        };
        if let Err(e) = result {
            undo(&ready);
            return Err(e);
        }
    }
    // Into place. Not cancelled from here: a card half renamed is what this
    // order is for avoiding
    for (i, (temp, target)) in ready.iter().enumerate() {
        if let Err(e) = fs::rename(temp, target) {
            undo(&ready[i..]);
            return Err(format!(
                "Putting {} in place: {e}. The update stopped partway: run it again.",
                target.display()
            ));
        }
        drop_apple_double(target);
    }
    progress(Progress::Checking { done: total, total });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, data: Option<&[u8]>) -> Entry {
        Entry { path: path.into(), time: 0, data: data.map(|d| d.to_vec()) }
    }

    fn card() -> PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("tortos-brick-{n}"));
        for (path, data) in [
            ("TortOS/tortos.elf", "old elf"),
            ("TortOS/systems.cfg", "old cfg"),
            ("TortOS/bootlogo.stock.bmp", "stock logo"),
            ("TortOS/.bootlogo_applied", ""),
            (".tmp_update/updater", "old updater"),
            (".userdata/tg3040/tortos.db", "settings"),
            ("Roms/NES/game.nes", "a game"),
            ("Saves/game.srm", "a save"),
            ("Bios/readme.txt", "mine"),
        ] {
            let p = root.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, data).unwrap();
        }
        root
    }

    fn release() -> Vec<Entry> {
        vec![
            entry("TortOS/", None),
            entry("TortOS/tortos.elf", Some(b"new elf")),
            entry("TortOS/systems.cfg", Some(b"new cfg")),
            entry("TortOS/cores/", None),
            entry("TortOS/cores/new_libretro.so", Some(b"core")),
            entry(".tmp_update/updater", Some(b"new updater")),
            entry("trimui/app/MainUI", Some(b"mainui")),
            entry("Roms/NES/.media/", None),
            entry("Roms/Genesis/.media/", None),
            entry("Bios/readme.txt", Some(b"theirs")),
            entry("Music/", None),
        ]
    }

    fn read(root: &Path, path: &str) -> String {
        fs::read_to_string(root.join(path)).unwrap()
    }

    fn temps(root: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut dirs = vec![root.to_path_buf()];
        while let Some(d) = dirs.pop() {
            for e in fs::read_dir(d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    dirs.push(p);
                } else if p.to_string_lossy().ends_with(NEW) {
                    found.push(p);
                }
            }
        }
        found
    }

    #[test]
    fn updates_tortos_and_keeps_the_rest() {
        let root = card();
        let mut last = None;
        update(&root, &release(), &mut |p| last = Some(p), &|| false).unwrap();
        assert_eq!(read(&root, "TortOS/tortos.elf"), "new elf");
        assert_eq!(read(&root, "TortOS/systems.cfg"), "new cfg");
        assert_eq!(read(&root, "TortOS/cores/new_libretro.so"), "core");
        assert_eq!(read(&root, ".tmp_update/updater"), "new updater");
        assert_eq!(read(&root, "trimui/app/MainUI"), "mainui");
        // What TortOS wrote, and the player's own
        assert_eq!(read(&root, "TortOS/bootlogo.stock.bmp"), "stock logo");
        assert!(root.join("TortOS/.bootlogo_applied").exists());
        assert_eq!(read(&root, ".userdata/tg3040/tortos.db"), "settings");
        assert_eq!(read(&root, "Roms/NES/game.nes"), "a game");
        assert_eq!(read(&root, "Saves/game.srm"), "a save");
        assert_eq!(read(&root, "Bios/readme.txt"), "mine");
        assert!(root.join("Roms/Genesis/.media").is_dir() && root.join("Music").is_dir());
        assert!(temps(&root).is_empty());
        assert!(matches!(last, Some(Progress::Checking { .. })));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancelled_leaves_the_card_as_it_was() {
        let root = card();
        let calls = std::cell::Cell::new(0);
        let cancel = || {
            calls.set(calls.get() + 1);
            calls.get() > 4
        };
        assert_eq!(update(&root, &release(), &mut |_| {}, &cancel), Err("cancelled".into()));
        assert_eq!(read(&root, "TortOS/tortos.elf"), "old elf");
        assert_eq!(read(&root, ".tmp_update/updater"), "old updater");
        assert!(temps(&root).is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_a_card_without_the_bricks_tortos() {
        let root = card();
        fs::remove_dir_all(root.join(".tmp_update")).unwrap();
        assert!(update(&root, &release(), &mut |_| {}, &|| false).is_err());
        assert_eq!(read(&root, "TortOS/tortos.elf"), "old elf");
        fs::remove_dir_all(root).unwrap();
    }

    /// BRICK_ZIP=TortOS-v1.3.0.zip CARD_IMAGE=out.img CARD_BYTES=...: a card
    /// image to check with fsck and to mount
    #[test]
    #[ignore]
    fn card_image_from_a_release() {
        let zip = std::env::var("BRICK_ZIP").expect("BRICK_ZIP");
        let out = std::env::var("CARD_IMAGE").expect("CARD_IMAGE");
        let bytes: u64 = std::env::var("CARD_BYTES").expect("CARD_BYTES").parse().unwrap();
        let started = std::time::Instant::now();
        let entries = entries(Path::new(&zip)).unwrap();
        let image = crate::fat32::build(bytes, entries, 0x5B47_6000, 0xC0FF_EE01).unwrap();
        println!("{} bytes to write, {} clusters of {}, in {:?}", image.len, image.clusters_used, image.clusters, started.elapsed());
        let mut f = fs::File::create(&out).unwrap();
        std::io::copy(&mut image.open(), &mut f).unwrap();
        f.set_len(bytes).unwrap();
        f.flush().unwrap();
    }
}
