// TortOS Installer: the window, and the commands its page calls. Run with
// `--write` it is the helper instead, with no window (install.rs).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod cards;
mod device;
mod image;
mod install;
mod release;
mod writer;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

/// Windows' AutoPlay held off while a card is written. Blanking a card's table
/// makes Windows mount it as one unformatted volume, and AutoPlay then offered
/// to format it, or opened it and said "The directory name is invalid", in the
/// middle of an install (seen 2026-10-24). Windows asks the window in front
/// with the "QueryCancelAutoPlay" message before it shows anything, and an
/// answer of 1 cancels it: the installer's window answers so while it writes.
#[cfg(windows)]
mod autoplay {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use windows::core::w;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;

    pub static HOLD: AtomicBool = AtomicBool::new(false);
    static QUERY: AtomicU32 = AtomicU32::new(0);

    unsafe extern "system" fn answer(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _id: usize, _data: usize) -> LRESULT {
        if msg != 0 && msg == QUERY.load(Ordering::Relaxed) && HOLD.load(Ordering::Relaxed) {
            return LRESULT(1);
        }
        DefSubclassProc(hwnd, msg, wp, lp)
    }

    /// The window to answer from: called on its own thread
    pub fn watch(hwnd: isize) {
        unsafe {
            QUERY.store(RegisterWindowMessageW(w!("QueryCancelAutoPlay")), Ordering::Relaxed);
            let _ = SetWindowSubclass(HWND(hwnd as _), Some(answer), 1, 0);
        }
    }
}

/// The write under way, if any: its cancel flag.
#[derive(Default)]
struct Running(Mutex<Option<Arc<AtomicBool>>>);

#[derive(serde::Serialize, Clone)]
struct ProgressEvent {
    phase: &'static str,
    done: u64,
    total: u64,
}

#[derive(serde::Serialize, Clone)]
struct FinishedEvent {
    ok: bool,
    cancelled: bool,
    message: String,
    /// Where a backup was made, for the screen to say
    backup: Option<String>,
    /// Whether the card was ejected after a job that finished well, and if
    /// not, why
    ejected: bool,
    eject_error: String,
}

/// A card, and the backup waiting to go back on it, if any: a started Brick
/// card on its own system, the size of a card that was moved over.
#[derive(serde::Serialize)]
struct CardView {
    #[serde(flatten)]
    card: cards::Card,
    backup: Option<backup::Pending>,
}

/// The cards in this computer and what is on each. Asks the system for its
/// disks, which takes a moment, so it runs off the window's thread.
#[tauri::command]
async fn list_cards() -> Vec<CardView> {
    let list = || {
        let folder = backup::folder();
        cards::list()
            .into_iter()
            .map(|card| {
                let waiting = card.device == Some(cards::Device::Brick) && card.started && !card.trimui;
                let backup = folder.as_deref().filter(|_| waiting).and_then(|f| backup::pending(f, card.size_bytes));
                CardView { card, backup }
            })
            .collect()
    };
    tauri::async_runtime::spawn_blocking(list).await.unwrap_or_default()
}

/// How much a backup of this card's saves, settings and BIOS files would be,
/// and where it would go.
#[tauri::command]
async fn backup_plan(card: String) -> Result<(u64, String), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let found = cards::list().into_iter().find(|c| c.id == card).ok_or("The card isn't there anymore.")?;
        let volume = found.volume.ok_or("The card isn't mounted. Take it out and put it back in.")?;
        let folder = backup::folder().ok_or("This computer has no Documents or home folder for the backup.")?;
        Ok((backup::size(std::path::Path::new(&volume))?, folder.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// TortOS's latest version, for the screen to say what it installs.
#[tauri::command]
async fn latest_tortos() -> Result<String, String> {
    if let Some(path) = local_release(true).or_else(|| local_release(false)) {
        return Ok(format!("from {}", path.file_name().unwrap_or_default().to_string_lossy()));
    }
    tauri::async_runtime::spawn_blocking(|| release::latest().map(|r| r.version().to_string()))
        .await
        .map_err(|e| e.to_string())?
}

/// A newer installer's version, if one is out.
#[tauri::command]
async fn newer_installer() -> Option<String> {
    tauri::async_runtime::spawn_blocking(release::newer_installer).await.ok().flatten()
}

/// The installer's releases page, in the browser. A fixed address: nothing
/// from the page goes into the command.
#[tauri::command]
fn open_installer_releases() {
    let url = release::INSTALLER_RELEASES;
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(windows)]
    let opener = "explorer";
    #[cfg(target_os = "linux")]
    let opener = "xdg-open";
    let _ = std::process::Command::new(opener).arg(url).spawn();
}

/// A card image to use instead of downloading one, for trying out a build
/// before it is published: TORTOS_BRICK_IMAGE for the Brick, TORTOS_IMAGE for
/// the Pixel.
fn local_release(brick: bool) -> Option<PathBuf> {
    std::env::var_os(if brick { "TORTOS_BRICK_IMAGE" } else { "TORTOS_IMAGE" }).map(PathBuf::from)
}

/// What the window asked for
#[derive(Clone)]
enum Job {
    Write(install::Action),
    /// A Brick card on TrimUI's firmware to the Brick's own system: its saves,
    /// settings and BIOS files backed up into a folder of this name, then a
    /// fresh install
    Move(String),
    /// A backup put back on the card
    Restore,
}

fn find(card_id: &str) -> Result<cards::Card, String> {
    cards::list()
        .into_iter()
        .find(|c| c.id == card_id)
        .ok_or_else(|| "The card isn't there anymore. Put it back in and choose it again.".into())
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The job, once the card is found again. Returns where a backup was made.
fn run(
    card_id: &str,
    brick: bool,
    job: Job,
    progress: &mut dyn FnMut(writer::Progress),
    cancel: &AtomicBool,
) -> Result<Option<String>, String> {
    let cancelled = || cancel.load(Ordering::Relaxed);
    match job {
        Job::Write(action) => write(card_id, brick, action, progress, cancel).map(|_| None),
        Job::Move(label) => {
            let card = find(card_id)?;
            if !brick || !card.trimui {
                return Err("Only a Brick card on TrimUI's firmware is moved over.".into());
            }
            let volume = card.volume.ok_or("The card isn't mounted. Take it out and put it back in.")?;
            let folder = backup::folder().ok_or("This computer has no Documents or home folder for the backup.")?;
            let made = backup::back_up(std::path::Path::new(&volume), &folder, &label, card.size_bytes, now(),
                                       progress, &cancelled)?;
            let shown = made.to_string_lossy().into_owned();
            write(card_id, true, install::Action::Fresh, progress, cancel)
                .map(|_| Some(shown.clone()))
                .map_err(|e| if e == "cancelled" { e } else { format!("{e} Your saves and settings are in {shown}.") })
        }
        Job::Restore => {
            let card = find(card_id)?;
            let volume = card.volume.ok_or("The card isn't mounted. Take it out and put it back in.")?;
            let folder = backup::folder().ok_or("No backup folder on this computer.")?;
            let waiting = backup::pending(&folder, card.size_bytes).ok_or("There is no backup waiting for this card.")?;
            backup::restore(std::path::Path::new(&waiting.path), std::path::Path::new(&volume), progress, &cancelled)
                .map(|_| None)
        }
    }
}

/// The write, once the card is found again: it must still be there, and an
/// update must be onto that device's TortOS, on its own system: a Brick card
/// on TrimUI's firmware only moves over by a fresh install. The release is
/// downloaded first, and the download removed after.
fn write(
    card_id: &str,
    brick: bool,
    action: install::Action,
    progress: &mut dyn FnMut(writer::Progress),
    cancel: &AtomicBool,
) -> Result<(), String> {
    let card = find(card_id)?;
    let device = if brick { cards::Device::Brick } else { cards::Device::Pixel2 };
    if action == install::Action::Update && card.device != Some(device) {
        return Err("This card doesn't have that device's TortOS on it to update.".into());
    }
    if action == install::Action::Update && card.trimui {
        return Err("This card runs TrimUI's firmware, so it can't be updated: moving it to TortOS's own \
                    system erases it. A fresh install does that."
            .into());
    }
    let (source, downloaded) = match local_release(brick) {
        Some(path) => (path, false),
        None => {
            let latest = release::latest()?;
            let asset = latest.asset(brick)?;
            let dir = std::env::temp_dir().join("tortos-installer");
            (release::download(&latest, asset, &dir, progress, &|| cancel.load(Ordering::Relaxed))?, true)
        }
    };
    let action = match action {
        install::Action::Update if brick => install::Action::BrickUpdate,
        a => a,
    };
    let result = install::run(card_id, action, &source, card.size_bytes, progress, cancel);
    if downloaded {
        let _ = std::fs::remove_file(&source);
    }
    result
}

/// Starts a job; progress and the end arrive as "progress" and "finished"
/// events. `action` is update, fresh, move (with `label`, the backup folder's
/// name, made by the page in the computer's own time) or restore.
#[tauri::command]
fn start(app: AppHandle, running: State<Running>, card: String, device: String, action: String,
         label: Option<String>) -> Result<(), String> {
    let brick = match device.as_str() {
        "brick" => true,
        "pixel2" => false,
        _ => return Err("Unknown device".into()),
    };
    let job = match (action.as_str(), label) {
        ("move", Some(l)) if !l.is_empty() && l.len() <= 40
            && l.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-') => Job::Move(l),
        ("restore", _) => Job::Restore,
        (a, _) => match install::Action::parse(a) {
            Some(a @ (install::Action::Update | install::Action::Fresh)) => Job::Write(a),
            _ => return Err("Unknown action".into()),
        },
    };
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut slot = running.0.lock().unwrap();
        if slot.is_some() {
            return Err("A card is already being written.".into());
        }
        *slot = Some(cancel.clone());
    }
    std::thread::spawn(move || {
        let mut last = std::time::Instant::now();
        let mut last_phase = "";
        let mut progress = |p: writer::Progress| {
            let (phase, done, total) = match p {
                writer::Progress::Downloading { done, total } => ("downloading", done, total),
                writer::Progress::Writing { done, total } => ("writing", done, total),
                writer::Progress::Checking { done, total } => ("checking", done, total),
                writer::Progress::Copying { done, total } => ("copying", done, total),
            };
            // A new phase always gets through: the screen tells a stop before
            // anything was written from one after
            if phase != last_phase || done == total || last.elapsed().as_millis() >= 100 {
                let _ = app.emit("progress", ProgressEvent { phase, done, total });
                last = std::time::Instant::now();
                last_phase = phase;
            }
        };
        #[cfg(windows)]
        autoplay::HOLD.store(true, Ordering::Relaxed);
        let result = run(&card, brick, job, &mut progress, &cancel);
        // A card that finished well is ejected, ready to take out; one that
        // didn't stays, to be written again
        let eject = result.is_ok().then(|| {
            let _ = app.emit("progress", ProgressEvent { phase: "ejecting", done: 0, total: 0 });
            device::eject(&card)
        });
        #[cfg(windows)]
        autoplay::HOLD.store(false, Ordering::Relaxed);
        if let Some(r) = app.try_state::<Running>() {
            *r.0.lock().unwrap() = None;
        }
        let cancelled = matches!(&result, Err(e) if e == "cancelled");
        let (ok, backup, message) = match result {
            Ok(b) => (true, b, String::new()),
            Err(e) => (false, None, e),
        };
        let ejected = matches!(eject, Some(Ok(())));
        let eject_error = eject.and_then(Result::err).unwrap_or_default();
        let _ = app.emit("finished", FinishedEvent { ok, cancelled, message, backup, ejected, eject_error });
    });
    Ok(())
}

/// Stops the write at the next chunk.
#[tauri::command]
fn cancel(running: State<Running>) {
    if let Some(c) = running.0.lock().unwrap().as_ref() {
        c.store(true, Ordering::Relaxed);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--write") {
        std::process::exit(install::helper(&args[2..]));
    }
    tauri::Builder::default()
        .manage(Running::default())
        .setup(|_app| {
            #[cfg(windows)]
            if let Some(Ok(hwnd)) = _app.get_webview_window("main").map(|w| w.hwnd()) {
                autoplay::watch(hwnd.0 as isize);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![list_cards, backup_plan, latest_tortos, newer_installer,
                                                 open_installer_releases, start, cancel])
        .run(tauri::generate_context!())
        .expect("TortOS Installer could not start");
}
