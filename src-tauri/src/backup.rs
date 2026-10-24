// A Brick card moving from TrimUI's firmware to the Brick's own system is
// erased, so what can't be got back is copied off it first, and put back
// after the Brick's first start has made its games partition: Saves/,
// .userdata/ (settings, states, achievements) and Bios/. Games, music and
// books are the player's to copy: a full disk on their computer is not a risk
// to take, so the installer never copies those.
//
// The backup goes to a folder of its own in the computer's Documents, kept
// until the player deletes it. Its record, backup.json, is written last, so a
// backup that stopped partway is never offered back. It is offered for a
// started Brick card of the same size, the card it came from, until it has
// been put back.
//
// Every file is read back after it is copied, both ways. Putting back writes
// each file under a temporary name first and renames it once it reads back.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::writer::Progress;

/// What is copied off the card, from its top
pub const KEPT: &[&str] = &["Saves", ".userdata", "Bios"];
/// Room the computer keeps after the backup
pub const MARGIN: u64 = 5_000_000_000;
const RECORD: &str = "backup.json";
const NEW: &str = ".tortos-new";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Record {
    pub device: String,
    /// The card's size, to know it again
    pub card_bytes: u64,
    /// When it was made, in seconds since 1970, for the screen to show
    pub made: u64,
    pub restored: bool,
}

/// A backup waiting to go back on a card, for the screen.
#[derive(Serialize, Clone, Debug)]
pub struct Pending {
    pub path: String,
    pub made: u64,
}

/// Where backups go: TortOS backups in the computer's Documents, or the home
/// folder where there is none.
pub fn folder() -> Option<PathBuf> {
    Some(dirs::document_dir().or_else(dirs::home_dir)?.join("TortOS backups"))
}

/// The files under KEPT on a card, from its top, with their sizes. macOS's
/// own clutter (._ files, .DS_Store) is left out.
fn files(root: &Path) -> Result<Vec<(PathBuf, u64)>, String> {
    fn walk(root: &Path, rel: &Path, out: &mut Vec<(PathBuf, u64)>) -> Result<(), String> {
        let dir = root.join(rel);
        let entries = fs::read_dir(&dir).map_err(|e| format!("Reading {}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Reading {}: {e}", dir.display()))?;
            let name = entry.file_name();
            let text = name.to_string_lossy();
            if text.starts_with("._") || text == ".DS_Store" {
                continue;
            }
            let rel = rel.join(&name);
            let kind = entry.file_type().map_err(|e| format!("Reading {}: {e}", rel.display()))?;
            if kind.is_dir() {
                walk(root, &rel, out)?;
            } else if kind.is_file() {
                let len = entry.metadata().map_err(|e| format!("Reading {}: {e}", rel.display()))?.len();
                out.push((rel, len));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    for top in KEPT {
        if root.join(top).is_dir() {
            walk(root, Path::new(top), &mut out)?;
        }
    }
    Ok(out)
}

/// How much a backup of this card would be.
pub fn size(volume: &Path) -> Result<u64, String> {
    Ok(files(volume)?.iter().map(|(_, n)| n).sum())
}

/// The free space where `path` is, from its nearest folder that exists.
fn free_space(path: &Path) -> Result<u64, String> {
    let mut at = path;
    while !at.exists() {
        at = at.parent().ok_or("Finding where the backup goes")?;
    }
    free_at(at).map_err(|e| format!("Finding the free space in {}: {e}", at.display()))
}

#[cfg(unix)]
fn free_at(path: &Path) -> std::io::Result<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(s.f_bavail as u64 * s.f_frsize as u64)
}

#[cfg(windows)]
fn free_at(path: &Path) -> std::io::Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0u64;
    unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut free), None, None) }
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(free)
}

fn gb(n: u64) -> String {
    format!("{:.1} GB", n as f64 / 1e9)
}

/// A file copied and read back. `temp` writes it under a temporary name
/// first and renames it into place only once it reads back.
fn copy_checked(from: &Path, to: &Path, temp: bool) -> Result<u64, String> {
    let data = fs::read(from).map_err(|e| format!("Reading {}: {e}", from.display()))?;
    if let Some(dir) = to.parent() {
        make_dirs(dir)?;
    }
    let target = if temp {
        let mut name = to.file_name().unwrap_or_default().to_os_string();
        name.push(NEW);
        to.with_file_name(name)
    } else {
        to.to_path_buf()
    };
    let shown = to.display();
    let write = || -> std::io::Result<()> {
        let mut f = fs::File::create(&target)?;
        f.write_all(&data)?;
        f.sync_all()
    };
    write().map_err(|e| format!("Writing {shown}: {e}"))?;
    if fs::read(&target).map_err(|e| format!("Reading {shown} back: {e}"))? != data {
        let _ = fs::remove_file(&target);
        return Err(format!("{shown} didn't read back as it was written."));
    }
    if temp {
        fs::rename(&target, to).map_err(|e| format!("Putting {shown} in place: {e}"))?;
    }
    drop_apple_double(to);
    Ok(data.len() as u64)
}

/// A folder and any missing above it, each without the "._" file macOS
/// leaves beside a folder it makes on a card.
fn make_dirs(dir: &Path) -> Result<(), String> {
    if dir.is_dir() {
        return Ok(());
    }
    if let Some(up) = dir.parent() {
        make_dirs(up)?;
    }
    match fs::create_dir(dir) {
        Ok(()) => drop_apple_double(dir),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(format!("Making {}: {e}", dir.display())),
    }
    Ok(())
}

/// macOS gives every file an app makes an attribute it won't let go of
/// (com.apple.provenance), and FAT and exFAT keep attributes in a "._" file
/// beside it, a folder's too. The file is whole without it; the "._" file is
/// only clutter.
fn drop_apple_double(path: &Path) {
    if cfg!(target_os = "macos") {
        if let Some(name) = path.file_name() {
            let mut dot = std::ffi::OsString::from("._");
            dot.push(name);
            let _ = fs::remove_file(path.with_file_name(dot));
        }
    }
}

/// The card's Saves/, .userdata/ and Bios/ into a new folder under `parent`,
/// named `label` (a second one the same minute gets " 2"), after checking the
/// computer keeps MARGIN free after it. Returns the folder.
pub fn back_up(
    volume: &Path,
    parent: &Path,
    label: &str,
    card_bytes: u64,
    made: u64,
    progress: &mut dyn FnMut(Progress),
    cancelled: &dyn Fn() -> bool,
) -> Result<PathBuf, String> {
    back_up_with(free_space(parent)?, volume, parent, label, card_bytes, made, progress, cancelled)
}

/// back_up, given the free space: apart for the tests.
#[allow(clippy::too_many_arguments)]
fn back_up_with(
    free: u64,
    volume: &Path,
    parent: &Path,
    label: &str,
    card_bytes: u64,
    made: u64,
    progress: &mut dyn FnMut(Progress),
    cancelled: &dyn Fn() -> bool,
) -> Result<PathBuf, String> {
    let list = files(volume)?;
    let total: u64 = list.iter().map(|(_, n)| n).sum();
    if free < total + MARGIN {
        return Err(format!(
            "This computer needs {} free for the backup and 5 GB to spare, and has {}. Nothing was \
             written to the card.",
            gb(total),
            gb(free)
        ));
    }
    let mut dest = parent.join(label);
    let mut n = 2;
    while dest.exists() {
        dest = parent.join(format!("{label} {n}"));
        n += 1;
    }
    fs::create_dir_all(&dest).map_err(|e| format!("Making {}: {e}", dest.display()))?;
    let stopped = |e: String| format!("{e} The backup stopped, and nothing was written to the card.");
    let mut done = 0;
    progress(Progress::Copying { done, total });
    for (rel, _) in &list {
        if cancelled() {
            return Err("cancelled".into());
        }
        done += copy_checked(&volume.join(rel), &dest.join(rel), false).map_err(stopped)?;
        progress(Progress::Copying { done, total });
    }
    let record = Record { device: "brick".into(), card_bytes, made, restored: false };
    write_record(&dest, &record).map_err(stopped)?;
    Ok(dest)
}

fn write_record(dest: &Path, record: &Record) -> Result<(), String> {
    let text = serde_json::to_string_pretty(record).map_err(|e| e.to_string())?;
    fs::write(dest.join(RECORD), text).map_err(|e| format!("Writing the backup's record: {e}"))
}

fn read_record(dir: &Path) -> Option<Record> {
    serde_json::from_str(&fs::read_to_string(dir.join(RECORD)).ok()?).ok()
}

/// The newest backup under `parent` waiting to go back on a Brick card of
/// this size.
pub fn pending(parent: &Path, card_bytes: u64) -> Option<Pending> {
    fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| Some((e.path(), read_record(&e.path())?)))
        .filter(|(_, r)| r.device == "brick" && !r.restored && r.card_bytes == card_bytes)
        .max_by_key(|(_, r)| r.made)
        .map(|(path, r)| Pending { path: path.to_string_lossy().into_owned(), made: r.made })
}

/// Where a backed-up file goes on the Brick's own system: the settings
/// folder TrimUI's model named, .userdata/tg3040, is .userdata/brick now.
fn on_card(rel: &Path) -> PathBuf {
    match rel.strip_prefix(".userdata/tg3040") {
        Ok(rest) => Path::new(".userdata/brick").join(rest),
        Err(_) => rel.to_path_buf(),
    }
}

/// The backup in `dir` back onto the card at `volume`. Each folder of
/// .userdata it holds replaces the card's (moved aside as NAME.before-restore,
/// never deleted): the first start's fresh settings database beside the old
/// one's journal would spoil it. Saves/ and Bios/ are merged, a file already
/// on the card kept. Marks the backup restored.
pub fn restore(
    dir: &Path,
    volume: &Path,
    progress: &mut dyn FnMut(Progress),
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    let mut record = read_record(dir).ok_or("That backup's record can't be read.")?;
    let mut list = files(dir)?;
    // A backup with both settings folders (never from TrimUI's firmware) keeps
    // the newer name's
    if list.iter().any(|(r, _)| r.starts_with(".userdata/brick")) {
        list.retain(|(r, _)| !r.starts_with(".userdata/tg3040"));
    }
    // The .userdata folders the backup has files in, such as .userdata/brick
    let mut replaced: Vec<PathBuf> = list
        .iter()
        .filter_map(|(r, _)| {
            let p = on_card(r);
            let mut parts = p.components();
            let (top, folder) = (parts.next()?, parts.next()?);
            parts.next()?;
            (top.as_os_str() == ".userdata").then(|| Path::new(".userdata").join(folder.as_os_str()))
        })
        .filter(|p| volume.join(p).is_dir())
        .collect();
    replaced.sort();
    replaced.dedup();
    for p in &replaced {
        let from = volume.join(p);
        let mut aside = volume.join(format!("{}.before-restore", p.display()));
        let mut n = 2;
        while aside.exists() {
            aside = volume.join(format!("{}.before-restore {n}", p.display()));
            n += 1;
        }
        fs::rename(&from, &aside).map_err(|e| format!("Moving {} aside: {e}", from.display()))?;
    }
    let total: u64 = list.iter().map(|(_, n)| n).sum();
    let mut done = 0;
    progress(Progress::Copying { done, total });
    for (rel, len) in &list {
        if cancelled() {
            return Err("cancelled".into());
        }
        let to = volume.join(on_card(rel));
        if rel.starts_with(".userdata") || !to.exists() {
            copy_checked(&dir.join(rel), &to, true)?;
        }
        done += len;
        progress(Progress::Copying { done, total });
    }
    record.restored = true;
    write_record(dir, &record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, read_to_string, write};

    fn fresh(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("tortos-installer-backup-{name}"));
        let _ = fs::remove_dir_all(&root);
        create_dir_all(&root).unwrap();
        root
    }

    /// A Brick card on TrimUI's firmware, as TortOS 1.6.0 leaves it
    fn trimui_card(root: &Path) {
        for (path, data) in [
            ("Saves/Contra (USA).srm", "a save"),
            (".userdata/tg3040/tortos.db", "old settings"),
            (".userdata/tg3040/tortos.db-wal", "old journal"),
            (".userdata/shared/states/Contra (USA).st0", "a state"),
            ("Bios/gba_bios.bin", "a bios"),
            ("Roms/NES/Contra (USA).nes", "a game"),
            ("Saves/._Contra (USA).srm", "clutter"),
        ] {
            create_dir_all(root.join(path).parent().unwrap()).unwrap();
            write(root.join(path), data).unwrap();
        }
    }

    fn back_up_now(card: &Path, parent: &Path) -> Result<PathBuf, String> {
        back_up(card, parent, "Brick 2026-10-24 1405", 64_000_000_000, 1_792_840_000, &mut |_| {}, &|| false)
    }

    #[test]
    fn backs_up_only_what_cant_be_got_back() {
        let root = fresh("only");
        trimui_card(&root.join("card"));
        let dest = back_up_now(&root.join("card"), &root.join("backups")).unwrap();
        assert_eq!(read_to_string(dest.join("Saves/Contra (USA).srm")).unwrap(), "a save");
        assert_eq!(read_to_string(dest.join(".userdata/tg3040/tortos.db")).unwrap(), "old settings");
        assert_eq!(read_to_string(dest.join("Bios/gba_bios.bin")).unwrap(), "a bios");
        assert!(!dest.join("Roms").exists(), "games are the player's to copy");
        assert!(!dest.join("Saves/._Contra (USA).srm").exists());
        assert_eq!(size(&root.join("card")).unwrap(), 6 + 12 + 11 + 7 + 6);
        let p = pending(&root.join("backups"), 64_000_000_000).unwrap();
        assert_eq!((Path::new(&p.path), p.made), (dest.as_path(), 1_792_840_000));
        assert!(pending(&root.join("backups"), 32_000_000_000).is_none(), "another card");
        // The same minute again: a folder of its own
        assert!(back_up_now(&root.join("card"), &root.join("backups")).unwrap().ends_with("Brick 2026-10-24 1405 2"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_backup_cut_short_is_never_offered() {
        let root = fresh("short");
        trimui_card(&root.join("card"));
        let n = std::cell::Cell::new(0);
        let err = back_up(&root.join("card"), &root.join("backups"), "Brick", 64, 1, &mut |_| {}, &|| {
            n.set(n.get() + 1);
            n.get() > 2
        });
        assert_eq!(err.unwrap_err(), "cancelled");
        assert!(pending(&root.join("backups"), 64).is_none(), "no record, no offer");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restores_onto_the_bricks_own_system() {
        let root = fresh("restore");
        let card = root.join("card");
        trimui_card(&card);
        let dest = back_up_now(&card, &root.join("backups")).unwrap();
        // The card as the Brick's first start leaves it
        let _ = fs::remove_dir_all(&card);
        for (path, data) in [
            (".userdata/brick/tortos.db", "new settings"),
            (".userdata/brick/tortos.db-wal", "new journal"),
            (".userdata/shared/achievements.db", "new"),
            ("Saves/Played since (USA).srm", "new save"),
            ("Bios/gba_bios.bin", "the player's newer bios"),
        ] {
            create_dir_all(card.join(path).parent().unwrap()).unwrap();
            write(card.join(path), data).unwrap();
        }
        restore(&dest, &card, &mut |_| {}, &|| false).unwrap();
        let read = |p: &str| read_to_string(card.join(p)).unwrap();
        assert_eq!(read(".userdata/brick/tortos.db"), "old settings");
        assert_eq!(read(".userdata/brick/tortos.db-wal"), "old journal", "the old database's own journal, not the new one's");
        assert_eq!(read(".userdata/shared/states/Contra (USA).st0"), "a state");
        assert!(!card.join(".userdata/shared/achievements.db").exists(), "shared replaced");
        assert_eq!(read(".userdata/brick.before-restore/tortos.db"), "new settings", "moved aside, not deleted");
        assert!(!card.join(".userdata/tg3040").exists());
        assert_eq!(read("Saves/Contra (USA).srm"), "a save");
        assert_eq!(read("Saves/Played since (USA).srm"), "new save");
        assert_eq!(read("Bios/gba_bios.bin"), "the player's newer bios", "a file already there is kept");
        assert!(pending(&root.join("backups"), 64_000_000_000).is_none(), "restored, so offered no more");
        assert!(dest.join("Saves/Contra (USA).srm").exists(), "the backup is kept");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn without_room_to_spare_nothing_is_copied() {
        let root = fresh("room");
        trimui_card(&root.join("card"));
        let total = size(&root.join("card")).unwrap();
        let short = back_up_with(total + MARGIN - 1, &root.join("card"), &root.join("backups"), "Brick", 64, 1,
                                 &mut |_| {}, &|| false);
        assert!(short.unwrap_err().contains("5 GB to spare"));
        assert!(!root.join("backups").exists(), "nothing made");
        back_up_with(total + MARGIN, &root.join("card"), &root.join("backups"), "Brick", 64, 1, &mut |_| {}, &|| false)
            .unwrap();
        // And the real free space, from a folder not made yet
        assert!(free_space(&root.join("backups/not/yet/made")).unwrap() > 0);
        let _ = fs::remove_dir_all(&root);
    }
}
