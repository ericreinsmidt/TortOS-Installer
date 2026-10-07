// The cards in this computer, and what is on each.
//
// A card is a whole disk that is removable media: an SD card in a reader, or
// a flash drive. Fixed disks are never offered, external ones included, so a
// backup drive can't be picked by mistake. On a Mac that means asking for
// "removable media", not "external": the built-in SD reader calls itself
// internal, and a listing of external disks misses the card in it.
//
// What is on a card decides what the installer offers. The GKD Pixel 2's card
// has a PX2BOOT partition; the Brick's has TortOS/ beside .tmp_update/ at the
// top. TortOS/VERSION says which TortOS, from v1.4.0 on; a card made before
// that has none, and shows as older.

use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Device {
    Brick,
    Pixel2,
}

#[derive(Serialize, Clone, Debug)]
pub struct Card {
    /// How this system names the disk: disk4 on a Mac
    pub id: String,
    /// The volume's label where there is one, else the reader's name
    pub name: String,
    pub size_bytes: u64,
    /// The reader or drive, as the system names it
    pub reader: String,
    /// Which device's TortOS is on it, if any
    pub device: Option<Device>,
    /// TortOS's version from TortOS/VERSION; None with a device means older
    /// than v1.4.0, which wrote no such file
    pub version: Option<String>,
    pub games: u32,
    pub songs: u32,
    pub books: u32,
    /// False for a Pixel 2 card that has never been started: its games
    /// partition, TORTOS, is made by its first start, and until then there is
    /// no TortOS/ and no version to read
    pub started: bool,
}

const AUDIO: &[&str] = &["mp3", "m4a", "m4b", "aac", "flac", "ogg", "opus", "wav"];

fn visible(name: &str) -> bool {
    !name.starts_with('.')
}

fn entries(dir: &Path) -> Vec<fs::DirEntry> {
    fs::read_dir(dir)
        .map(|it| {
            it.flatten()
                .filter(|e| visible(&e.file_name().to_string_lossy()))
                .collect()
        })
        .unwrap_or_default()
}

/// Games: each file or folder in a console's folder under Roms/, as a game
/// with several discs keeps them in a folder of its own.
fn count_games(root: &Path) -> u32 {
    entries(&root.join("Roms"))
        .iter()
        .filter(|console| console.path().is_dir())
        .map(|console| entries(&console.path()).len() as u32)
        .sum()
}

/// Songs: every audio file anywhere under Music/.
fn count_songs(dir: &Path) -> u32 {
    entries(dir)
        .iter()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                count_songs(&p)
            } else {
                let ext = p.extension().map(|x| x.to_string_lossy().to_lowercase());
                u32::from(ext.is_some_and(|x| AUDIO.contains(&x.as_str())))
            }
        })
        .sum()
}

/// Audiobooks: each folder or audio file straight in Audiobooks/ is one book,
/// however many chapter files it holds.
fn count_books(root: &Path) -> u32 {
    entries(&root.join("Audiobooks"))
        .iter()
        .filter(|e| {
            let p = e.path();
            p.is_dir()
                || p.extension()
                    .map(|x| x.to_string_lossy().to_lowercase())
                    .is_some_and(|x| AUDIO.contains(&x.as_str()))
        })
        .count() as u32
}

fn read_version(root: &Path) -> Option<String> {
    let v = fs::read_to_string(root.join("TortOS/VERSION")).ok()?;
    let v = v.trim();
    (!v.is_empty()).then(|| v.to_string())
}

/// Fills in what a mounted volume holds. `pixel` is whether the disk carries
/// the Pixel 2's PX2BOOT partition.
fn read_contents(card: &mut Card, root: &Path, pixel: bool) {
    let tortos = root.join("TortOS").is_dir();
    card.device = if pixel {
        Some(Device::Pixel2)
    } else if tortos && root.join(".tmp_update").is_dir() {
        Some(Device::Brick)
    } else {
        None
    };
    if tortos {
        card.version = read_version(root);
    }
    card.games = count_games(root);
    card.songs = count_songs(&root.join("Music"));
    card.books = count_books(root);
}

#[cfg(target_os = "macos")]
mod system {
    use super::*;
    use plist::Value;
    use std::process::Command;

    fn diskutil(args: &[&str]) -> Option<Value> {
        let out = Command::new("diskutil").args(args).output().ok()?;
        if !out.status.success() {
            return None;
        }
        Value::from_reader_xml(&out.stdout[..]).ok()
    }

    fn text(d: &plist::Dictionary, key: &str) -> String {
        d.get(key).and_then(Value::as_string).unwrap_or("").to_string()
    }

    fn flag(d: &plist::Dictionary, key: &str) -> bool {
        d.get(key).and_then(Value::as_boolean).unwrap_or(false)
    }

    pub fn list() -> Vec<Card> {
        let Some(all) = diskutil(&["list", "-plist"]) else { return Vec::new() };
        let Some(disks) = all
            .as_dictionary()
            .and_then(|d| d.get("AllDisksAndPartitions"))
            .and_then(Value::as_array)
        else {
            return Vec::new();
        };
        let mut cards = Vec::new();
        for disk in disks.iter().filter_map(Value::as_dictionary) {
            let id = text(disk, "DeviceIdentifier");
            let Some(info) = diskutil(&["info", "-plist", &id]) else { continue };
            let Some(info) = info.as_dictionary() else { continue };
            // A card: removable media, a real device, not a disk image
            if !flag(info, "RemovableMedia")
                || text(info, "VirtualOrPhysical") == "Virtual"
                || text(info, "BusProtocol") == "Disk Image"
            {
                continue;
            }
            let partitions: Vec<&plist::Dictionary> = disk
                .get("Partitions")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_dictionary).collect())
                .unwrap_or_default();
            let pixel = partitions.iter().any(|p| text(p, "VolumeName") == "PX2BOOT");
            let started = !pixel || partitions.iter().any(|p| text(p, "VolumeName") == "TORTOS");
            let size = info.get("TotalSize").and_then(Value::as_unsigned_integer).unwrap_or(0);
            let mut card = Card {
                id: id.clone(),
                name: text(info, "MediaName"),
                size_bytes: size,
                reader: text(info, "MediaName"),
                device: None,
                version: None,
                games: 0,
                songs: 0,
                books: 0,
                started,
            };
            // The largest partition is where the games are: TORTOS on a Pixel
            // 2's card, never its system partition
            let main = partitions
                .iter()
                .max_by_key(|p| p.get("Size").and_then(Value::as_unsigned_integer).unwrap_or(0));
            if let Some(name) = main.map(|p| text(p, "VolumeName")).filter(|n| !n.is_empty()) {
                card.name = name;
            }
            match main.map(|p| text(p, "MountPoint")).filter(|m| !m.is_empty()) {
                Some(m) => read_contents(&mut card, Path::new(&m), pixel),
                None => card.device = pixel.then_some(Device::Pixel2),
            }
            cards.push(card);
        }
        cards
    }
}

/// Linux: lsblk, which every distribution has, for disks and their
/// partitions' labels and mount points. A card shows as the Pixel 2's from its
/// PX2BOOT label even before the desktop mounts it; what is on it can only be
/// read once it is mounted, which a desktop does by itself.
#[cfg(target_os = "linux")]
mod system {
    use super::*;
    use serde_json::Value;
    use std::process::Command;

    pub fn list() -> Vec<Card> {
        let out = Command::new("lsblk")
            .args(["-J", "-b", "-o", "NAME,PATH,SIZE,RM,HOTPLUG,TYPE,MODEL,LABEL,MOUNTPOINT"])
            .output();
        match out {
            Ok(o) if o.status.success() => parse(&String::from_utf8_lossy(&o.stdout)),
            _ => Vec::new(),
        }
    }

    // lsblk's JSON changed over the years: flags and sizes were strings
    // ("1", "64021856256") before util-linux 2.33 and are true and numbers since
    fn truthy(v: Option<&Value>) -> bool {
        match v {
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => s == "1",
            Some(Value::Number(n)) => n.as_u64() == Some(1),
            _ => false,
        }
    }

    fn number(v: Option<&Value>) -> u64 {
        match v {
            Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
            Some(Value::String(s)) => s.parse().unwrap_or(0),
            _ => 0,
        }
    }

    fn text<'a>(v: &'a Value, key: &str) -> &'a str {
        v.get(key).and_then(Value::as_str).unwrap_or("").trim()
    }

    /// The running system's own disk is never a card: on a Raspberry Pi it is
    /// an SD card too.
    fn holds_the_system(parts: &[Value]) -> bool {
        parts.iter().any(|p| {
            let m = text(p, "mountpoint");
            m == "/" || m.starts_with("/boot") || m == "[SWAP]"
        })
    }

    pub(super) fn parse(json: &str) -> Vec<Card> {
        let Ok(root) = serde_json::from_str::<Value>(json) else { return Vec::new() };
        let Some(devices) = root.get("blockdevices").and_then(Value::as_array) else {
            return Vec::new();
        };
        let mut cards = Vec::new();
        for d in devices {
            let name = text(d, "name");
            let removable = truthy(d.get("rm")) || truthy(d.get("hotplug")) || name.starts_with("mmcblk");
            let size = number(d.get("size"));
            if text(d, "type") != "disk" || !removable || size == 0 {
                continue;
            }
            let parts: Vec<Value> = d.get("children").and_then(Value::as_array).cloned().unwrap_or_default();
            if holds_the_system(&parts) {
                continue;
            }
            let pixel = parts.iter().any(|p| text(p, "label") == "PX2BOOT");
            let started = !pixel || parts.iter().any(|p| text(p, "label") == "TORTOS");
            let model = text(d, "model");
            let mut card = Card {
                id: text(d, "path").to_string(),
                name: if model.is_empty() { name.to_string() } else { model.to_string() },
                size_bytes: size,
                reader: model.to_string(),
                device: None,
                version: None,
                games: 0,
                songs: 0,
                books: 0,
                started,
            };
            // The largest partition is where the games are: TORTOS on a Pixel
            // 2's card, never its system partition, which a desktop may mount
            // too (and whose label, rootfs, once named the card)
            let main = parts.iter().max_by_key(|p| number(p.get("size")));
            if let Some(l) = main.map(|p| text(p, "label")).filter(|l| !l.is_empty()) {
                card.name = l.to_string();
            }
            match main.map(|p| text(p, "mountpoint")).filter(|m| !m.is_empty()) {
                Some(m) => read_contents(&mut card, Path::new(m), pixel),
                None => card.device = pixel.then_some(Device::Pixel2),
            }
            cards.push(card);
        }
        cards
    }
}

/// Windows: the removable drive letters, grouped by the disk they are on, so
/// a Pixel 2's card (PX2BOOT and TORTOS) shows once. A letter with nothing
/// readable behind it is an empty slot in a card reader and is skipped.
#[cfg(windows)]
mod system {
    use super::*;
    use std::collections::BTreeMap;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW, FILE_FLAGS_AND_ATTRIBUTES,
        FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::Ioctl::{IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, IOCTL_STORAGE_GET_DEVICE_NUMBER};
    use windows::Win32::System::IO::DeviceIoControl;

    const DRIVE_REMOVABLE: u32 = 2;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// Opened with no access at all: enough to ask questions, never to write.
    fn open(path: &str) -> Option<HANDLE> {
        let w = wide(path);
        unsafe {
            CreateFileW(PCWSTR(w.as_ptr()), 0, FILE_SHARE_READ | FILE_SHARE_WRITE, None, OPEN_EXISTING,
                        FILE_FLAGS_AND_ATTRIBUTES(0), None)
        }
        .ok()
    }

    fn disk_number(letter: char) -> Option<u32> {
        let h = open(&format!("\\\\.\\{letter}:"))?;
        // STORAGE_DEVICE_NUMBER: type, number, partition
        let mut out = [0u32; 3];
        let mut n = 0u32;
        let ok = unsafe {
            DeviceIoControl(h, IOCTL_STORAGE_GET_DEVICE_NUMBER, None, 0, Some(out.as_mut_ptr() as *mut _),
                            std::mem::size_of_val(&out) as u32, Some(&mut n), None)
        }
        .is_ok();
        unsafe { let _ = CloseHandle(h); }
        ok.then_some(out[1])
    }

    /// The disk's size, 0 when there is no medium. The geometry request, as
    /// it needs no access to the disk: asking for its length needs read
    /// access, which takes an administrator, and quietly gave 0 without one
    /// (2026-10-07). An empty slot in a reader refuses it, or says 0.
    fn disk_size(disk: u32) -> u64 {
        let Some(h) = open(&format!("\\\\.\\PhysicalDrive{disk}")) else { return 0 };
        // DISK_GEOMETRY_EX: a 24-byte DISK_GEOMETRY, then the size; u64s for
        // the alignment, and room for the partition and detection data after
        let mut out = [0u64; 32];
        let mut n = 0u32;
        let ok = unsafe {
            DeviceIoControl(h, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, None, 0, Some(out.as_mut_ptr() as *mut _),
                            std::mem::size_of_val(&out) as u32, Some(&mut n), None)
        }
        .is_ok();
        unsafe { let _ = CloseHandle(h); }
        if ok && n >= 32 { out[3] } else { 0 }
    }

    /// The volume's label, or None when there is no volume to read: an empty
    /// card reader slot.
    fn label(letter: char) -> Option<String> {
        let root = wide(&format!("{letter}:\\"));
        let mut name = [0u16; 261];
        unsafe { GetVolumeInformationW(PCWSTR(root.as_ptr()), Some(&mut name), None, None, None, None) }.ok()?;
        let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        Some(String::from_utf16_lossy(&name[..len]))
    }

    pub fn list() -> Vec<Card> {
        let bits = unsafe { GetLogicalDrives() };
        // disk number -> its letters and their labels
        let mut disks: BTreeMap<u32, Vec<(char, String)>> = BTreeMap::new();
        for i in 0..26u8 {
            if bits & (1 << i) == 0 {
                continue;
            }
            let letter = (b'A' + i) as char;
            let root = wide(&format!("{letter}:\\"));
            if unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) } != DRIVE_REMOVABLE {
                continue;
            }
            let (Some(disk), Some(l)) = (disk_number(letter), label(letter)) else { continue };
            disks.entry(disk).or_default().push((letter, l));
        }
        let mut cards = Vec::new();
        for (disk, volumes) in disks {
            // An empty slot answers for its letter but has no size: no card
            let size = disk_size(disk);
            if size == 0 {
                continue;
            }
            let pixel = volumes.iter().any(|(_, l)| l == "PX2BOOT");
            let started = !pixel || volumes.iter().any(|(_, l)| l == "TORTOS");
            let games = volumes.iter().find(|(_, l)| !pixel || l != "PX2BOOT").cloned();
            let mut card = Card {
                id: format!("\\\\.\\PhysicalDrive{disk}"),
                name: String::new(),
                size_bytes: size,
                reader: format!("Removable drive {disk}"),
                device: None,
                version: None,
                games: 0,
                songs: 0,
                books: 0,
                started,
            };
            match games {
                Some((letter, l)) => {
                    card.name = if l.is_empty() { format!("{letter}:") } else { l };
                    read_contents(&mut card, Path::new(&format!("{letter}:\\")), pixel);
                }
                None => {
                    card.name = "PX2BOOT".to_string();
                    card.device = pixel.then_some(Device::Pixel2);
                }
            }
            cards.push(card);
        }
        cards
    }
}

pub fn list() -> Vec<Card> {
    system::list()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};

    fn card() -> Card {
        Card { id: "t".into(), name: String::new(), size_bytes: 0, reader: String::new(),
               device: None, version: None, games: 0, songs: 0, books: 0, started: true }
    }

    #[test]
    fn reads_a_pixel_card() {
        let root = std::env::temp_dir().join("tortos-installer-cards-pixel");
        let _ = fs::remove_dir_all(&root);
        create_dir_all(root.join("TortOS")).unwrap();
        write(root.join("TortOS/VERSION"), "1.4.0\n").unwrap();
        create_dir_all(root.join("Roms/NES/.media")).unwrap();
        write(root.join("Roms/NES/Contra (USA).nes"), "").unwrap();
        write(root.join("Roms/NES/.hidden"), "").unwrap();
        create_dir_all(root.join("Roms/PS/Final Fantasy VII")).unwrap();
        create_dir_all(root.join("Music/Band/Album/.media")).unwrap();
        write(root.join("Music/Band/Album/01 Song.flac"), "").unwrap();
        write(root.join("Music/Band/Album/cover.jpg"), "").unwrap();
        write(root.join("Music/Singles.MP3"), "").unwrap();
        create_dir_all(root.join("Audiobooks/A Book")).unwrap();
        write(root.join("Audiobooks/Another.m4b"), "").unwrap();
        write(root.join("Audiobooks/notes.txt"), "").unwrap();

        let mut c = card();
        read_contents(&mut c, &root, true);
        assert_eq!(c.device, Some(Device::Pixel2));
        assert_eq!(c.version.as_deref(), Some("1.4.0"));
        assert_eq!((c.games, c.songs, c.books), (2, 2, 2));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reads_an_older_brick_card() {
        let root = std::env::temp_dir().join("tortos-installer-cards-brick");
        let _ = fs::remove_dir_all(&root);
        create_dir_all(root.join("TortOS")).unwrap();
        create_dir_all(root.join(".tmp_update")).unwrap();
        let mut c = card();
        read_contents(&mut c, &root, false);
        assert_eq!(c.device, Some(Device::Brick));
        assert_eq!(c.version, None, "v1.3.0 wrote no VERSION");
        let _ = fs::remove_dir_all(&root);
    }

    /// lsblk as Ubuntu 22.04 prints it, with a Pixel 2's card in a USB reader
    /// (TORTOS mounted by the desktop), the system's own disk, and an empty
    /// reader slot.
    #[cfg(target_os = "linux")]
    #[test]
    fn reads_lsblk() {
        let json = r#"{"blockdevices": [
          {"name":"sda","path":"/dev/sda","size":512110190592,"rm":false,"hotplug":false,"type":"disk","model":"Samsung SSD","label":null,"mountpoint":null,
           "children":[{"name":"sda1","path":"/dev/sda1","size":512109142016,"rm":false,"hotplug":false,"type":"part","model":null,"label":null,"mountpoint":"/"}]},
          {"name":"sdb","path":"/dev/sdb","size":62534975488,"rm":true,"hotplug":true,"type":"disk","model":"USB3.0 CRW -SD","label":null,"mountpoint":null,
           "children":[{"name":"sdb1","path":"/dev/sdb1","size":67108864,"rm":true,"hotplug":true,"type":"part","model":null,"label":"PX2BOOT","mountpoint":null},
                       {"name":"sdb2","path":"/dev/sdb2","size":268435456,"rm":true,"hotplug":true,"type":"part","model":null,"label":"rootfs","mountpoint":"/media/user/rootfs"},
                       {"name":"sdb3","path":"/dev/sdb3","size":62199431168,"rm":true,"hotplug":true,"type":"part","model":null,"label":"TORTOS","mountpoint":"/nonexistent/TORTOS"}]},
          {"name":"sdc","path":"/dev/sdc","size":0,"rm":true,"hotplug":true,"type":"disk","model":"Card Reader","label":null,"mountpoint":null}
        ]}"#;
        let cards = system::parse(json);
        assert_eq!(cards.len(), 1, "only the card: not the system disk, not the empty slot");
        assert_eq!(cards[0].id, "/dev/sdb");
        assert_eq!(cards[0].name, "TORTOS");
        assert_eq!(cards[0].device, Some(Device::Pixel2));
        // And the strings older lsblk printed
        let old = json.replace("\"rm\":true", "\"rm\":\"1\"").replace("\"size\":62534975488", "\"size\":\"62534975488\"");
        assert_eq!(system::parse(&old).len(), 1);
    }

    /// lsblk output captured on a real machine, parsed: LSBLK_JSON=file
    /// cargo test lsblk_file -- --ignored --nocapture
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore]
    fn lsblk_file() {
        let json = fs::read_to_string(std::env::var("LSBLK_JSON").expect("LSBLK_JSON")).unwrap();
        for c in system::parse(&json) {
            println!("{c:?}");
        }
    }

    /// What this computer's cards are, printed: `cargo test print_cards --
    /// --ignored --nocapture`. Reads only.
    #[test]
    #[ignore]
    fn print_cards() {
        for c in list() {
            println!("{c:?}");
        }
    }

    #[test]
    fn a_blank_card_is_no_device() {
        let root = std::env::temp_dir().join("tortos-installer-cards-blank");
        let _ = fs::remove_dir_all(&root);
        create_dir_all(&root).unwrap();
        let mut c = card();
        read_contents(&mut c, &root, false);
        assert_eq!(c.device, None);
        let _ = fs::remove_dir_all(&root);
    }
}
