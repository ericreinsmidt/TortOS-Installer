// A FAT32 card made here, for the Brick's fresh install: one partition over
// the whole card, formatted, with TortOS's files already in it, as one image
// the writer puts on the card the way it puts the Pixel's.
//
// The image ends at the last cluster the files use. The rest of the card is
// free space and the FATs say so, so it is never written; the FATs themselves
// are, whole (about 8 MB each on a 64 GB card).
//
// Laid out as the SD Association's formatter lays out a card: the partition
// starts at 4 MB, and the clusters start on a 4 MB boundary too, where the
// card's erase blocks do. 32 KB clusters from 4 GB up, 4 KB below.

use std::io::{self, Read};
use std::sync::Arc;

const SECTOR: u64 = 512;
/// The partition's start, and what the clusters are aligned to: 4 MB
const ALIGN: u64 = 8192;
const RESERVED: u64 = 32;
const ROOT_CLUSTER: u32 = 2;
const END_OF_CHAIN: u32 = 0x0FFF_FFFF;
const LABEL: &[u8; 11] = b"TORTOS     ";

/// One file or folder for the card, by its path inside it ("TortOS/menu.ttf").
/// `time` is a DOS date and time, date in the high 16 bits.
pub struct Entry {
    pub path: String,
    pub time: u32,
    /// None for a folder
    pub data: Option<Vec<u8>>,
}

/// The finished image: pieces of bytes and runs of zeros, end to end.
pub struct Image {
    pieces: Arc<Vec<Piece>>,
    pub len: u64,
    /// How full the card starts out; the tests look
    #[allow(dead_code)]
    pub clusters_used: u32,
    #[allow(dead_code)]
    pub clusters: u32,
}

enum Piece {
    Bytes(Vec<u8>),
    Zeros(u64),
}

impl Piece {
    fn len(&self) -> u64 {
        match self {
            Piece::Bytes(b) => b.len() as u64,
            Piece::Zeros(n) => *n,
        }
    }
}

impl Image {
    /// The image from its first byte, as the writer asks for it, twice
    pub fn open(&self) -> Box<dyn Read + Send> {
        Box::new(Reader { pieces: self.pieces.clone(), index: 0, offset: 0 })
    }
}

struct Reader {
    pieces: Arc<Vec<Piece>>,
    index: usize,
    offset: u64,
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while let Some(piece) = self.pieces.get(self.index) {
            let left = piece.len() - self.offset;
            if left == 0 {
                self.index += 1;
                self.offset = 0;
                continue;
            }
            let n = (buf.len() as u64).min(left) as usize;
            match piece {
                Piece::Bytes(b) => buf[..n].copy_from_slice(&b[self.offset as usize..self.offset as usize + n]),
                Piece::Zeros(_) => buf[..n].fill(0),
            }
            self.offset += n as u64;
            return Ok(n);
        }
        Ok(0)
    }
}

/// Where everything goes on a card of `card_bytes`, in sectors from the
/// card's start.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Layout {
    part_sectors: u64,
    cluster_sectors: u64,
    reserved: u64,
    fat_sectors: u64,
    clusters: u32,
}

impl Layout {
    fn new(card_bytes: u64) -> Result<Layout, String> {
        let card_sectors = card_bytes / SECTOR;
        if card_bytes < 1 << 30 {
            return Err("This card is too small for TortOS: it needs at least 1 GB.".into());
        }
        // The partition table counts sectors in 32 bits
        let part_sectors = (card_sectors - ALIGN).min(u32::MAX as u64);
        let cluster_sectors = if card_bytes < 4 << 30 { 8 } else { 64 };
        let mut fat_sectors = 1;
        loop {
            let pad = (ALIGN - (ALIGN + RESERVED + 2 * fat_sectors) % ALIGN) % ALIGN;
            let reserved = RESERVED + pad;
            let clusters = (part_sectors - reserved - 2 * fat_sectors) / cluster_sectors;
            let need = ((clusters + 2) * 4).div_ceil(SECTOR);
            if need <= fat_sectors {
                return Ok(Layout { part_sectors, cluster_sectors, reserved, fat_sectors, clusters: clusters as u32 });
            }
            fat_sectors = need;
        }
    }
    fn cluster_bytes(&self) -> u64 {
        self.cluster_sectors * SECTOR
    }
    /// The first cluster's sector, from the card's start
    fn data_start(&self) -> u64 {
        ALIGN + self.reserved + 2 * self.fat_sectors
    }
}

/// A folder or a file, before it has clusters.
enum Node {
    Dir { name: String, time: u32, children: Vec<Node> },
    File { name: String, time: u32, data: Vec<u8> },
}

impl Node {
    fn name(&self) -> &str {
        match self {
            Node::Dir { name, .. } | Node::File { name, .. } => name,
        }
    }
}

fn tree(entries: Vec<Entry>) -> Result<Vec<Node>, String> {
    let mut root = Vec::new();
    for entry in entries {
        let parts: Vec<&str> = entry.path.split('/').filter(|p| !p.is_empty()).collect();
        let Some((last, folders)) = parts.split_last() else { continue };
        let mut here = &mut root;
        for folder in folders {
            let at = match here.iter().position(|n: &Node| n.name() == *folder) {
                Some(i) => i,
                None => {
                    here.push(Node::Dir { name: folder.to_string(), time: entry.time, children: Vec::new() });
                    here.len() - 1
                }
            };
            here = match &mut here[at] {
                Node::Dir { children, .. } => children,
                Node::File { .. } => return Err(format!("{} is both a file and a folder", entry.path)),
            };
        }
        let exists = here.iter().any(|n| n.name() == *last);
        match entry.data {
            None if exists => {}
            None => here.push(Node::Dir { name: last.to_string(), time: entry.time, children: Vec::new() }),
            Some(_) if exists => return Err(format!("{} is in the release twice", entry.path)),
            Some(data) => {
                if data.len() as u64 > u32::MAX as u64 {
                    return Err(format!("{} is too big for FAT32", entry.path));
                }
                here.push(Node::File { name: last.to_string(), time: entry.time, data });
            }
        }
    }
    Ok(root)
}

/// The 8.3 name a long name is also known by: NAME~N.EXT, unique in its
/// folder. A name that already is one is kept as it is, with no long name.
fn short_name(name: &str, taken: &[[u8; 11]]) -> ([u8; 11], bool) {
    const OK: &[u8] = b"$%'-_@~`!(){}^#&";
    let valid = |c: u8| c.is_ascii_uppercase() || c.is_ascii_digit() || OK.contains(&c);
    let (base, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i + 1..]),
        _ => (name, ""),
    };
    if !base.is_empty() && base.len() <= 8 && ext.len() <= 3 && base.bytes().chain(ext.bytes()).all(valid) {
        let mut s = [b' '; 11];
        s[..base.len()].copy_from_slice(base.as_bytes());
        s[8..8 + ext.len()].copy_from_slice(ext.as_bytes());
        if !taken.contains(&s) {
            return (s, false);
        }
    }
    let clean = |part: &str, max: usize| -> Vec<u8> {
        part.chars()
            .filter(|&c| c != ' ' && c != '.')
            .map(|c| {
                let u = c.to_ascii_uppercase();
                if u.is_ascii() && valid(u as u8) { u as u8 } else { b'_' }
            })
            .take(max)
            .collect()
    };
    let base = clean(base.trim_start_matches('.'), 8);
    let ext = clean(ext, 3);
    for n in 1u32.. {
        let tail = format!("~{n}");
        let keep = base.len().min(8 - tail.len());
        let mut s = [b' '; 11];
        s[..keep].copy_from_slice(&base[..keep]);
        s[keep..keep + tail.len()].copy_from_slice(tail.as_bytes());
        s[8..8 + ext.len()].copy_from_slice(&ext);
        if !taken.contains(&s) {
            return (s, true);
        }
    }
    unreachable!()
}

fn checksum(short: &[u8; 11]) -> u8 {
    short.iter().fold(0u8, |sum, &b| (sum >> 1).wrapping_add(sum << 7).wrapping_add(b))
}

/// A folder entry pointing at `cluster`
fn dir_entry(short: &[u8; 11], attr: u8, time: u32, cluster: u32, size: u32) -> [u8; 32] {
    let mut e = [0u8; 32];
    e[..11].copy_from_slice(short);
    e[11] = attr;
    let (date, clock) = ((time >> 16) as u16, time as u16);
    e[14..16].copy_from_slice(&clock.to_le_bytes());
    e[16..18].copy_from_slice(&date.to_le_bytes());
    e[18..20].copy_from_slice(&date.to_le_bytes());
    e[20..22].copy_from_slice(&((cluster >> 16) as u16).to_le_bytes());
    e[22..24].copy_from_slice(&clock.to_le_bytes());
    e[24..26].copy_from_slice(&date.to_le_bytes());
    e[26..28].copy_from_slice(&(cluster as u16).to_le_bytes());
    e[28..32].copy_from_slice(&size.to_le_bytes());
    e
}

/// The long-name entries that go before a short one, last part first
fn long_entries(name: &str, short: &[u8; 11]) -> Result<Vec<[u8; 32]>, String> {
    let mut units: Vec<u16> = name.encode_utf16().collect();
    if units.len() > 255 {
        return Err(format!("{name}: the name is too long for FAT32"));
    }
    let parts = units.len().div_ceil(13);
    if units.len() % 13 != 0 {
        units.push(0);
    }
    units.resize(parts * 13, 0xFFFF);
    let sum = checksum(short);
    let mut out = Vec::new();
    for i in (0..parts).rev() {
        let mut e = [0u8; 32];
        e[0] = (i + 1) as u8 | if i + 1 == parts { 0x40 } else { 0 };
        e[11] = 0x0F;
        e[13] = sum;
        let chunk = &units[i * 13..i * 13 + 13];
        for (k, &u) in chunk.iter().enumerate() {
            let at = match k {
                0..=4 => 1 + 2 * k,
                5..=10 => 14 + 2 * (k - 5),
                _ => 28 + 2 * (k - 11),
            };
            e[at..at + 2].copy_from_slice(&u.to_le_bytes());
        }
        out.push(e);
    }
    Ok(out)
}

/// A folder's entries before its children have clusters: per child, its
/// long-name entries and its short name.
fn names(children: &[Node]) -> Result<Vec<(Vec<[u8; 32]>, [u8; 11])>, String> {
    let mut taken: Vec<[u8; 11]> = Vec::new();
    let mut out = Vec::new();
    for child in children {
        let (short, long) = short_name(child.name(), &taken);
        taken.push(short);
        let longs = if long { long_entries(child.name(), &short)? } else { Vec::new() };
        out.push((longs, short));
    }
    Ok(out)
}

/// Clusters, handed out in order from the first
struct Clusters {
    next: u32,
    fat: Vec<u32>,
    data: Vec<u8>,
    cluster_bytes: u64,
    limit: u32,
}

impl Clusters {
    /// `bytes` worth of clusters in a row, chained; the first one's number
    fn take(&mut self, bytes: u64) -> Result<u32, String> {
        let count = bytes.div_ceil(self.cluster_bytes).max(1) as u32;
        let first = self.next;
        if (first - 2) as u64 + count as u64 > self.limit as u64 {
            return Err("TortOS doesn't fit on this card.".into());
        }
        for c in first..first + count {
            self.fat.push(if c + 1 == first + count { END_OF_CHAIN } else { c + 1 });
        }
        self.next += count;
        self.data.resize(((self.next - 2) as u64 * self.cluster_bytes) as usize, 0);
        Ok(first)
    }
    fn put(&mut self, cluster: u32, at: usize, bytes: &[u8]) {
        let start = (cluster - 2) as usize * self.cluster_bytes as usize + at;
        self.data[start..start + bytes.len()].copy_from_slice(bytes);
    }
}

/// Lays out a folder and everything in it, its own clusters first. `parent`
/// is None for the root, else its parent's first cluster (0 for the root).
fn place(
    clusters: &mut Clusters,
    children: &[Node],
    parent: Option<u32>,
    time: u32,
) -> Result<u32, String> {
    let entries = names(children)?;
    let count = entries.iter().map(|(l, _)| l.len() + 1).sum::<usize>() + if parent.is_some() { 2 } else { 1 };
    let first = clusters.take(count as u64 * 32)?;
    let mut at = 0;
    let mut put = |clusters: &mut Clusters, e: &[u8; 32]| {
        clusters.put(first, at, e);
        at += 32;
    };
    match parent {
        // The root: its label
        None => put(clusters, &dir_entry(LABEL, 0x08, time, 0, 0)),
        Some(parent) => {
            put(clusters, &dir_entry(b".          ", 0x10, time, first, 0));
            put(clusters, &dir_entry(b"..         ", 0x10, time, parent, 0));
        }
    }
    for (child, (longs, short)) in children.iter().zip(&entries) {
        let entry = match child {
            Node::File { time, data, .. } => {
                let cluster = if data.is_empty() { 0 } else { clusters.take(data.len() as u64)? };
                if cluster != 0 {
                    clusters.put(cluster, 0, data);
                }
                dir_entry(short, 0x20, *time, cluster, data.len() as u32)
            }
            Node::Dir { time, children, .. } => {
                // ".." in a folder at the top points at 0, not the root's cluster
                let me = if parent.is_some() { first } else { 0 };
                let cluster = place(clusters, children, Some(me), *time)?;
                dir_entry(short, 0x10, *time, cluster, 0)
            }
        };
        for l in longs {
            put(clusters, l);
        }
        put(clusters, &entry);
    }
    Ok(first)
}

fn chs(lba: u64) -> [u8; 3] {
    let (c, h, s) = if lba >= 1024 * 255 * 63 {
        (1023, 254, 63)
    } else {
        (lba / (255 * 63), (lba / 63) % 255, lba % 63 + 1)
    };
    [h as u8, ((s as u8) & 0x3F) | (((c >> 8) as u8) << 6), c as u8]
}

/// The whole card: its table, the file system, and `entries` in it. `time`
/// is the folders' DOS time where the release gives none, and `id` the
/// volume's serial number.
pub fn build(card_bytes: u64, entries: Vec<Entry>, time: u32, id: u32) -> Result<Image, String> {
    let layout = Layout::new(card_bytes)?;
    let nodes = tree(entries)?;
    let mut clusters = Clusters {
        next: ROOT_CLUSTER,
        fat: vec![0x0FFF_FFF8, END_OF_CHAIN],
        data: Vec::new(),
        cluster_bytes: layout.cluster_bytes(),
        limit: layout.clusters,
    };
    place(&mut clusters, &nodes, None, time)?;
    let used = clusters.next - 2;

    // The table: one FAT32 partition from 4 MB to the end
    let mut mbr = vec![0u8; SECTOR as usize];
    mbr[440..444].copy_from_slice(&id.rotate_left(16).to_le_bytes());
    let p = &mut mbr[446..462];
    p[1..4].copy_from_slice(&chs(ALIGN));
    p[4] = 0x0C;
    p[5..8].copy_from_slice(&chs(ALIGN + layout.part_sectors - 1));
    p[8..12].copy_from_slice(&(ALIGN as u32).to_le_bytes());
    p[12..16].copy_from_slice(&(layout.part_sectors as u32).to_le_bytes());
    mbr[510] = 0x55;
    mbr[511] = 0xAA;

    // The reserved sectors: the boot sector, FSInfo, and copies of both at 6
    let mut reserved = vec![0u8; (layout.reserved * SECTOR) as usize];
    let b = &mut reserved[..512];
    b[..3].copy_from_slice(&[0xEB, 0x58, 0x90]);
    b[3..11].copy_from_slice(b"MSWIN4.1");
    b[11..13].copy_from_slice(&(SECTOR as u16).to_le_bytes());
    b[13] = layout.cluster_sectors as u8;
    b[14..16].copy_from_slice(&(layout.reserved as u16).to_le_bytes());
    b[16] = 2;
    b[21] = 0xF8;
    b[24..26].copy_from_slice(&63u16.to_le_bytes());
    b[26..28].copy_from_slice(&255u16.to_le_bytes());
    b[28..32].copy_from_slice(&(ALIGN as u32).to_le_bytes());
    b[32..36].copy_from_slice(&(layout.part_sectors as u32).to_le_bytes());
    b[36..40].copy_from_slice(&(layout.fat_sectors as u32).to_le_bytes());
    b[44..48].copy_from_slice(&ROOT_CLUSTER.to_le_bytes());
    b[48..50].copy_from_slice(&1u16.to_le_bytes());
    b[50..52].copy_from_slice(&6u16.to_le_bytes());
    b[64] = 0x80;
    b[66] = 0x29;
    b[67..71].copy_from_slice(&id.to_le_bytes());
    b[71..82].copy_from_slice(LABEL);
    b[82..90].copy_from_slice(b"FAT32   ");
    b[510] = 0x55;
    b[511] = 0xAA;
    let info = &mut reserved[512..1024];
    info[..4].copy_from_slice(&0x4161_5252u32.to_le_bytes());
    info[484..488].copy_from_slice(&0x6141_7272u32.to_le_bytes());
    info[488..492].copy_from_slice(&(layout.clusters - used).to_le_bytes());
    info[492..496].copy_from_slice(&clusters.next.to_le_bytes());
    info[508..512].copy_from_slice(&0xAA55_0000u32.to_le_bytes());
    reserved[1024 + 510] = 0x55;
    reserved[1024 + 511] = 0xAA;
    let (boot, copy) = reserved.split_at_mut(6 * 512);
    copy[..3 * 512].copy_from_slice(&boot[..3 * 512]);

    let fat_used: Vec<u8> = clusters.fat.iter().flat_map(|c| c.to_le_bytes()).collect();
    let fat_rest = layout.fat_sectors * SECTOR - fat_used.len() as u64;

    let pieces = vec![
        Piece::Bytes(mbr),
        Piece::Zeros((ALIGN - 1) * SECTOR),
        Piece::Bytes(reserved),
        Piece::Bytes(fat_used.clone()),
        Piece::Zeros(fat_rest),
        Piece::Bytes(fat_used),
        Piece::Zeros(fat_rest),
        Piece::Bytes(clusters.data),
    ];
    let len = pieces.iter().map(Piece::len).sum();
    debug_assert_eq!(len, layout.data_start() * SECTOR + used as u64 * layout.cluster_bytes());
    Ok(Image { pieces: Arc::new(pieces), len, clusters_used: used, clusters: layout.clusters })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIME: u32 = (((2026 - 1980) << 9 | 10 << 5 | 7) << 16) | (12 << 11);

    fn file(path: &str, data: &[u8]) -> Entry {
        Entry { path: path.into(), time: TIME, data: Some(data.to_vec()) }
    }

    fn bytes(image: &Image) -> Vec<u8> {
        let mut out = Vec::new();
        image.open().read_to_end(&mut out).unwrap();
        out
    }

    #[test]
    fn lays_out_cards_of_each_size() {
        let small = Layout::new(2 << 30).unwrap();
        assert_eq!(small.cluster_sectors, 8);
        let card = Layout::new(62_723_719_168).unwrap();
        assert_eq!(card.cluster_sectors, 64);
        for l in [small, card, Layout::new(1 << 40).unwrap()] {
            assert_eq!(l.data_start() % ALIGN, 0, "{l:?}");
            assert!(l.clusters >= 65525, "{l:?}");
            assert!((l.clusters as u64 + 2) * 4 <= l.fat_sectors * SECTOR, "{l:?}");
            assert!(l.reserved >= RESERVED && l.reserved <= u16::MAX as u64);
            assert!(l.reserved + 2 * l.fat_sectors + l.clusters as u64 * l.cluster_sectors <= l.part_sectors);
        }
        assert!(Layout::new(512 << 20).is_err());
    }

    #[test]
    fn short_names() {
        let s = |n: &str, taken: &[[u8; 11]]| {
            let (s, long) = short_name(n, taken);
            (String::from_utf8(s.to_vec()).unwrap(), long)
        };
        assert_eq!(s("MAINUI", &[]), ("MAINUI     ".into(), false));
        assert_eq!(s("SNES.PNG", &[]), ("SNES    PNG".into(), false));
        assert_eq!(s("MainUI", &[]), ("MAINUI~1   ".into(), true));
        assert_eq!(s(".tmp_update", &[]), ("TMP_UP~1   ".into(), true));
        assert_eq!(s("Game Boy Color", &[]), ("GAMEBO~1   ".into(), true));
        assert_eq!(s("tortos-boot.mp4", &[]), ("TORTOS~1MP4".into(), true));
        assert_eq!(s("mednafen_pce_fast_libretro.so", &[*b"MEDNAF~1SO "]), ("MEDNAF~2SO ".into(), true));
        assert_eq!(s(".media", &[]), ("MEDIA~1    ".into(), true));
    }

    #[test]
    fn long_names_spell_the_name() {
        let short = *b"GAMEBO~1   ";
        let e = long_entries("Game Boy Color", &short).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0][0], 0x42);
        assert_eq!(e[1][0], 0x01);
        assert!(e.iter().all(|x| x[11] == 0x0F && x[13] == checksum(&short)));
        // "Game " in the first entry's first five characters
        let first: Vec<u16> = (0..5).map(|k| u16::from_le_bytes([e[1][1 + 2 * k], e[1][2 + 2 * k]])).collect();
        assert_eq!(String::from_utf16(&first).unwrap(), "Game ");
    }

    #[test]
    fn builds_a_card() {
        let big = vec![7u8; 70_000];
        let image = build(
            4 << 30,
            vec![
                Entry { path: "Roms/".into(), time: TIME, data: None },
                Entry { path: "Roms/Game Boy/.media/".into(), time: TIME, data: None },
                file("TortOS/launch.sh", b"#!/bin/sh\n"),
                file("TortOS/cores/big.so", &big),
                file("TortOS/empty", b""),
                file(".tmp_update/tg3040.sh", b"sh\n"),
            ],
            TIME,
            0x1234_5678,
        )
        .unwrap();
        let card = bytes(&image);
        assert_eq!(card.len() as u64, image.len);
        assert_eq!(&card[510..512], &[0x55, 0xAA]);
        assert_eq!(card[446 + 4], 0x0C);
        let part = ALIGN as usize * 512;
        assert_eq!(&card[part + 82..part + 90], b"FAT32   ");
        assert_eq!(&card[part..part + 512], &card[part + 6 * 512..part + 7 * 512]);
        // Root, Roms, Game Boy, .media, TortOS, launch.sh, cores, big.so (3
        // clusters of 32 KB), .tmp_update, tg3040.sh; the empty file has none
        assert_eq!(image.clusters_used, 12);
        let layout = Layout::new(4 << 30).unwrap();
        let fat = (ALIGN + layout.reserved) as usize * 512;
        let entry = |c: usize| u32::from_le_bytes(card[fat + 4 * c..fat + 4 * c + 4].try_into().unwrap());
        assert_eq!(entry(0), 0x0FFF_FFF8);
        assert_eq!(entry(2), END_OF_CHAIN);
        // big.so's chain runs through three clusters in a row
        let data = layout.data_start() as usize * 512;
        let at = card[data..].windows(big.len()).position(|w| w == &big[..]).unwrap();
        let first = (at / layout.cluster_bytes() as usize + 2) as usize;
        assert_eq!((entry(first), entry(first + 1), entry(first + 2)), (first as u32 + 1, first as u32 + 2, END_OF_CHAIN));
        // The second FAT is the first's copy
        let second = fat + layout.fat_sectors as usize * 512;
        assert_eq!(&card[fat..fat + 64], &card[second..second + 64]);
        // The label comes first in the root
        assert_eq!(&card[data..data + 11], LABEL);
    }

    #[test]
    fn refuses_what_doesnt_fit() {
        let a = vec![file("a", b"1"), file("a", b"2")];
        assert!(build(2 << 30, a, TIME, 1).is_err());
    }
}
