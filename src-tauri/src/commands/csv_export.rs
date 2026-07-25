use super::*;

// ── CSV Export Commands ────────────────────────────────────────────────────

/// Export attendance events to a CSV file selected by the user.
#[tauri::command]
pub async fn export_csv_with_folder(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    events: Vec<AttendanceEvent>,
    students: Vec<Student>,
    classes: Vec<Class>,
    _global_late_after: String,
) -> std::result::Result<String, String> {
    let event_count = events.len();
    let student_count = students.len();
    let class_count = classes.len();
    let mut csv_content = String::new();

    // Header
    csv_content.push_str("Date,Class,Room,Name,IN,Late\n");

    // Group events by student and date
    use std::collections::HashMap;
    let mut groups: HashMap<String, (Student, Vec<AttendanceEvent>)> = HashMap::new();

    let student_map: HashMap<StudentId, Student> =
        students.into_iter().map(|s| (s.id, s)).collect();

    let class_map: HashMap<String, Class> =
        classes.into_iter().map(|c| (c.id.clone(), c)).collect();

    for event in events {
        if let Some(student) = student_map.get(&event.student_id) {
            let date = event
                .timestamp
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d")
                .to_string();
            let key = format!("{}|{}", event.student_id, date);

            let entry = groups.entry(key).or_insert((student.clone(), Vec::new()));
            entry.1.push(event);
        }
    }

    // Generate CSV rows
    for (student, events) in groups.values_mut() {
        events.sort_by_key(|a| a.timestamp);

        let mut check_in_time: Option<String> = None;
        let mut is_late = String::new();

        for event in &*events {
            let event_time = event.timestamp.with_timezone(&chrono::Local);
            let time_str = event_time.format("%H:%M").to_string();

            let is_earliest_check_in = match &check_in_time {
                Some(current_check_in) => time_str < *current_check_in,
                None => true,
            };

            if is_earliest_check_in {
                check_in_time = Some(time_str.clone());

                // Check if late
                if let Some(class) = student.class_id.as_ref().and_then(|id| class_map.get(id)) {
                    let mut late_after = &class.late_after;
                    let time_str = event_time.format("%H:%M").to_string();

                    for session in &class.sessions {
                        if time_str >= session.start_time && time_str <= session.end_time {
                            late_after = &session.late_after;
                            break;
                        }
                    }

                    let parts: Vec<&str> = late_after.split(':').collect();
                    let [h, m] = [
                        parts
                            .first()
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0),
                        parts
                            .get(1)
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0),
                    ];
                    let late_time = event_time
                        .date_naive()
                        .and_hms_opt(h, m, 0)
                        .and_then(|time| time.and_local_timezone(chrono::Local).earliest())
                        .ok_or("Invalid time")?;
                    if event_time > late_time {
                        is_late = "Yes".to_string();
                    } else {
                        is_late = "No".to_string();
                    }
                }
            }
        }

        let class_name = student
            .class_id
            .as_ref()
            .and_then(|id| class_map.get(id))
            .map(|c| c.name.as_str())
            .unwrap_or("Unknown");

        let room_name = student
            .class_id
            .as_ref()
            .and_then(|id| class_map.get(id))
            .and_then(|c| c.room.as_deref())
            .unwrap_or("N/A");

        let date = events
            .first()
            .map(|e| {
                e.timestamp
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d")
                    .to_string()
            })
            .unwrap_or_default();

        push_csv_row(
            &mut csv_content,
            &[
                date,
                class_name.to_string(),
                room_name.to_string(),
                student.name.clone(),
                check_in_time.unwrap_or_default(),
                is_late,
            ],
        );
    }

    // Show save dialog to let user choose location and filename
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("CSV Files", &["csv"])
        .set_file_name(format!(
            "attendance-records-{}.csv",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
        .save_file(move |result| {
            let _ = tx.send(result);
        });

    let file_path = rx
        .recv()
        .map_err(|e| format!("Failed to receive file path: {}", e))?
        .ok_or_else(|| "User cancelled save dialog".to_string())?;

    // Convert FilePath to PathBuf
    let file_path_buf = match file_path {
        tauri_plugin_dialog::FilePath::Path(path) => path,
        tauri_plugin_dialog::FilePath::Url(url) => {
            return Err(format!("URL file paths not supported: {}", url));
        }
    };

    // Write CSV data to file
    fs::write(&file_path_buf, csv_content).map_err(|e| format!("Failed to write file: {}", e))?;

    let metadata_json = audit_metadata_json(serde_json::json!({
        "format": "csv",
        "path": file_path_buf.to_string_lossy(),
        "events": event_count,
        "students": student_count,
        "classes": class_count,
    }))?;
    record_command_audit(
        pool.inner(),
        "data_export",
        None,
        "export",
        "Exported attendance CSV",
        Some(metadata_json),
    )?;

    Ok(file_path_buf.to_string_lossy().to_string())
}

// ── CSV Formatting Helpers ─────────────────────────────────────────────────

fn push_csv_row(output: &mut String, fields: &[String]) {
    let row = fields
        .iter()
        .map(|field| escape_csv_field(field))
        .collect::<Vec<_>>()
        .join(",");
    output.push_str(&row);
    output.push('\n');
}

fn escape_csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
