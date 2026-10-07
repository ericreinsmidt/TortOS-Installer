// Putting TortOS on a card: the steps between the button and the writer.
//
// The GKD Pixel 2: its image, fresh or as an update. The Brick's fresh
// install: a FAT32 card built here with the release in it (fat32.rs), written
// the same way. The Brick's update needs none of this: it copies files onto
// the card's mounted volume (brick.rs).
//
// On macOS and Windows the card is opened here (device.rs). On Linux the
// window isn't root, so the same work runs in a helper the installer starts
// through pkexec: this program again, with `--write` and no window, printing
// its progress a line at a time for the window to read.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::writer::Progress;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Update,
    Fresh,
    BrickFresh,
}

impl Action {
    pub fn parse(s: &str) -> Option<Action> {
        match s {
            "update" => Some(Action::Update),
            "fresh" => Some(Action::Fresh),
            "brick-fresh" => Some(Action::BrickFresh),
            _ => None,
        }
    }
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    fn word(self) -> &'static str {
        match self {
            Action::Update => "update",
            Action::Fresh => "fresh",
            Action::BrickFresh => "brick-fresh",
        }
    }
}

/// Where a Pixel 2 update starts: its bootloader, at 32 KB. The table before
/// it is the card's own and stays.
const PIXEL_SKIP: u64 = 32 * 1024;

/// The write itself, with the card opened here. What the helper runs, and
/// what macOS and Windows run in the window's process. `source` is the
/// Pixel's image or the Brick's zip; `card_bytes` the card's size.
pub fn write_here(
    card_id: &str,
    action: Action,
    source: &Path,
    card_bytes: u64,
    progress: &mut dyn FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<(), String> {
    let cancelled = || cancel.load(Ordering::Relaxed);
    let result = match action {
        Action::Update | Action::Fresh => {
            let len = crate::image::size(source).map_err(|e| format!("Reading the image: {e}"))?;
            let open = || crate::image::open(source);
            let mut card = crate::device::open(card_id)?;
            let result = if action == Action::Update {
                crate::writer::update(&mut card, &open, len, PIXEL_SKIP, progress, &cancelled).map(|_| ())
            } else {
                crate::writer::fresh(&mut card, &open, len, progress, &cancelled)
            };
            crate::device::close(card, card_id);
            result
        }
        Action::BrickFresh => {
            let entries = crate::brick::entries(source)?;
            // Folders the release doesn't date take its newest file's date
            let time = entries.iter().map(|e| e.time).max().unwrap_or(0);
            let id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u32)
                .unwrap_or(0x7E57_C0DE);
            let image = crate::fat32::build(card_bytes, entries, time, id)?;
            let open = || Ok(image.open());
            let mut card = crate::device::open(card_id)?;
            let result = crate::writer::fresh(&mut card, &open, image.len, progress, &cancelled);
            crate::device::close(card, card_id);
            result
        }
    };
    result
}

/// The write, wherever it has to run: here, or on Linux in the helper.
pub fn run(
    card_id: &str,
    action: Action,
    source: &Path,
    card_bytes: u64,
    progress: &mut dyn FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return run_helper(card_id, action, source, card_bytes, progress, cancel);
    #[cfg(not(target_os = "linux"))]
    return write_here(card_id, action, source, card_bytes, progress, cancel);
}

/// Linux: pkexec runs this program again as root with `--write`, which asks
/// for the password through the desktop's own prompt.
#[cfg(target_os = "linux")]
fn run_helper(
    card_id: &str,
    action: Action,
    source: &Path,
    card_bytes: u64,
    progress: &mut dyn FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let me = std::env::current_exe().map_err(|e| format!("Finding the installer: {e}"))?;
    let mut child = Command::new("pkexec")
        .arg(&me)
        .args(["--write", action.word(), card_id])
        .arg(source)
        .arg(card_bytes.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn't run pkexec, which asks for the password: {e}"))?;
    let out = child.stdout.take().expect("piped");
    let mut last = None;
    for line in BufReader::new(out).lines().map_while(Result::ok) {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("cancelled".into());
        }
        match parse_line(&line) {
            Some(Line::Progress(p)) => progress(p),
            Some(Line::Done(r)) => last = Some(r),
            None => {}
        }
    }
    let status = child.wait().map_err(|e| format!("The helper: {e}"))?;
    match (last, status.code()) {
        (Some(r), _) => r,
        // pkexec's own: 126 the password was dismissed, 127 not authorized
        (None, Some(126)) | (None, Some(127)) => {
            Err("The card wasn't opened: the password was cancelled or not accepted.".into())
        }
        (None, _) => Err("The helper stopped without saying why.".into()),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
/// The helper's lines: `writing DONE TOTAL`, `checking DONE TOTAL`, then
/// `ok` or `error MESSAGE`.
enum Line {
    Progress(Progress),
    Done(Result<(), String>),
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_line(line: &str) -> Option<Line> {
    let mut f = line.splitn(3, ' ');
    let word = f.next()?;
    let num = |s: Option<&str>| s.and_then(|x| x.parse::<u64>().ok());
    match word {
        "writing" | "checking" => {
            let (done, total) = (num(f.next())?, num(f.next())?);
            Some(Line::Progress(if word == "writing" {
                Progress::Writing { done, total }
            } else {
                Progress::Checking { done, total }
            }))
        }
        "ok" => Some(Line::Done(Ok(()))),
        "error" => Some(Line::Done(Err(line["error ".len().min(line.len())..].to_string()))),
        _ => None,
    }
}

/// The helper: `--write update|fresh|brick-fresh CARD SOURCE CARD_BYTES`, run
/// as root on Linux.
/// Prints progress at most every few hundred milliseconds.
pub fn helper(args: &[String]) -> i32 {
    let (Some(action), Some(card), Some(source), Some(card_bytes)) = (
        args.first().and_then(|a| Action::parse(a)),
        args.get(1),
        args.get(2).map(PathBuf::from),
        args.get(3).and_then(|n| n.parse::<u64>().ok()),
    ) else {
        eprintln!("usage: tortos-installer --write update|fresh|brick-fresh CARD SOURCE CARD_BYTES");
        return 2;
    };
    let mut last = std::time::Instant::now();
    let mut last_word = "";
    let mut progress = |p: Progress| {
        let (word, done, total) = match p {
            // The window downloads before the helper starts
            Progress::Downloading { .. } => return,
            Progress::Writing { done, total } => ("writing", done, total),
            Progress::Checking { done, total } => ("checking", done, total),
        };
        if word != last_word || done == total || last.elapsed().as_millis() >= 200 {
            println!("{word} {done} {total}");
            last = std::time::Instant::now();
            last_word = word;
        }
    };
    let never = AtomicBool::new(false);
    match write_here(card, action, &source, card_bytes, &mut progress, &never) {
        Ok(()) => {
            println!("ok");
            0
        }
        Err(e) => {
            println!("error {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_helpers_lines() {
        assert!(matches!(parse_line("writing 10 20"), Some(Line::Progress(Progress::Writing { done: 10, total: 20 }))));
        assert!(matches!(parse_line("checking 20 20"), Some(Line::Progress(Progress::Checking { done: 20, total: 20 }))));
        assert!(matches!(parse_line("ok"), Some(Line::Done(Ok(())))));
        match parse_line("error This card's partitions aren't where") {
            Some(Line::Done(Err(e))) => assert_eq!(e, "This card's partitions aren't where"),
            _ => panic!(),
        }
        assert!(parse_line("something else").is_none());
        assert_eq!(Action::parse("fresh").map(Action::word), Some("fresh"));
        assert_eq!(Action::parse("brick-fresh").map(Action::word), Some("brick-fresh"));
    }
}
