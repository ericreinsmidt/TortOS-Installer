// The window only, for now: the screen from the mockup, with nothing behind
// it yet. Cards, writing and downloads come in the next steps.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("TortOS Installer could not start");
}
