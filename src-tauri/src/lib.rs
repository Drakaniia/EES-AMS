mod backup;
mod commands;
mod domain;
mod infrastructure;
pub mod sf2;

use commands::{
    add_event,
    add_events,
    // Updater commands
    check_for_updates,
    choose_backup_sync_folder,
    choose_restore_backup,
    clear_audit_events,
    clear_backup_sync_folder,
    connect_google_drive_backup,
    create_backup_now,
    create_class,
    create_sf2_workbook_from_template,
    create_student,
    create_students,
    delete_class,
    delete_event,
    delete_events,
    delete_student,
    disconnect_google_drive_backup,
    download_and_install,
    export_all,
    export_csv_with_folder,
    export_database,
    export_json_with_folder,
    export_sf2_workbook,
    find_student_by_card,
    get_backup_status,
    get_class,
    // Settings commands
    get_settings,
    get_sf2_export_preview,
    get_sf2_export_readiness,
    get_sf2_workbook_settings,
    get_student,
    import_all,
    import_sf2_workbook,
    last_event_for_student,
    list_attendance_audit,
    list_audit_events,
    list_backups,
    // Class commands
    list_classes,
    // Event commands
    list_events,
    list_events_for_date,
    list_events_for_student,
    // Student commands
    list_students,
    open_backup_folder,
    open_sf2_workbook,
    restore_backup,
    save_settings,
    set_sf2_preview_attendance,
    set_sf2_report_month,
    toggle_sf2_preview_attendance,
    update_class,
    update_event,
    sync_and_open_sf2_workbook,
    sync_sf2_attendance,
    sync_sf2_roster,
    update_sf2_workbook_settings,
    update_student,
    upload_latest_backup_to_google_drive,
    validate_sf2_workbook_import,
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
            create_students,
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
            list_events_for_date,
            list_events_for_student,
            last_event_for_student,
            add_event,
            add_events,
            update_event,
            delete_event,
            delete_events,
            list_attendance_audit,
            list_audit_events,
            clear_audit_events,
            // Settings commands
            get_settings,
            save_settings,
            // Export/Import commands
            export_all,
            export_database,
            export_json_with_folder,
            export_csv_with_folder,
            import_all,
            wipe_all,
            get_backup_status,
            create_backup_now,
            list_backups,
            open_backup_folder,
            choose_backup_sync_folder,
            clear_backup_sync_folder,
            connect_google_drive_backup,
            disconnect_google_drive_backup,
            upload_latest_backup_to_google_drive,
            choose_restore_backup,
            restore_backup,
            validate_sf2_workbook_import,
            import_sf2_workbook,
            create_sf2_workbook_from_template,
            get_sf2_workbook_settings,
            update_sf2_workbook_settings,
            get_sf2_export_readiness,
            get_sf2_export_preview,
            set_sf2_preview_attendance,
            set_sf2_report_month,
            toggle_sf2_preview_attendance,
            sync_and_open_sf2_workbook,
            sync_sf2_attendance,
            sync_sf2_roster,
            export_sf2_workbook,
            open_sf2_workbook,
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
            backup::service::spawn_backup_scheduler(pool.clone(), app_dir.clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
