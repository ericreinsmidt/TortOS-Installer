// TortOS Installer: the window, and the commands its page calls.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cards;

/// The cards in this computer and what is on each. Asks the system for its
/// disks, which takes a moment, so it runs off the window's thread.
#[tauri::command]
async fn list_cards() -> Vec<cards::Card> {
    tauri::async_runtime::spawn_blocking(cards::list).await.unwrap_or_default()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_cards])
        .run(tauri::generate_context!())
        .expect("TortOS Installer could not start");
}
