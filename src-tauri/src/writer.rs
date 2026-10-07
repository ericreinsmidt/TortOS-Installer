// Writing a card image to a card: the GKD Pixel 2's whole image for a fresh
// install, or only its system part for an update.
//
// AN UPDATE writes the image from `skip` (the Pixel's bootloader, at 32 KB) to
// the end of its last partition, and nothing past it: the games partition the
// first boot added over the rest of the card stays as it is. That is only
// safe while the card's partitions are where the image's are, so the card's
// own table is checked first, and nothing is written when they differ. Its
// table is never written either, so an update cut short is finished by
// running it again.
//
// A FRESH INSTALL writes the whole image, the partition table LAST. The
// first thing written is a blank table, so the computer sees no partitions to
// mount while the rest goes on, and a card that is pulled halfway has no
// table at all rather than half a system that looks whole.
//
// Both read everything back against a second pass of the image: no checksum
// to trust, and a mismatch says where it is. Everything here works on any
// Read + Write + Seek, so the tests run it on files.

use std::io::{self, Read, Seek, SeekFrom, Write};

pub const SECTOR: u64 = 512;

/// Written and read this much at a time: a whole number of sectors, as raw
/// disks on Windows and macOS insist.
const CHUNK: usize = 4 << 20;

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

/// What an update of this card from this image would write, or why it can't.
pub fn plan_update(image0: &[u8], image_len: u64, card0: &[u8], skip: u64) -> Result<Span, String> {
    let image = partitions(image0).ok_or("The image has no partition table")?;
    let card = partitions(card0).ok_or_else(|| layout_differs("the card has no partition table"))?;
    let ours: Vec<(usize, Partition)> =
        image.iter().enumerate().filter_map(|(i, p)| p.map(|p| (i, p))).collect();
    let first = ours.iter().map(|(_, p)| p.start).min().ok_or("The image has no partitions")? * SECTOR;
    let to = ours.iter().map(|(_, p)| p.end()).max().unwrap_or(0) * SECTOR;
    if skip < SECTOR || skip % SECTOR != 0 || skip > first {
        return Err(format!("An update can't start at byte {skip} of this image"));
    }
    if image_len < to {
        return Err(format!("The image ({image_len} bytes) is shorter than its partitions ({to})"));
    }
    // The image's partitions exactly where the card has them...
    for &(i, p) in &ours {
        match card[i] {
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
            if image[i].is_none() && c.start * SECTOR < to {
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

fn read_sector0(r: &mut dyn Read) -> Result<[u8; 512], String> {
    let mut s = [0u8; 512];
    r.read_exact(&mut s).map_err(|e| format!("Reading a partition table: {e}"))?;
    Ok(s)
}

/// An update: the system part only, after checking the card can take it.
pub fn update(
    card: &mut dyn Card,
    open: OpenImage,
    image_len: u64,
    skip: u64,
    progress: &mut dyn FnMut(Progress),
    cancelled: &dyn Fn() -> bool,
) -> Result<Span, String> {
    card.seek(SeekFrom::Start(0)).map_err(|e| format!("Seeking on the card: {e}"))?;
    let card0 = read_sector0(card)?;
    let image0 = read_sector0(&mut *open().map_err(|e| format!("Opening the image: {e}"))?)?;
    let span = plan_update(&image0, image_len, &card0, skip)?;
    write_span(card, open, span.from, span.to, progress, cancelled)?;
    Ok(span)
}

/// A fresh install: the whole image, a blank table first and the real one
/// last. `image_len` must be a whole number of sectors.
pub fn fresh(
    card: &mut dyn Card,
    open: OpenImage,
    image_len: u64,
    progress: &mut dyn FnMut(Progress),
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if image_len < SECTOR || image_len % SECTOR != 0 {
        return Err(format!("The image ({image_len} bytes) isn't a whole number of sectors"));
    }
    let image0 = read_sector0(&mut *open().map_err(|e| format!("Opening the image: {e}"))?)?;
    partitions(&image0).ok_or("The image has no partition table")?;
    // From here the card is changed: the screen hears so first
    progress(Progress::Writing { done: 0, total: image_len - SECTOR });
    card.seek(SeekFrom::Start(0)).map_err(|e| format!("Seeking on the card: {e}"))?;
    card.write_all(&[0u8; 512]).map_err(|e| format!("Clearing the card's table: {e}"))?;
    card.sync().map_err(|e| format!("Flushing the card: {e}"))?;
    write_span(card, open, SECTOR, image_len, progress, cancelled)?;
    card.seek(SeekFrom::Start(0)).map_err(|e| format!("Seeking on the card: {e}"))?;
    card.write_all(&image0).map_err(|e| format!("Writing the card's table: {e}"))?;
    card.sync().map_err(|e| format!("Flushing the card: {e}"))?;
    card.drop_cache().map_err(|e| format!("Clearing the card's cache: {e}"))?;
    card.seek(SeekFrom::Start(0)).map_err(|e| format!("Seeking on the card: {e}"))?;
    if read_sector0(card)? != image0 {
        return Err("The card didn't read back its new partition table. The card may be failing: \
                    try again, or another card."
            .into());
    }
    Ok(())
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
        update(c, &opener(img), img.len() as u64, SKIP, &mut |_| {}, &|| false)
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
        fresh(&mut c, &opener(&new), new.len() as u64, &mut |p| seen.push(p), &|| false).unwrap();
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
        assert!(fresh(&mut c, &opener(&new), new.len() as u64, &mut |_| {}, &|| false).is_err());
        assert_eq!(c.data.into_inner()[..512], [0u8; 512], "no table, not a half system");
    }

    #[test]
    fn a_cancel_leaves_the_update_table() {
        let parts = [Some(BOOT), Some(ROOT)];
        let before = booted(&image(&parts, 1));
        let new = image(&parts, 2);
        let mut c = card(before.clone());
        let err = update(&mut c, &opener(&new), new.len() as u64, SKIP, &mut |_| {}, &|| true).unwrap_err();
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
                               &table(&[Some(boot), Some(root), Some(games)]), 32 * 1024).unwrap(),
                   Span { from: 32 * 1024, to: 352_321_536 });
    }
}
