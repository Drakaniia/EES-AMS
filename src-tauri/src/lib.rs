mod commands;
mod domain;
mod infrastructure;

use commands::get_server_info;
use infrastructure::{init_db, start_server, AppState};
use tauri::Manager;

const DEFAULT_PORT: u16 = 3030;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_server_info])
        .setup(|app| {
            // Setup logging
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Initialize database
            let app_dir = app
                .handle()
                .path()
                .app_data_dir()
                .expect("failed to get app data directory");
            std::fs::create_dir_all(&app_dir).expect("failed to create app data directory");

            let db_path = app_dir.join("attendance.db");
            log::info!("initializing database at {:?}", db_path);

            let pool = init_db(&db_path).expect("failed to initialize database");

            // Get local IP address
            let local_ip = local_ip_address::local_ip()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|_| "127.0.0.1".to_string());

            log::info!("local IP address: {}", local_ip);
            log::info!(
                "server will be accessible at http://{}:{}",
                local_ip,
                DEFAULT_PORT
            );

            // Start HTTP server in background
            let state = AppState::new(pool);
            tauri::async_runtime::spawn(async move {
                if let Err(e) = start_server(state, DEFAULT_PORT).await {
                    log::error!("server error: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
