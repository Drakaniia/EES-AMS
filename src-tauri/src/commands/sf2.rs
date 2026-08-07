use super::*;

/// Kill all running EXCEL.EXE processes so orphaned background instances
/// don't prevent the SF2 workbook from opening. Returns the count of
/// terminated processes.
#[tauri::command]
pub fn kill_all_excel_processes() -> std::result::Result<u32, String> {
    let killed = crate::sf2::excel::kill_excel_processes();
    log::info!("killed {killed} EXCEL.EXE process(es)");
    Ok(killed)
}

#[tauri::command]
pub async fn validate_sf2_workbook_import(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<Sf2ImportValidation, String> {
    let validation =
        service::validate_workbook_import(app, pool.inner().clone()).map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "sourcePath": validation.source_path.as_str(),
        "classId": validation.class_id.as_deref(),
        "className": validation.class_name.as_str(),
        "currentStudentCount": validation.current_student_count,
        "sf2LearnerCount": validation.sf2_learner_count,
        "missingFromSf2": validation.missing_from_sf2.len(),
        "missingFromCurrent": validation.missing_from_current.len(),
        "possibleNameMismatches": validation.possible_name_mismatches.len(),
        "duplicateCurrentStudents": validation.duplicate_current_students.len(),
        "duplicateSf2Learners": validation.duplicate_sf2_learners.len(),
        "missingLearnerInfo": validation.missing_learner_info.len(),
        "hasDiscrepancies": validation.has_discrepancies,
    }))?;
    record_command_audit(
        pool.inner(),
        "sf2_workbook",
        None,
        "validate",
        "Validated SF2 workbook import",
        Some(metadata_json),
    )?;
    Ok(validation)
}

#[tauri::command]
pub async fn import_sf2_workbook(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    source_path: String,
    proceed_anyway: bool,
) -> std::result::Result<Sf2ImportSummary, String> {
    let summary = service::import_workbook(
        app,
        pool.inner().clone(),
        source_path.clone(),
        proceed_anyway,
    )
    .map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "classId": summary.class_id.as_str(),
        "className": summary.class_name.as_str(),
        "selectedSourcePath": source_path.as_str(),
        "sourcePath": summary.source_path.as_str(),
        "learnersFound": summary.learners_found,
        "studentsCreated": summary.students_created,
        "studentsReused": summary.students_reused,
        "studentsUpdated": summary.students_updated,
        "datesMapped": summary.dates_mapped,
        "proceedAnyway": proceed_anyway,
    }))?;
    record_command_audit(
        pool.inner(),
        "sf2_workbook",
        Some(summary.template_id.as_str()),
        "import",
        "Imported SF2 workbook",
        Some(metadata_json),
    )?;
    Ok(summary)
}

#[tauri::command]
pub async fn create_sf2_workbook_from_template(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    draft: Sf2TemplateDraft,
) -> std::result::Result<Sf2ImportSummary, String> {
    let summary = service::create_workbook_from_template(app, pool.inner().clone(), draft)
        .map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "classId": summary.class_id.as_str(),
        "className": summary.class_name.as_str(),
        "sourcePath": summary.source_path.as_str(),
        "learnersFound": summary.learners_found,
        "studentsCreated": summary.students_created,
        "studentsReused": summary.students_reused,
        "datesMapped": summary.dates_mapped,
    }))?;
    record_command_audit(
        pool.inner(),
        "sf2_workbook",
        Some(summary.template_id.as_str()),
        "create",
        "Created SF2 workbook from template",
        Some(metadata_json),
    )?;
    Ok(summary)
}

#[tauri::command]
pub async fn get_sf2_workbook_settings(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Sf2WorkbookSettings, String> {
    let pool = pool.inner().clone();
    tokio::task::spawn_blocking(move || service::workbook_settings(pool, class_id))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_sf2_workbook_settings(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    draft: Sf2TemplateDraft,
) -> std::result::Result<Sf2ImportSummary, String> {
    let summary = service::update_workbook_settings(pool.inner().clone(), draft)
        .map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "classId": summary.class_id.as_str(),
        "className": summary.class_name.as_str(),
        "sourcePath": summary.source_path.as_str(),
        "learnersFound": summary.learners_found,
        "studentsCreated": summary.students_created,
        "studentsReused": summary.students_reused,
        "datesMapped": summary.dates_mapped,
    }))?;
    record_command_audit(
        pool.inner(),
        "sf2_workbook",
        Some(summary.template_id.as_str()),
        "update",
        "Updated SF2 workbook settings",
        Some(metadata_json),
    )?;
    Ok(summary)
}

#[tauri::command]
pub async fn set_sf2_report_month(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
    report_month: String,
) -> std::result::Result<(), String> {
    let pool = pool.inner().clone();
    tokio::task::spawn_blocking(move || {
        service::set_report_month_with_progress(&app, pool, &class_id, &report_month)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_sf2_export_readiness(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Sf2ExportReadiness, String> {
    service::export_readiness(pool.inner().clone(), class_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_sf2_export_preview(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Sf2ExportPreview, String> {
    let pool = pool.inner().clone();
    tokio::task::spawn_blocking(move || service::export_preview(pool, class_id))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Sync the latest attendance events from the database to the SF2 Excel working copy.
/// Call this before reloading the preview to ensure the working copy matches the DB.
#[tauri::command]
pub fn sync_sf2_attendance(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
) -> std::result::Result<(), String> {
    service::sync_attendance_to_sf2_workbook(pool.inner().clone(), &class_id)
        .map_err(|e| e.to_string())
}

/// Sync the class roster to the SF2 working workbook.
/// For bundled templates this re-assigns all students to dynamic row slots,
/// expanding the workbook if needed. For imported workbooks this maps new
/// students to available empty learner rows in the workbook.
/// Mark ALL mapped students as present for the current report month.
/// Creates "in" events for every absent student on every day where
/// attendance was taken. Open days (no attendance taken) are left as-is.
#[tauri::command]
pub fn present_all_sf2_preview_attendance(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
) -> std::result::Result<usize, String> {
    service::set_all_students_present(pool.inner().clone(), &class_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sync_sf2_roster(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
) -> std::result::Result<(), String> {
    service::sync_workbook_roster_for_class(pool.inner().clone(), &class_id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Lightweight toggle — only persists the DB event, no Excel I/O or preview rebuild.
/// Call this from the pre-export review grid. Click the Refresh button to reload the full preview.
#[tauri::command]
pub fn toggle_sf2_preview_attendance(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
    student_id: String,
    date: String,
    present: bool,
) -> std::result::Result<(), String> {
    service::set_preview_attendance_lightweight(
        pool.inner().clone(),
        class_id,
        student_id,
        date,
        present,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_sf2_preview_attendance(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
    student_id: String,
    date: String,
    present: bool,
) -> std::result::Result<Sf2ExportPreview, String> {
    service::set_preview_attendance(pool.inner().clone(), class_id, student_id, date, present)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_sf2_workbook(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
) -> std::result::Result<Sf2ExportResult, String> {
    let result = service::export_workbook(app, pool.inner().clone(), class_id.clone())
        .map_err(|e| e.to_string())?;
    let metadata_json = audit_metadata_json(serde_json::json!({
        "classId": class_id.as_str(),
        "outputPath": result.output_path.as_str(),
        "marksWritten": result.marks_written,
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        Some(class_id.as_str()),
        "export",
        "Exported SF2 workbook",
        Some(metadata_json),
    )?;
    Ok(result)
}

/// Sync attendance to the SF2 workbook and open it in Excel, emitting progress
/// events throughout the entire workflow so the frontend can show a determinate
/// progress bar with friendly messages.
#[tauri::command]
pub async fn sync_and_open_sf2_workbook(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: String,
) -> std::result::Result<String, String> {
    service::sync_and_open_sf2_workbook(&app, pool.inner().clone(), &class_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_sf2_workbook(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<String, String> {
    service::open_workbook(pool.inner().clone(), class_id).map_err(|e| e.to_string())
}
