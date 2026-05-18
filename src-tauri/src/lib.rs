mod commands;
mod domain;
mod infrastructure;

use commands::{
    add_event,
    // Updater commands
    check_for_updates,
    create_class,
    create_student,
    delete_class,
    delete_event,
    delete_student,
    download_and_install,
    export_all,
    export_csv_with_folder,
    export_dtr_excel,
    export_database,
    export_json_with_folder,
    find_student_by_card,
    get_class,
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
    save_settings,
    update_class,
    update_student,
    wipe_all,
};
use infrastructure::init_db;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
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
            export_database,
            export_json_with_folder,
            export_csv_with_folder,
            export_dtr_excel,
            import_all,
            wipe_all,
            // Updater commands
            check_for_updates,
            download_and_install,
        ])
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
