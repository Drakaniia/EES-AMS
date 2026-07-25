use super::*;

// ── Export/Import Commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn export_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<ExportData, String> {
    let export_data = collect_export_data(pool.inner())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "format": "json",
        "students": export_data.students.len(),
        "classes": export_data.classes.len(),
        "events": export_data.events.len(),
        "settings": export_data.settings.len(),
        "auditEvents": export_data.audit_events.len(),
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        None,
        "export",
        "Exported application data snapshot",
        Some(metadata_json),
    )?;

    Ok(export_data)
}

#[tauri::command]
pub fn import_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    payload: ExportData,
) -> std::result::Result<(), String> {
    let ExportData {
        students,
        classes,
        events,
        settings,
        audit_events,
        exported_at,
    } = payload;
    let student_count = students.len();
    let class_count = classes.len();
    let event_count = events.len();
    let settings_count = settings.len();
    let imported_audit_count = audit_events.len();

    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let transaction = conn.transaction().map_err(|e| e.to_string())?;

    for class in classes {
        let sessions_json = serde_json::to_string(&class.sessions)
            .map_err(|e| format!("Invalid class sessions: {e}"))?;
        let days_json =
            serde_json::to_string(&class.days).map_err(|e| format!("Invalid class days: {e}"))?;

        transaction
            .execute(
                "INSERT INTO classes (id, name, room, day_start, day_end, late_after, sessions, days, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    room = excluded.room,
                    day_start = excluded.day_start,
                    day_end = excluded.day_end,
                    late_after = excluded.late_after,
                    sessions = excluded.sessions,
                    days = excluded.days,
                    created_at = excluded.created_at",
                params![
                    class.id,
                    class.name,
                    class.room,
                    class.day_start,
                    class.day_end,
                    class.late_after,
                    sessions_json,
                    days_json,
                    class.created_at.timestamp(),
                ],
            )
            .map_err(|e| e.to_string())?;
    }

    for student in students {
        transaction
            .execute(
                "INSERT INTO students (id, name, gender, card_serial, class_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    gender = excluded.gender,
                    card_serial = excluded.card_serial,
                    class_id = excluded.class_id,
                    created_at = excluded.created_at",
                params![
                    student.id.0.to_string(),
                    student.name,
                    student.gender.map(StudentGender::as_db_value),
                    student.card_serial,
                    student.class_id,
                    student.created_at.timestamp(),
                ],
            )
            .map_err(|e| e.to_string())?;
    }

    for event in events {
        let session_key = event.session_key.clone().unwrap_or_else(|| {
            let local_date = event
                .timestamp
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d");
            let class_key = event
                .class_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("unassigned");
            format!("{local_date}|{class_key}|day")
        });

        transaction
            .execute(
                "INSERT INTO events (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    student_id = excluded.student_id,
                    class_id = excluded.class_id,
                    event_type = excluded.event_type,
                    timestamp = excluded.timestamp,
                    note = excluded.note,
                    session_key = excluded.session_key,
                    override_reason = excluded.override_reason,
                    updated_at = excluded.updated_at",
                params![
                    event.id.0.to_string(),
                    event.student_id.0.to_string(),
                    event.class_id,
                    "in",
                    event.timestamp.timestamp(),
                    event.note,
                    session_key,
                    event.override_reason,
                    event.updated_at.map(|timestamp| timestamp.timestamp()),
                ],
            )
            .map_err(|e| e.to_string())?;
    }

    if let Some(mut settings) = settings.into_iter().next() {
        if !matches!(
            settings.quarter.as_str(),
            "1st Quarter" | "2nd Quarter" | "3rd Quarter"
        ) {
            settings.quarter = "3rd Quarter".to_string();
        }
        settings.attendance_mode = settings.attendance_mode.normalize();

        transaction
            .execute(
                "INSERT INTO settings (id, day_start, day_end, late_after, quarter, q1_start, q1_end, q2_start, q2_end, q3_start, q3_end, attendance_mode, school_id, school_name, school_year, report_month, grade_level, section, adviser_name, school_head_name)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
                 ON CONFLICT(id) DO UPDATE SET
                    day_start = excluded.day_start,
                    day_end = excluded.day_end,
                    late_after = excluded.late_after,
                    quarter = excluded.quarter,
                    q1_start = excluded.q1_start,
                    q1_end = excluded.q1_end,
                    q2_start = excluded.q2_start,
                    q2_end = excluded.q2_end,
                    q3_start = excluded.q3_start,
                    q3_end = excluded.q3_end,
                    attendance_mode = excluded.attendance_mode,
                    school_id = excluded.school_id,
                    school_name = excluded.school_name,
                    school_year = excluded.school_year,
                    report_month = excluded.report_month,
                    grade_level = excluded.grade_level,
                    section = excluded.section,
                    adviser_name = excluded.adviser_name,
                    school_head_name = excluded.school_head_name",
                params![
                    settings.id,
                    settings.day_start,
                    settings.day_end,
                    settings.late_after,
                    settings.quarter,
                    settings.q1_start,
                    settings.q1_end,
                    settings.q2_start,
                    settings.q2_end,
                    settings.q3_start,
                    settings.q3_end,
                    settings.attendance_mode.as_str(),
                    settings.school_id,
                    settings.school_name,
                    settings.school_year,
                    settings.report_month,
                    settings.grade_level,
                    settings.section,
                    settings.adviser_name,
                    settings.school_head_name,
                ],
            )
            .map_err(|e| e.to_string())?;
    }

    for audit_event in &audit_events {
        insert_imported_audit_event(&transaction, audit_event)?;
    }

    let metadata_json = audit_metadata_json(serde_json::json!({
        "sourceExportedAt": exported_at,
        "students": student_count,
        "classes": class_count,
        "events": event_count,
        "settings": settings_count,
        "importedAuditEvents": imported_audit_count,
    }))?;
    record_audit_event(
        &transaction,
        AuditEventInput {
            entity_type: "data_import",
            entity_id: None,
            action: "import",
            summary: "Imported JSON backup merge",
            before_json: None,
            after_json: None,
            metadata_json: Some(metadata_json),
        },
    )
    .map_err(|e| e.to_string())?;

    transaction.commit().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wipe_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let transaction = conn.transaction().map_err(|e| e.to_string())?;
    let student_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM students", [], |row| row.get(0))
        .unwrap_or(0);
    let class_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM classes", [], |row| row.get(0))
        .unwrap_or(0);
    let event_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
        .unwrap_or(0);
    let settings_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
        .unwrap_or(0);
    let attendance_audit_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM attendance_event_audit", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);
    let sf2_template_count: i64 = transaction
        .query_row("SELECT COUNT(*) FROM sf2_templates", [], |row| row.get(0))
        .unwrap_or(0);

    // Clear all tables
    transaction
        .execute("DELETE FROM attendance_event_audit", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM sf2_student_mappings", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM sf2_date_mappings", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM attendance_day_status", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM sf2_templates", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM events", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM students", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM classes", [])
        .map_err(|e| e.to_string())?;
    transaction
        .execute("DELETE FROM settings", [])
        .map_err(|e| e.to_string())?;

    // Re-insert default settings
    transaction.execute(
        "INSERT OR IGNORE INTO settings (id, day_start, day_end, late_after, quarter, attendance_mode) VALUES ('app', '08:30', '15:30', '08:45', '1st Quarter', 'manual')",
        []
    ).map_err(|e| e.to_string())?;

    let metadata_json = audit_metadata_json(serde_json::json!({
        "students": student_count,
        "classes": class_count,
        "events": event_count,
        "settings": settings_count,
        "attendanceAuditEvents": attendance_audit_count,
        "sf2Templates": sf2_template_count,
    }))?;
    record_audit_event(
        &transaction,
        AuditEventInput {
            entity_type: "database",
            entity_id: None,
            action: "wipe",
            summary: "Wiped all application data",
            before_json: None,
            after_json: None,
            metadata_json: Some(metadata_json),
        },
    )
    .map_err(|e| e.to_string())?;

    transaction.commit().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_database(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<String, String> {
    let file_path_buf = save_file_dialog(
        &app,
        "SQLite Database",
        &["db"],
        format!(
            "attendance-backup-{}.db",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ),
    )?;

    backup_service::backup_database_to_path(pool.inner(), &file_path_buf)
        .map_err(|e| e.to_string())?;

    let metadata_json = audit_metadata_json(serde_json::json!({
        "format": "sqlite",
        "path": file_path_buf.to_string_lossy(),
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        None,
        "export",
        "Exported SQLite database",
        Some(metadata_json),
    )?;

    Ok(file_path_buf.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_json_with_folder(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<String, String> {
    // Get export data
    let export_data = collect_export_data(pool.inner())?;

    let file_path_buf = save_file_dialog(
        &app,
        "JSON Files",
        &["json"],
        format!(
            "attendance-backup-{}.json",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ),
    )?;

    // Write JSON data to file
    let json_content = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize data: {}", e))?;

    fs::write(&file_path_buf, json_content).map_err(|e| format!("Failed to write file: {}", e))?;

    let metadata_json = audit_metadata_json(serde_json::json!({
        "format": "json",
        "path": file_path_buf.to_string_lossy(),
        "students": export_data.students.len(),
        "classes": export_data.classes.len(),
        "events": export_data.events.len(),
        "settings": export_data.settings.len(),
        "auditEvents": export_data.audit_events.len(),
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        None,
        "export",
        "Exported JSON backup",
        Some(metadata_json),
    )?;

    Ok(file_path_buf.to_string_lossy().to_string())
}
