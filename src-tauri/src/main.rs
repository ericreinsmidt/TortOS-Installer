// TortOS Installer: the window, and the commands its page calls. Run with
// `--write` it is the helper instead, with no window (install.rs).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod brick;
mod cards;
mod device;
mod fat32;
mod image;
mod install;
mod writer;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

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
}

/// The cards in this computer and what is on each. Asks the system for its
/// disks, which takes a moment, so it runs off the window's thread.
#[tauri::command]
async fn list_cards() -> Vec<cards::Card> {
    tauri::async_runtime::spawn_blocking(cards::list).await.unwrap_or_default()
}

/// The release to put on the card: the Pixel's image or the Brick's zip.
/// Downloading the latest TortOS comes in a later step; until then they are
/// files named by TORTOS_IMAGE and TORTOS_ZIP.
fn source_path(brick: bool) -> Result<PathBuf, String> {
    std::env::var_os(if brick { "TORTOS_ZIP" } else { "TORTOS_IMAGE" })
        .map(PathBuf::from)
        .ok_or_else(|| "Nothing to write yet: downloading TortOS isn't built yet.".to_string())
}

/// The write, once the card is found again: it must still be there, and an
/// update must be onto that device's TortOS.
fn write(
    card_id: &str,
    brick: bool,
    action: install::Action,
    source: &std::path::Path,
    progress: &mut dyn FnMut(writer::Progress),
    cancel: &AtomicBool,
) -> Result<(), String> {
    let card = cards::list()
        .into_iter()
        .find(|c| c.id == card_id)
        .ok_or("The card isn't there anymore. Put it back in and choose it again.")?;
    let device = if brick { cards::Device::Brick } else { cards::Device::Pixel2 };
    if action == install::Action::Update && card.device != Some(device) {
        return Err("This card doesn't have that device's TortOS on it to update.".into());
    }
    match (brick, action) {
        (true, install::Action::Update) => {
            let volume = card.volume.ok_or("The card isn't mounted, so it can't be updated. Take it out and put it back in.")?;
            let entries = brick::entries(source)?;
            brick::update(std::path::Path::new(&volume), &entries, progress, &|| cancel.load(Ordering::Relaxed))
        }
        (true, _) => install::run(card_id, install::Action::BrickFresh, source, card.size_bytes, progress, cancel),
        (false, _) => install::run(card_id, action, source, card.size_bytes, progress, cancel),
    }
}

/// Starts writing; progress and the end arrive as "progress" and "finished"
/// events.
#[tauri::command]
fn start(app: AppHandle, running: State<Running>, card: String, device: String, action: String) -> Result<(), String> {
    let brick = match device.as_str() {
        "brick" => true,
        "pixel2" => false,
        _ => return Err("Unknown device".into()),
    };
    let action = match install::Action::parse(&action) {
        Some(a @ (install::Action::Update | install::Action::Fresh)) => a,
        _ => return Err("Unknown action".into()),
    };
    let source = source_path(brick)?;
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
        let mut progress = |p: writer::Progress| {
            let (phase, done, total) = match p {
                writer::Progress::Writing { done, total } => ("writing", done, total),
                writer::Progress::Checking { done, total } => ("checking", done, total),
            };
            if done == total || last.elapsed().as_millis() >= 100 {
                let _ = app.emit("progress", ProgressEvent { phase, done, total });
                last = std::time::Instant::now();
            }
        };
        let result = write(&card, brick, action, &source, &mut progress, &cancel);
        if let Some(r) = app.try_state::<Running>() {
            *r.0.lock().unwrap() = None;
        }
        let cancelled = matches!(&result, Err(e) if e == "cancelled");
        let _ = app.emit("finished", FinishedEvent {
            ok: result.is_ok(),
            cancelled,
            message: result.err().unwrap_or_default(),
        });
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
        .invoke_handler(tauri::generate_handler![list_cards, start, cancel])
        .run(tauri::generate_context!())
        .expect("TortOS Installer could not start");
}
