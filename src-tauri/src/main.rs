#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Mobile entry point for Android/iOS
#[cfg_attr(mobile, tauri::mobile_entry_point)]

fn main() {
    cryptainer_lib::run();
}
