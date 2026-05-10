mod commands;
mod domain;
mod infrastructure;

use commands::{
    add_event,
    check_nfc_reader,
    create_class,
    create_student,
    delete_class,
    delete_event,
    delete_student,
    // Export/Import commands
    export_all,
    find_student_by_card,
    get_class,
    get_server_info,
    // Settings commands
    get_settings,
    get_student,
    import_all,
    last_event_for_student,
    // Class commands
    list_classes,
    // Event commands
    list_events,
    list_events_for_student,
    // Student commands
    list_students,
    read_nfc_card,
    save_settings,
    start_nfc_scanning,
    stop_nfc_scanning,
    update_class,
    update_student,
    wipe_all,
};
use infrastructure::{init_db, start_server, AppState};
use tauri::Manager;

const DEFAULT_PORT: u16 = 3030;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_server_info,
            check_nfc_reader,
            start_nfc_scanning,
            stop_nfc_scanning,
            read_nfc_card,
            // Student commands
            list_students,
            get_student,
            find_student_by_card,
            create_student,
            update_student,
            delete_student,
            // Class commands
            list_classes,
            get_class,
            create_class,
            update_class,
            delete_class,
            // Event commands
            list_events,
            list_events_for_student,
            last_event_for_student,
            add_event,
            delete_event,
            // Settings commands
            get_settings,
            save_settings,
            // Export/Import commands
            export_all,
            import_all,
            wipe_all,
        ])
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

            // Add database pool to Tauri state
            app.manage(pool.clone());

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
