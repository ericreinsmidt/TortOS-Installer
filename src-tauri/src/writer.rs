// Writing a card image to a card: the whole image for a fresh install, or
// only its system part for an update. The GKD Pixel 2's image has an MBR, the
// Brick's a GPT: sector 0 holds the GPT's "protective" MBR entry, and its
// header and partition list follow in sectors 1 to 33.
//
// AN UPDATE writes the image from `skip` (the bootloader: 32 KB on the Pixel,
// 128 KB on the Brick) to the end of the root, partition `through`, and
// nothing past it: the Brick's state partition after the root (its Bluetooth
// pairings) and the games partition the first boot added over the rest of the
// card stay as they are. That is only safe while the card's partitions are
// where the image's are, so the card's own table is checked first, and
// nothing is written when they differ. Its table is never written either, so
// an update cut short is finished by running it again.
//
// A FRESH INSTALL writes the whole image, the partition table LAST. The
// first thing written is a blank table, so the computer sees no partitions to
// mount while the rest goes on, and a card that is pulled halfway has no
// table at all rather than half a system that looks whole. On the Brick the
// GPT is fitted to the card as it goes on: as the image has it, it describes a
// disk the image's size (its second copy at the image's end, its protective
// MBR covering only the image), which Windows "repairs" when the card goes in,
// leaving a primary header that points its partition list into the boot
// loader (seen 2026-10-24). So the protective MBR covers the card, the headers
// name its last sector, and the second copy goes at the card's end.
//
// Both read everything back against a second pass of the image: no checksum
// to trust, and a mismatch says where it is. Everything here works on any
// Read + Write + Seek, so the tests run it on files.

use std::io::{self, Read, Seek, SeekFrom, Write};

pub const SECTOR: u64 = 512;

/// Written and read this much at a time: a whole number of sectors, as raw
/// disks on Windows and macOS insist.
const CHUNK: usize = 4 << 20;

/// A GPT's own sectors at the start of the disk: the protective MBR, the
/// header, and 128 partition entries of 128 bytes, as genimage writes them.
/// Read in full from card and image; an MBR's table is only the first.
pub const GPT_SECTORS: u64 = 34;
const HEAD: usize = (GPT_SECTORS * SECTOR) as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Partition {
    pub kind: u8,
    /// In sectors, as the table has them
    pub start: u64,
    pub sectors: u64,
}

impl Partition {
    fn end(&self) -> u64 {
        self.start + self.sectors
    }
}

/// An MBR's four primary entries, None where one is empty.
pub fn partitions(sector0: &[u8]) -> Option<[Option<Partition>; 4]> {
    if sector0.len() < 512 || sector0[510..512] != [0x55, 0xAA] {
        return None;
    }
    let mut out = [None; 4];
    for (i, slot) in out.iter_mut().enumerate() {
        let e = &sector0[446 + 16 * i..462 + 16 * i];
        let start = u64::from(u32::from_le_bytes([e[8], e[9], e[10], e[11]]));
        let sectors = u64::from(u32::from_le_bytes([e[12], e[13], e[14], e[15]]));
        if e[4] != 0 && sectors != 0 {
            *slot = Some(Partition { kind: e[4], start, sectors });
        }
    }
    Some(out)
}

/// Whether sector 0 is a GPT's protective MBR: one entry of type 0xEE.
fn is_gpt(sector0: &[u8]) -> bool {
    partitions(sector0).is_some_and(|p| p.iter().flatten().any(|e| e.kind == 0xEE))
}

/// A GPT's partitions, from the disk's first sectors (HEAD bytes), in their
/// slots, None where one is empty; None if there is no GPT header. `kind` is
/// 0 for all of them: what an update checks is where they are.
pub fn gpt_partitions(head: &[u8]) -> Option<Vec<Option<Partition>>> {
    let header = head.get(512..604)?;
    if &header[..8] != b"EFI PART" {
        return None;
    }
    let u32_at = |o: usize| u32::from_le_bytes(header[o..o + 4].try_into().unwrap());
    let lba = u64::from_le_bytes(header[72..80].try_into().unwrap());
    let (count, size) = (u32_at(80) as usize, u32_at(84) as usize);
    if size < 128 || count > 128 {
        return None;
    }
    let mut out = Vec::new();
    for i in 0..count {
        let at = usize::try_from(lba * SECTOR).ok()? + i * size;
        let e = head.get(at..at + 128)?;
        let first = u64::from_le_bytes(e[32..40].try_into().unwrap());
        let last = u64::from_le_bytes(e[40..48].try_into().unwrap());
        out.push((e[..16] != [0u8; 16] && last >= first).then(|| Partition { kind: 0, start: first, sectors: last - first + 1 }));
    }
    while out.last() == Some(&None) {
        out.pop();
    }
    Some(out)
}

/// A disk's partitions from its first sectors, whichever kind of table.
fn table_of(head: &[u8]) -> Option<Vec<Option<Partition>>> {
    if is_gpt(head) {
        gpt_partitions(head)
    } else {
        partitions(head).map(|p| p.to_vec())
    }
}

/// CRC-32 as the GPT uses it (IEEE, reflected), for its headers and list.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// A GPT image's first sectors fitted to a card of `card_bytes`, and the
/// second copy for the card's end: the partition list, then the header, from
/// the returned offset. Also where the image's own second header was, to clear.
fn fit_gpt(head: &[u8], card_bytes: u64) -> Result<(Vec<u8>, u64, Vec<u8>, u64), String> {
    let u64_at = |h: &[u8], o: usize| u64::from_le_bytes(h[o..o + 8].try_into().unwrap());
    let u32_at = |h: &[u8], o: usize| u32::from_le_bytes(h[o..o + 4].try_into().unwrap());
    let mut head = head.to_vec();
    let size = u32_at(&head, 512 + 12) as usize;
    let list_at = u64_at(&head, 512 + 72);
    let list_len = u32_at(&head, 512 + 80) as usize * u32_at(&head, 512 + 84) as usize;
    if size < 92 || size > 512 || list_at != 2 || 1024 + list_len > head.len() {
        return Err("The image's partition table isn't laid out as expected".into());
    }
    let list = head[1024..1024 + list_len].to_vec();
    let list_sectors = (list_len as u64).div_ceil(SECTOR);
    let last = card_bytes / SECTOR - 1;
    let old_backup = u64_at(&head, 512 + 32);
    let ends = gpt_partitions(&head).unwrap_or_default().into_iter().flatten().map(|p| p.end()).max().unwrap_or(0);
    if ends > last - list_sectors {
        return Err("This card is too small for TortOS.".into());
    }
    // Protective MBR: one 0xEE entry from sector 1 over the whole card
    let cover = (last).min(0xFFFF_FFFF) as u32;
    head[446 + 12..446 + 16].copy_from_slice(&cover.to_le_bytes());
    let mut header = head[512..512 + size].to_vec();
    header[32..40].copy_from_slice(&last.to_le_bytes());
    header[48..56].copy_from_slice(&(last - list_sectors - 1).to_le_bytes());
    let sum = |h: &mut Vec<u8>| {
        h[16..20].fill(0);
        let c = crc32(h);
        h[16..20].copy_from_slice(&c.to_le_bytes());
    };
    let mut backup = header.clone();
    sum(&mut header);
    head[512..512 + size].copy_from_slice(&header);
    backup[24..32].copy_from_slice(&last.to_le_bytes());
    backup[32..40].copy_from_slice(&1u64.to_le_bytes());
    backup[72..80].copy_from_slice(&(last - list_sectors).to_le_bytes());
    sum(&mut backup);
    let mut tail = list;
    tail.resize((list_sectors * SECTOR) as usize, 0);
    tail.extend_from_slice(&backup);
    tail.resize(((list_sectors + 1) * SECTOR) as usize, 0);
    Ok((head, (last - list_sectors) * SECTOR, tail, old_backup * SECTOR))
}

/// Bytes written at `at`, flushed, and read back.
fn put_checked(card: &mut dyn Card, at: u64, data: &[u8]) -> Result<(), String> {
    card.seek(SeekFrom::Start(at)).map_err(|e| format!("Seeking on the card: {e}"))?;
    card.write_all(data).map_err(|e| format!("Writing the card's table: {e}"))?;
    card.sync().map_err(|e| format!("Flushing the card: {e}"))?;
    card.drop_cache().map_err(|e| format!("Clearing the card's cache: {e}"))?;
    card.seek(SeekFrom::Start(at)).map_err(|e| format!("Seeking on the card: {e}"))?;
    let mut back = vec![0u8; data.len()];
    card.read_exact(&mut back).map_err(|e| format!("Reading a partition table: {e}"))?;
    if back != data {
        return Err("The card didn't read back its new partition table. The card may be failing: \
                    try again, or another card."
            .into());
    }
    Ok(())
}

/// The bytes an update writes, from..to, the same offsets in image and card.
#[derive(Debug, PartialEq, Eq)]
pub struct Span {
    pub from: u64,
    pub to: u64,
}

/// Why a card can't be updated in place: shown as it is, so it says what to
/// do as well.
fn layout_differs(why: &str) -> String {
    format!(
        "This card's partitions aren't where this version puts them ({why}), so it can't be \
         updated in place. A fresh install will work, but it erases the card."
    )
}

/// What an update of this card from this image would write, or why it can't:
/// `image_head` and `card_head` are each disk's first sectors (an MBR's
/// sector 0 is enough), and the update writes from `skip` to the end of the
/// image's partition `through` (counted from 1).
pub fn plan_update(image_head: &[u8], image_len: u64, card_head: &[u8], skip: u64, through: usize) -> Result<Span, String> {
    let gpt = is_gpt(image_head);
    let image = table_of(image_head).ok_or("The image has no partition table")?;
    if gpt != is_gpt(card_head) {
        return Err(layout_differs("the card's partition table is another kind"));
    }
    let card = table_of(card_head).ok_or_else(|| layout_differs("the card has no partition table"))?;
    let ours: Vec<(usize, Partition)> =
        image.iter().enumerate().filter_map(|(i, p)| p.map(|p| (i, p))).collect();
    let first = ours.iter().map(|(_, p)| p.start).min().ok_or("The image has no partitions")? * SECTOR;
    let end = ours.iter().map(|(_, p)| p.end()).max().unwrap_or(0) * SECTOR;
    let to = match image.get(through.wrapping_sub(1)) {
        Some(Some(p)) => p.end() * SECTOR,
        _ => return Err(format!("The image has no partition {through}")),
    };
    let table = if gpt { GPT_SECTORS * SECTOR } else { SECTOR };
    if skip < table || skip % SECTOR != 0 || skip > first {
        return Err(format!("An update can't start at byte {skip} of this image"));
    }
    if image_len < to {
        return Err(format!("The image ({image_len} bytes) is shorter than its partitions ({to})"));
    }
    // The image's partitions exactly where the card has them, the ones the
    // update leaves too...
    for &(i, p) in &ours {
        match card.get(i).copied().flatten() {
            Some(c) if c == p => {}
            Some(c) => {
                return Err(layout_differs(&format!(
                    "partition {}: sector {} for {} on the card, {} for {} in the image",
                    i + 1, c.start, c.sectors, p.start, p.sectors
                )))
            }
            None => return Err(layout_differs(&format!("the card has no partition {}", i + 1))),
        }
    }
    // ...and the card's others, the games partition, wholly past them
    for (i, c) in card.iter().enumerate() {
        if let Some(c) = c {
            if image.get(i).copied().flatten().is_none() && c.start * SECTOR < end {
                return Err(layout_differs(&format!(
                    "the card's partition {} starts at sector {}, inside the system",
                    i + 1, c.start
                )));
            }
        }
    }
    Ok(Span { from: skip, to })
}

/// A card opened for writing, as each system opens it (device.rs).
pub trait Card: Read + Write + Seek {
    /// What was written is on the card, not in a cache.
    fn sync(&mut self) -> io::Result<()>;
    /// What is read next comes from the card, not memory: a read-back of the
    /// cache proves nothing.
    fn drop_cache(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Progress {
    /// The release coming from GitHub, before anything is written
    Downloading { done: u64, total: u64 },
    Writing { done: u64, total: u64 },
    Checking { done: u64, total: u64 },
    /// A backup made or put back, each file read back as it goes
    Copying { done: u64, total: u64 },
}

/// The image, from its first byte, as many times as asked.
pub type OpenImage<'a> = &'a dyn Fn() -> io::Result<Box<dyn Read + Send>>;

fn skip_bytes(r: &mut dyn Read, n: u64) -> Result<(), String> {
    let got = io::copy(&mut r.take(n), &mut io::sink()).map_err(|e| format!("Reading the image: {e}"))?;
    if got < n {
        return Err("The image ended early".into());
    }
    Ok(())
}

/// The image's bytes from..to onto the card at the same offsets, then read
/// back and compared. `cancelled` is asked between chunks.
fn write_span(
    card: &mut dyn Card,
    open: OpenImage,
    from: u64,
    to: u64,
    progress: &mut dyn FnMut(Progress),
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    let total = to - from;
    let mut buf = vec![0u8; CHUNK];
    let mut image = open().map_err(|e| format!("Opening the image: {e}"))?;
    skip_bytes(&mut image, from)?;
    progress(Progress::Writing { done: 0, total });
    card.seek(SeekFrom::Start(from)).map_err(|e| format!("Seeking on the card: {e}"))?;
    let mut at = from;
    while at < to {
        if cancelled() {
            return Err("cancelled".into());
        }
        let n = CHUNK.min((to - at) as usize);
        image.read_exact(&mut buf[..n]).map_err(|e| format!("Reading the image at byte {at}: {e}"))?;
        card.write_all(&buf[..n]).map_err(|e| format!("Writing the card at byte {at}: {e}"))?;
        at += n as u64;
        progress(Progress::Writing { done: at - from, total });
    }
    card.sync().map_err(|e| format!("Flushing the card: {e}"))?;
    card.drop_cache().map_err(|e| format!("Clearing the card's cache: {e}"))?;
    drop(image);

    let mut image = open().map_err(|e| format!("Opening the image: {e}"))?;
    let mut back = vec![0u8; CHUNK];
    skip_bytes(&mut image, from)?;
    card.seek(SeekFrom::Start(from)).map_err(|e| format!("Seeking on the card: {e}"))?;
    let mut at = from;
    while at < to {
        if cancelled() {
            return Err("cancelled".into());
        }
        let n = CHUNK.min((to - at) as usize);
        image.read_exact(&mut buf[..n]).map_err(|e| format!("Reading the image at byte {at}: {e}"))?;
        card.read_exact(&mut back[..n]).map_err(|e| format!("Reading the card back at byte {at}: {e}"))?;
        if let Some(i) = buf[..n].iter().zip(&back[..n]).position(|(a, b)| a != b) {
            return Err(format!(
                "The card didn't read back what was written, at byte {}. The card may be failing: \
                 try again, or another card.",
                at + i as u64
            ));
        }
        at += n as u64;
        progress(Progress::Checking { done: at - from, total });
    }
    Ok(())
}

/// A disk's first sectors: enough for either kind of table.
fn read_head(r: &mut dyn Read) -> Result<Vec<u8>, String> {
    let mut s = vec![0u8; HEAD];
    r.read_exact(&mut s).map_err(|e| format!("Reading a partition table: {e}"))?;
    Ok(s)
}

/// An update: the system part only, from `skip` to the end of partition
/// `through`, after checking the card can take it.
pub fn update(
    card: &mut dyn Card,
    open: OpenImage,
    image_len: u64,
    skip: u64,
    through: usize,
    progress: &mut dyn FnMut(Progress),
    cancelled: &dyn Fn() -> bool,
) -> Result<Span, String> {
    card.seek(SeekFrom::Start(0)).map_err(|e| format!("Seeking on the card: {e}"))?;
    let card_head = read_head(card)?;
    let image_head = read_head(&mut *open().map_err(|e| format!("Opening the image: {e}"))?)?;
    let span = plan_update(&image_head, image_len, &card_head, skip, through)?;
    write_span(card, open, span.from, span.to, progress, cancelled)?;
    Ok(span)
}

/// A fresh install: the whole image, a blank table first and the real one
/// last (an MBR's sector, or a GPT's 34, fitted to the card's `card_bytes`).
/// `image_len` must be a whole number of sectors.
pub fn fresh(
    card: &mut dyn Card,
    open: OpenImage,
    image_len: u64,
    card_bytes: u64,
    progress: &mut dyn FnMut(Progress),
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if image_len < HEAD as u64 || image_len % SECTOR != 0 {
        return Err(format!("The image ({image_len} bytes) isn't a whole number of sectors"));
    }
    let mut head = read_head(&mut *open().map_err(|e| format!("Opening the image: {e}"))?)?;
    table_of(&head).ok_or("The image has no partition table")?;
    let gpt = is_gpt(&head);
    let table = if gpt { HEAD } else { SECTOR as usize };
    head.truncate(table);
    if card_bytes < image_len {
        return Err("This card is too small for TortOS.".into());
    }
    let fitted = if gpt { Some(fit_gpt(&head, card_bytes)?) } else { None };
    // From here the card is changed: the screen hears so first
    progress(Progress::Writing { done: 0, total: image_len - table as u64 });
    card.seek(SeekFrom::Start(0)).map_err(|e| format!("Seeking on the card: {e}"))?;
    card.write_all(&vec![0u8; table]).map_err(|e| format!("Clearing the card's table: {e}"))?;
    card.sync().map_err(|e| format!("Flushing the card: {e}"))?;
    write_span(card, open, table as u64, image_len, progress, cancelled)?;
    if let Some((fitted_head, tail_at, tail, old_backup)) = fitted {
        // The image's own second header goes, the card's is written at its
        // end, and the primary last
        if old_backup + SECTOR <= image_len && old_backup < tail_at {
            put_checked(card, old_backup, &[0u8; SECTOR as usize])?;
        }
        put_checked(card, tail_at, &tail)?;
        head = fitted_head;
    }
    put_checked(card, 0, &head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // A small card in the Pixel's shape, in sectors: the table, a gap, the
    // bootloader from 4 KB, boot and root, then the games partition
    const SKIP: u64 = 4096;
    const BOOT: Partition = Partition { kind: 0x0C, start: 16, sectors: 32 };
    const ROOT: Partition = Partition { kind: 0x83, start: 48, sectors: 80 };
    const GAMES: Partition = Partition { kind: 0x07, start: 128, sectors: 128 };
    const SYSTEM_END: usize = 128 * 512;
    const CARD_LEN: usize = 256 * 512;

    fn table(parts: &[Option<Partition>]) -> [u8; 512] {
        let mut s = [0u8; 512];
        for (i, p) in parts.iter().enumerate() {
            if let Some(p) = p {
                let e = &mut s[446 + 16 * i..462 + 16 * i];
                e[4] = p.kind;
                e[8..12].copy_from_slice(&(p.start as u32).to_le_bytes());
                e[12..16].copy_from_slice(&(p.sectors as u32).to_le_bytes());
            }
        }
        s[510] = 0x55;
        s[511] = 0xAA;
        s
    }

    /// An image whose bytes all differ from another seed's.
    fn image(parts: &[Option<Partition>], seed: u8) -> Vec<u8> {
        let end = parts.iter().flatten().map(|p| p.end()).max().unwrap() as usize * 512;
        let mut v: Vec<u8> = (0..end).map(|i| (i as u8).wrapping_mul(7).wrapping_add(seed)).collect();
        v[..512].copy_from_slice(&table(parts));
        v[512..SKIP as usize].fill(0);
        v
    }

    /// A card as the Pixel leaves it after its first boot.
    fn booted(old: &[u8]) -> Vec<u8> {
        let mut c = vec![0xEE; CARD_LEN];
        c[..old.len()].copy_from_slice(old);
        c[..512].copy_from_slice(&table(&[Some(BOOT), Some(ROOT), Some(GAMES)]));
        c[1000] = 0x42; // in the gap before the bootloader
        c
    }

    struct FileCard {
        data: Cursor<Vec<u8>>,
        /// Flips this byte as it is written: a card that lies
        corrupt_at: Option<u64>,
        /// Fails writes past this offset: a card pulled out
        fail_after: Option<u64>,
    }
    impl Read for FileCard {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> { self.data.read(b) }
    }
    impl Write for FileCard {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            let at = self.data.position();
            // Pulled out: nothing past the cutoff, even partway through a write
            let b = match self.fail_after {
                Some(f) if at >= f => return Err(io::Error::other("pulled out")),
                Some(f) => &b[..b.len().min((f - at) as usize)],
                None => b,
            };
            let n = self.data.write(b)?;
            if let Some(bad) = self.corrupt_at.filter(|&x| x >= at && x < at + n as u64) {
                self.data.get_mut()[bad as usize] ^= 0xFF;
            }
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> { Ok(()) }
    }
    impl Seek for FileCard {
        fn seek(&mut self, p: SeekFrom) -> io::Result<u64> { self.data.seek(p) }
    }
    impl Card for FileCard {
        fn sync(&mut self) -> io::Result<()> { Ok(()) }
    }

    fn card(data: Vec<u8>) -> FileCard {
        FileCard { data: Cursor::new(data), corrupt_at: None, fail_after: None }
    }

    fn opener(img: &[u8]) -> impl Fn() -> io::Result<Box<dyn Read + Send>> {
        let owned = img.to_vec();
        move || Ok(Box::new(Cursor::new(owned.clone())) as Box<dyn Read + Send>)
    }

    fn run_update(c: &mut FileCard, img: &[u8]) -> Result<Span, String> {
        update(c, &opener(img), img.len() as u64, SKIP, 2, &mut |_| {}, &|| false)
    }

    #[test]
    fn an_update_writes_the_system_and_nothing_else() {
        let parts = [Some(BOOT), Some(ROOT)];
        let before = booted(&image(&parts, 1));
        let new = image(&parts, 2);
        let mut c = card(before.clone());
        assert_eq!(run_update(&mut c, &new).unwrap(), Span { from: SKIP, to: SYSTEM_END as u64 });
        let after = c.data.into_inner();
        assert_eq!(after[..SKIP as usize], before[..SKIP as usize], "table and gap untouched");
        assert_eq!(after[SKIP as usize..SYSTEM_END], new[SKIP as usize..], "system written");
        assert_eq!(after[SYSTEM_END..], before[SYSTEM_END..], "games untouched");
    }

    #[test]
    fn a_card_never_booted_updates_too() {
        let parts = [Some(BOOT), Some(ROOT)];
        let mut c = card(image(&parts, 1));
        let new = image(&parts, 2);
        run_update(&mut c, &new).unwrap();
        assert_eq!(c.data.into_inner()[SKIP as usize..], new[SKIP as usize..]);
    }

    fn refused(before: Vec<u8>, img: &[u8], says: &str) {
        let mut c = card(before.clone());
        let err = run_update(&mut c, img).unwrap_err();
        assert!(err.contains(says), "{err:?} should say {says:?}");
        assert!(c.data.into_inner() == before, "a refusal wrote to the card");
    }

    #[test]
    fn a_bigger_root_partition_is_refused() {
        refused(booted(&image(&[Some(BOOT), Some(ROOT)], 1)),
                &image(&[Some(BOOT), Some(Partition { sectors: 96, ..ROOT })], 2), "partition 2");
    }

    #[test]
    fn a_games_partition_inside_the_system_is_refused() {
        let mut before = booted(&image(&[Some(BOOT), Some(ROOT)], 1));
        before[..512].copy_from_slice(&table(&[Some(BOOT), Some(ROOT), Some(Partition { start: 100, ..GAMES })]));
        refused(before, &image(&[Some(BOOT), Some(ROOT)], 2), "inside the system");
    }

    #[test]
    fn a_card_from_something_else_is_refused() {
        let mut other = vec![0x11; CARD_LEN];
        other[..512].copy_from_slice(&table(&[Some(Partition { kind: 0x0C, start: 2048, sectors: 4096 })]));
        refused(other, &image(&[Some(BOOT), Some(ROOT)], 2), "partition 1");
        refused(vec![0; CARD_LEN], &image(&[Some(BOOT), Some(ROOT)], 2), "no partition table");
    }

    #[test]
    fn a_card_that_reads_back_wrong_fails() {
        let parts = [Some(BOOT), Some(ROOT)];
        let mut c = card(booted(&image(&parts, 1)));
        c.corrupt_at = Some(50_000);
        assert!(run_update(&mut c, &image(&parts, 2)).unwrap_err().contains("at byte 50000"));
    }

    #[test]
    fn a_fresh_install_writes_the_whole_image_table_last() {
        let parts = [Some(BOOT), Some(ROOT)];
        let new = image(&parts, 3);
        let mut c = card(booted(&image(&parts, 1)));
        let mut seen = Vec::new();
        fresh(&mut c, &opener(&new), new.len() as u64, CARD_LEN as u64, &mut |p| seen.push(p), &|| false).unwrap();
        let after = c.data.into_inner();
        assert_eq!(after[..new.len()], new[..], "the whole image, table included");
        assert!(matches!(seen.last(), Some(Progress::Checking { done, total }) if done == total));
    }

    #[test]
    fn a_fresh_install_cut_short_leaves_no_table() {
        let parts = [Some(BOOT), Some(ROOT)];
        let new = image(&parts, 3);
        let mut c = card(booted(&image(&parts, 1)));
        c.fail_after = Some(40_000);
        assert!(fresh(&mut c, &opener(&new), new.len() as u64, CARD_LEN as u64, &mut |_| {}, &|| false).is_err());
        assert_eq!(c.data.into_inner()[..512], [0u8; 512], "no table, not a half system");
    }

    #[test]
    fn a_cancel_leaves_the_update_table() {
        let parts = [Some(BOOT), Some(ROOT)];
        let before = booted(&image(&parts, 1));
        let new = image(&parts, 2);
        let mut c = card(before.clone());
        let err = update(&mut c, &opener(&new), new.len() as u64, SKIP, 2, &mut |_| {}, &|| true).unwrap_err();
        assert_eq!(err, "cancelled");
        assert_eq!(c.data.into_inner()[..512], before[..512]);
    }

    /// The Pixel's layout as TortOS v1.3.0's image has it: the bootloader at
    /// 32 KB, boot at 16 MiB for 64 MiB, root after it for 256 MiB, and the
    /// games partition the first boot adds right after root.
    #[test]
    fn the_pixels_real_layout() {
        let boot = Partition { kind: 0x0C, start: 32768, sectors: 131072 };
        let root = Partition { kind: 0x83, start: 163840, sectors: 524288 };
        let games = Partition { kind: 0x07, start: 688128, sectors: 120_000_000 };
        assert_eq!(plan_update(&table(&[Some(boot), Some(root)]), 352_321_536,
                               &table(&[Some(boot), Some(root), Some(games)]), 32 * 1024, 2).unwrap(),
                   Span { from: 32 * 1024, to: 352_321_536 });
    }

    // A small card in the Brick's shape, in sectors: the GPT's 34, the
    // bootloader from 32 KB, boot, root and state, the image's GPT copy at its
    // end, then the games partition the first boot adds
    const G_SKIP: u64 = 64 * 512;
    const G_BOOT: Partition = Partition { kind: 0, start: 128, sectors: 32 };
    const G_ROOT: Partition = Partition { kind: 0, start: 160, sectors: 80 };
    const G_STATE: Partition = Partition { kind: 0, start: 240, sectors: 16 };
    const G_GAMES: Partition = Partition { kind: 0, start: 296, sectors: 200 };
    const G_ROOT_END: usize = 240 * 512;
    const G_IMAGE_LEN: usize = 289 * 512;
    const G_CARD_LEN: usize = 512 * 512;

    /// A GPT's first sectors as genimage writes them: the protective MBR, the
    /// header, 128 entries from sector 2.
    fn gpt(parts: &[Option<Partition>]) -> Vec<u8> {
        let mut h = vec![0u8; 34 * 512];
        h[..512].copy_from_slice(&table(&[Some(Partition { kind: 0xEE, start: 1, sectors: 0xFFFF_FFFF })]));
        h[512..520].copy_from_slice(b"EFI PART");
        h[512 + 8..512 + 12].copy_from_slice(&0x0001_0000u32.to_le_bytes());
        h[512 + 12..512 + 16].copy_from_slice(&92u32.to_le_bytes());
        h[512 + 24..512 + 32].copy_from_slice(&1u64.to_le_bytes());
        // The second copy at the image's end, as genimage puts it
        h[512 + 32..512 + 40].copy_from_slice(&((G_IMAGE_LEN / 512 - 1) as u64).to_le_bytes());
        h[512 + 40..512 + 48].copy_from_slice(&64u64.to_le_bytes());
        h[512 + 48..512 + 56].copy_from_slice(&((G_IMAGE_LEN / 512 - 34) as u64).to_le_bytes());
        h[512 + 72..512 + 80].copy_from_slice(&2u64.to_le_bytes());
        h[512 + 80..512 + 84].copy_from_slice(&128u32.to_le_bytes());
        h[512 + 84..512 + 88].copy_from_slice(&128u32.to_le_bytes());
        for (i, p) in parts.iter().enumerate() {
            if let Some(p) = p {
                let e = &mut h[1024 + 128 * i..1024 + 128 * (i + 1)];
                e[..16].fill(0x11);
                e[16..32].fill(i as u8 + 1);
                e[32..40].copy_from_slice(&p.start.to_le_bytes());
                e[40..48].copy_from_slice(&(p.end() - 1).to_le_bytes());
            }
        }
        h
    }

    fn gpt_image(seed: u8) -> Vec<u8> {
        let mut v: Vec<u8> = (0..G_IMAGE_LEN).map(|i| (i as u8).wrapping_mul(7).wrapping_add(seed)).collect();
        v[..34 * 512].copy_from_slice(&gpt(&[Some(G_BOOT), Some(G_ROOT), Some(G_STATE)]));
        v[34 * 512..G_SKIP as usize].fill(0);
        v
    }

    /// A card as the Brick leaves it after its first boot: its games
    /// partition added, and the state partition written since.
    fn gpt_booted(old: &[u8]) -> Vec<u8> {
        let mut c = vec![0xEE; G_CARD_LEN];
        c[..old.len()].copy_from_slice(old);
        c[..34 * 512].copy_from_slice(&gpt(&[Some(G_BOOT), Some(G_ROOT), Some(G_STATE), Some(G_GAMES)]));
        c[G_ROOT_END..G_ROOT_END + 512].fill(0x5A); // a pairing
        c
    }

    fn run_brick_update(c: &mut FileCard, img: &[u8]) -> Result<Span, String> {
        update(c, &opener(img), img.len() as u64, G_SKIP, 2, &mut |_| {}, &|| false)
    }

    #[test]
    fn reads_a_gpt() {
        let parts = gpt_partitions(&gpt(&[Some(G_BOOT), Some(G_ROOT), None, Some(G_GAMES)])).unwrap();
        assert_eq!(parts, vec![Some(G_BOOT), Some(G_ROOT), None, Some(G_GAMES)]);
        assert!(gpt_partitions(&vec![0u8; 34 * 512]).is_none());
    }

    #[test]
    fn a_brick_update_writes_through_the_root_and_nothing_else() {
        let before = gpt_booted(&gpt_image(1));
        let new = gpt_image(2);
        let mut c = card(before.clone());
        assert_eq!(run_brick_update(&mut c, &new).unwrap(), Span { from: G_SKIP, to: G_ROOT_END as u64 });
        let after = c.data.into_inner();
        assert_eq!(after[..G_SKIP as usize], before[..G_SKIP as usize], "tables untouched");
        assert_eq!(after[G_SKIP as usize..G_ROOT_END], new[G_SKIP as usize..G_ROOT_END], "boot and root written");
        assert_eq!(after[G_ROOT_END..], before[G_ROOT_END..], "state, the GPT's copy and games untouched");
    }

    #[test]
    fn a_brick_card_never_booted_updates_too() {
        let mut c = card(gpt_image(1));
        let new = gpt_image(2);
        run_brick_update(&mut c, &new).unwrap();
        assert_eq!(c.data.into_inner()[G_SKIP as usize..G_ROOT_END], new[G_SKIP as usize..G_ROOT_END]);
    }

    fn brick_refused(before: Vec<u8>, img: &[u8], says: &str) {
        let mut c = card(before.clone());
        let err = run_brick_update(&mut c, img).unwrap_err();
        assert!(err.contains(says), "{err:?} should say {says:?}");
        assert!(c.data.into_inner() == before, "a refusal wrote to the card");
    }

    #[test]
    fn a_moved_state_partition_is_refused() {
        let mut before = gpt_booted(&gpt_image(1));
        let moved = Partition { start: 248, ..G_STATE };
        before[..34 * 512].copy_from_slice(&gpt(&[Some(G_BOOT), Some(G_ROOT), Some(moved), Some(G_GAMES)]));
        brick_refused(before, &gpt_image(2), "partition 3");
    }

    #[test]
    fn a_card_with_another_kind_of_table_is_refused() {
        // A Brick card on TrimUI's firmware: one FAT32 partition in an MBR
        let mut trimui = vec![0x11; G_CARD_LEN];
        trimui[..512].copy_from_slice(&table(&[Some(Partition { kind: 0x0C, start: 64, sectors: 400 })]));
        brick_refused(trimui, &gpt_image(2), "another kind");
        // And the Pixel's image onto the Brick's card
        let mut c = card(gpt_booted(&gpt_image(1)));
        let pixel = image(&[Some(BOOT), Some(ROOT)], 2);
        assert!(run_update(&mut c, &pixel).unwrap_err().contains("another kind"));
    }

    /// A GPT header's checksum, recomputed
    fn header_ok(h: &[u8]) -> bool {
        let size = u32::from_le_bytes(h[12..16].try_into().unwrap()) as usize;
        let mut c = h[..size].to_vec();
        let want = u32::from_le_bytes(c[16..20].try_into().unwrap());
        c[16..20].fill(0);
        crc32(&c) == want
    }

    #[test]
    fn a_brick_fresh_install_fits_the_gpt_to_the_card() {
        let new = gpt_image(3);
        let mut c = card(gpt_booted(&gpt_image(1)));
        fresh(&mut c, &opener(&new), new.len() as u64, G_CARD_LEN as u64, &mut |_| {}, &|| false).unwrap();
        let after = c.data.into_inner();
        let last = (G_CARD_LEN / 512 - 1) as u64;
        let u64_at = |o: usize| u64::from_le_bytes(after[o..o + 8].try_into().unwrap());
        assert_eq!(after[34 * 512..G_IMAGE_LEN - 512], new[34 * 512..G_IMAGE_LEN - 512], "the image between");
        assert_eq!(gpt_partitions(&after[..34 * 512]).unwrap(), vec![Some(G_BOOT), Some(G_ROOT), Some(G_STATE)]);
        assert!(header_ok(&after[512..]), "the primary's checksum");
        assert_eq!((u64_at(512 + 32), u64_at(512 + 48), u64_at(512 + 72)), (last, last - 33, 2));
        assert_eq!(u32::from_le_bytes(after[458..462].try_into().unwrap()), last as u32, "the protective MBR covers the card");
        let backup = (last as usize) * 512;
        assert_eq!(&after[backup..backup + 8], b"EFI PART");
        assert!(header_ok(&after[backup..]), "the second copy's checksum");
        assert_eq!((u64_at(backup + 24), u64_at(backup + 32), u64_at(backup + 72)), (last, 1, last - 32));
        assert_eq!(after[(last as usize - 32) * 512..backup], after[1024..34 * 512], "the same list at the end");
        assert_eq!(after[G_IMAGE_LEN - 512..G_IMAGE_LEN], [0u8; 512], "the image's own second header cleared");
    }

    #[test]
    fn crc32_is_the_gpts() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn a_card_smaller_than_the_image_is_refused_before_anything_is_written() {
        let new = gpt_image(3);
        let before = gpt_booted(&gpt_image(1));
        let mut c = card(before.clone());
        assert!(fresh(&mut c, &opener(&new), new.len() as u64, 200 * 512, &mut |_| {}, &|| false).unwrap_err().contains("too small"));
        assert!(c.data.into_inner() == before);
    }

    #[test]
    fn a_brick_fresh_install_cut_short_leaves_no_table() {
        let new = gpt_image(3);
        let mut c = card(gpt_booted(&gpt_image(1)));
        c.fail_after = Some(60_000);
        assert!(fresh(&mut c, &opener(&new), new.len() as u64, G_CARD_LEN as u64, &mut |_| {}, &|| false).is_err());
        assert!(c.data.into_inner()[..34 * 512].iter().all(|&b| b == 0), "no MBR and no GPT header");
    }

    /// The Brick's layout as plastron's image has it (2026-10-24): the
    /// bootloader at 128 KB, boot at 4 MiB for 64 MiB, root after it for 256
    /// MiB, state for 16 MiB, the GPT's copy in the image's last 33 sectors,
    /// and the games partition the first boot adds on the next 4 MiB.
    #[test]
    fn the_bricks_real_layout() {
        let boot = Partition { kind: 0, start: 8192, sectors: 131072 };
        let root = Partition { kind: 0, start: 139264, sectors: 524288 };
        let state = Partition { kind: 0, start: 663552, sectors: 32768 };
        let games = Partition { kind: 0, start: 696320, sectors: 120_000_000 };
        assert_eq!(plan_update(&gpt(&[Some(boot), Some(root), Some(state)]), 356_536_320,
                               &gpt(&[Some(boot), Some(root), Some(state), Some(games)]), 128 * 1024, 2).unwrap(),
                   Span { from: 128 * 1024, to: 663552 * 512 });
    }
}
