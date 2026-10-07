// TortOS Installer: the window, and the commands its page calls. Run with
// `--write` it is the helper instead, with no window (install.rs).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cards;
mod device;
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

/// The image to write. Downloading the latest TortOS comes in a later step;
/// until then it is a file named by TORTOS_IMAGE.
fn image_path() -> Result<PathBuf, String> {
    std::env::var_os("TORTOS_IMAGE")
        .map(PathBuf::from)
        .ok_or_else(|| "No image to write yet: downloading TortOS isn't built yet.".to_string())
}

/// Starts writing; progress and the end arrive as "progress" and "finished"
/// events. Only the Pixel 2 so far.
#[tauri::command]
fn start(app: AppHandle, running: State<Running>, card: String, device: String, action: String) -> Result<(), String> {
    if device != "pixel2" {
        return Err("The Brick isn't built yet.".into());
    }
    let action = install::Action::parse(&action).ok_or("Unknown action")?;
    let image = image_path()?;
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
        let result = install::run(&card, action, &image, &mut progress, &cancel);
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
