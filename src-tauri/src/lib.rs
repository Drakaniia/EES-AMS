mod backup;
mod commands;
mod domain;
mod infrastructure;
pub mod sf2;

use commands::{
    add_event,
    add_events,
    // Updater commands
    cancel_update_download,
    check_for_updates,
    choose_backup_sync_folder,
    choose_restore_backup,
    choose_restore_database_file,
    clear_audit_events,
    clear_backup_sync_folder,
    connect_google_drive_backup,
    create_backup_now,
    create_class,
    create_sf2_month_file,
    create_sf2_workbook_from_template,
    create_student,
    create_students,
    create_workbooks_backup_now,
    delete_branding_logo,
    delete_class,
    delete_event,
    delete_events,
    delete_student,
    // Read-only mark diagnostic (spec §0 A5). Writes to neither the workbook
    // nor the database; see `commands/sf2_diagnose.rs`.
    diagnose_sf2_marks,
    disconnect_google_drive_backup,
    download_update,
    export_all,
    export_csv_with_folder,
    export_database,
    export_json_with_folder,
    export_sf2_workbook,
    find_student_by_card,
    get_backup_status,
    get_branding_logo_path,
    get_class,
    // Settings commands
    get_default_logo_path,
    get_settings,
    get_sf2_export_preview,
    get_sf2_export_readiness,
    get_sf2_launch_month,
    get_sf2_month_preview,
    get_sf2_workbook_settings,
    get_student,
    get_update_status,
    // Startup self-heal (spec D6, D8, §8): one command, run from `setup` on its
    // own thread so launch is never blocked on a COM pass over the workbook.
    heal_current_month_workbook,
    import_all,
    import_sf2_attendance_from_workbook,
    import_sf2_workbook,
    install_staged_update,
    kill_all_excel_processes,
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
    open_external_url,
    open_sf2_workbook,
    pick_branding_logo,
    present_all_sf2_preview_attendance,
    reset_branding,
    restore_backup,
    save_branding_logo,
    save_settings,
    set_sf2_preview_attendance,
    sync_and_open_sf2_workbook,
    sync_sf2_attendance,
    sync_sf2_roster,
    toggle_sf2_preview_attendance,
    update_class,
    update_event,
    update_sf2_workbook_settings,
    update_student,
    upload_latest_backup_to_google_drive,
    validate_sf2_workbook_import,
    wipe_all,
    UpdateState,
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
            save_settings, // Branding commands
            save_branding_logo,
            pick_branding_logo,
            get_branding_logo_path,
            get_default_logo_path,
            delete_branding_logo,
            reset_branding,
            // Export/Import commands
            export_all,
            export_database,
            export_json_with_folder,
            export_csv_with_folder,
            import_all,
            wipe_all,
            get_backup_status,
            create_backup_now,
            create_workbooks_backup_now,
            list_backups,
            open_backup_folder,
            choose_backup_sync_folder,
            clear_backup_sync_folder,
            connect_google_drive_backup,
            disconnect_google_drive_backup,
            upload_latest_backup_to_google_drive,
            choose_restore_backup,
            choose_restore_database_file,
            restore_backup,
            validate_sf2_workbook_import,
            import_sf2_workbook,
            create_sf2_workbook_from_template,
            get_sf2_workbook_settings,
            update_sf2_workbook_settings,
            get_sf2_export_readiness,
            get_sf2_export_preview,
            // Instant month switching (spec D9, §7): three read-only commands and
            // one create. No `set_sf2_report_month` - a switch no longer writes.
            get_sf2_launch_month,
            get_sf2_month_preview,
            create_sf2_month_file,
            set_sf2_preview_attendance,
            toggle_sf2_preview_attendance,
            sync_and_open_sf2_workbook,
            sync_sf2_attendance,
            import_sf2_attendance_from_workbook,
            sync_sf2_roster,
            present_all_sf2_preview_attendance,
            heal_current_month_workbook,
            export_sf2_workbook,
            open_sf2_workbook,
            // Read-only mark diagnostic (spec §0 A5). Writes to neither the
            // workbook nor the database; see `commands/sf2_diagnose.rs`.
            diagnose_sf2_marks,
            kill_all_excel_processes,
            // Updater commands
            check_for_updates,
            get_update_status,
            download_update,
            cancel_update_download,
            install_staged_update,
            open_external_url,
        ])
        .manage(UpdateState::default())
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

            // Startup self-heal (spec D6, D8, §8.2, acceptance #15). Spawned
            // after `init_db` because it reads the month rows, and deliberately
            // not awaited: it opens Excel, and a COM pass over forty learners on
            // the startup path is a multi-second hang. The spawner claims the
            // once-per-launch latch before the thread exists, so this line is
            // idempotent even if `setup` ever runs twice.
            sf2::heal::spawn_heal_at_startup(app.handle().clone(), pool.clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
