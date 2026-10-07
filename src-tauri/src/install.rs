// Putting TortOS on a card: the steps between the button and the writer.
//
// The GKD Pixel 2 for now: its image, fresh or as an update. The Brick comes
// in the next step.
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
}

impl Action {
    pub fn parse(s: &str) -> Option<Action> {
        match s {
            "update" => Some(Action::Update),
            "fresh" => Some(Action::Fresh),
            _ => None,
        }
    }
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    fn word(self) -> &'static str {
        match self {
            Action::Update => "update",
            Action::Fresh => "fresh",
        }
    }
}

/// Where a Pixel 2 update starts: its bootloader, at 32 KB. The table before
/// it is the card's own and stays.
const PIXEL_SKIP: u64 = 32 * 1024;

/// The write itself, with the card opened here. What the helper runs, and
/// what macOS and Windows run in the window's process.
pub fn write_here(
    card_id: &str,
    action: Action,
    image: &Path,
    progress: &mut dyn FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<(), String> {
    let len = crate::image::size(image).map_err(|e| format!("Reading the image: {e}"))?;
    let open = || crate::image::open(image);
    let mut card = crate::device::open(card_id)?;
    let cancelled = || cancel.load(Ordering::Relaxed);
    match action {
        Action::Update => crate::writer::update(&mut card, &open, len, PIXEL_SKIP, progress, &cancelled).map(|_| ()),
        Action::Fresh => crate::writer::fresh(&mut card, &open, len, progress, &cancelled),
    }
}

/// The write, wherever it has to run: here, or on Linux in the helper.
pub fn run(
    card_id: &str,
    action: Action,
    image: &Path,
    progress: &mut dyn FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return run_helper(card_id, action, image, progress, cancel);
    #[cfg(not(target_os = "linux"))]
    return write_here(card_id, action, image, progress, cancel);
}

/// Linux: pkexec runs this program again as root with `--write`, which asks
/// for the password through the desktop's own prompt.
#[cfg(target_os = "linux")]
fn run_helper(
    card_id: &str,
    action: Action,
    image: &Path,
    progress: &mut dyn FnMut(Progress),
    cancel: &AtomicBool,
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let me = std::env::current_exe().map_err(|e| format!("Finding the installer: {e}"))?;
    let mut child = Command::new("pkexec")
        .arg(&me)
        .args(["--write", action.word(), card_id])
        .arg(image)
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

/// The helper: `--write update|fresh CARD IMAGE`, run as root on Linux.
/// Prints progress at most every few hundred milliseconds.
pub fn helper(args: &[String]) -> i32 {
    let (Some(action), Some(card), Some(image)) = (
        args.first().and_then(|a| Action::parse(a)),
        args.get(1),
        args.get(2).map(PathBuf::from),
    ) else {
        eprintln!("usage: tortos-installer --write update|fresh CARD IMAGE");
        return 2;
    };
    let mut last = std::time::Instant::now();
    let mut progress = |p: Progress| {
        let (word, done, total) = match p {
            Progress::Writing { done, total } => ("writing", done, total),
            Progress::Checking { done, total } => ("checking", done, total),
        };
        if done == total || last.elapsed().as_millis() >= 200 {
            println!("{word} {done} {total}");
            last = std::time::Instant::now();
        }
    };
    let never = AtomicBool::new(false);
    match write_here(card, action, &image, &mut progress, &never) {
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
    }
}
