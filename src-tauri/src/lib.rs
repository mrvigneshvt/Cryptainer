//! Cryptainer - Offline Encrypted Container Manager
//! 
//! Tauri v2 + React + TypeScript + Rust
//! 
//! This library provides:
//! - AES-256-GCM authenticated encryption
//! - Argon2id key derivation
//! - SQLite metadata storage
//! - In-memory session management with automatic key zeroization

use tauri::Manager;

pub mod error;
pub mod crypto;
pub mod storage;
pub mod vault;
pub mod session;
pub mod export;
pub mod commands;

pub use error::{CryptoError, Result};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle();
            
            // Initialize SQLite database
            let app_data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");
            
            let pool: sqlx::SqlitePool = tauri::async_runtime::block_on(async {
                storage::init_db(&app_data_dir).await.expect("Failed to initialize database")
            });
            
            app.manage(pool);
            app.manage(session::SessionStore::new());
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_container,
            commands::unlock_container,
            commands::get_file_data,
            commands::save_edits,
            commands::list_containers,
            commands::delete_container,
            commands::lock_container,
            commands::export_container,
            commands::import_container,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cryptainer");
}
