/// Tauri commands
use crate::domain::models::*;
use crate::infrastructure::database::{
    ClassRepository, EventRepository, SettingsRepository, StudentRepository,
};
use chrono::{Datelike, Timelike};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rust_xlsxwriter::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub async fn export_dtr_excel(
    app: tauri::AppHandle,
    student: Student,
    _class: Option<Class>,
    events: Vec<AttendanceEvent>,
    month: u32,
    year: i32,
) -> std::result::Result<String, String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Set Column Widths
    worksheet
        .set_column_width(0, 5.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(1, 6.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(2, 6.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(3, 6.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(4, 6.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(5, 6.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(6, 6.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(7, 9.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(8, 2.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(9, 2.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(10, 5.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(11, 6.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(12, 6.0)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(13, 6.5)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(14, 6.2)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(15, 6.5)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(16, 7.2)
        .map_err(|e| e.to_string())?;
    worksheet
        .set_column_width(17, 9.2)
        .map_err(|e| e.to_string())?;

    // Formats
    let fmt_header = Format::new()
        .set_font_size(11.0)
        .set_bold()
        .set_align(FormatAlign::Center);
    let fmt_italic = Format::new()
        .set_font_size(8.0)
        .set_italic()
        .set_align(FormatAlign::Left);
    let fmt_small_italic = Format::new()
        .set_font_size(10.0)
        .set_italic()
        .set_align(FormatAlign::Center);
    let fmt_name = Format::new()
        .set_font_size(11.0)
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_border_bottom(FormatBorder::Thin);
    let fmt_border_thin = Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let fmt_border_thin_left = Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Left);
    let fmt_day = Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let fmt_month = Format::new()
        .set_align(FormatAlign::Center)
        .set_border_bottom(FormatBorder::Thin);
    let fmt_footer = Format::new()
        .set_font_size(9.0)
        .set_align(FormatAlign::Left);

    let month_name = match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    };

    // Helper to draw one DTR side
    let draw_side = |ws: &mut Worksheet, col_offset: u16| -> Result<(), XlsxError> {
        // Header
        ws.write_with_format(0, col_offset, "CSC Form 48", &fmt_italic)?;
        ws.merge_range(
            1,
            col_offset,
            1,
            col_offset + 6,
            "DAILY TIME RECORD",
            &fmt_header,
        )?;

        ws.merge_range(3, col_offset, 3, col_offset + 7, &student.name, &fmt_name)?;
        ws.merge_range(
            4,
            col_offset,
            4,
            col_offset + 7,
            "(Name)",
            &fmt_small_italic,
        )?;

        ws.write(6, col_offset, "For the month of")?;
        ws.merge_range(
            6,
            col_offset + 1,
            6,
            col_offset + 5,
            format!("{} {}", month_name, year).as_str(),
            &fmt_month,
        )?;

        ws.write(7, col_offset, "Official hours for Arrival")?;
        ws.merge_range(
            7,
            col_offset + 4,
            7,
            col_offset + 7,
            "_______________________",
            &Format::new(),
        )?;
        ws.write(8, col_offset, "and Departure")?;
        ws.merge_range(
            8,
            col_offset + 4,
            8,
            col_offset + 7,
            "_______________________",
            &Format::new(),
        )?;

        // Table Headers
        ws.merge_range(11, col_offset, 12, col_offset, "Day", &fmt_border_thin)?;
        ws.merge_range(
            11,
            col_offset + 1,
            11,
            col_offset + 2,
            "A.M.",
            &fmt_border_thin,
        )?;
        ws.merge_range(
            11,
            col_offset + 3,
            11,
            col_offset + 4,
            "P.M.",
            &fmt_border_thin,
        )?;
        ws.merge_range(
            11,
            col_offset + 5,
            11,
            col_offset + 6,
            "Undertime",
            &fmt_border_thin,
        )?;

        ws.write_with_format(12, col_offset + 1, "Arrival", &fmt_border_thin)?;
        ws.write_with_format(12, col_offset + 2, "Departure", &fmt_border_thin)?;
        ws.write_with_format(12, col_offset + 3, "Arrival", &fmt_border_thin)?;
        ws.write_with_format(12, col_offset + 4, "Departure", &fmt_border_thin)?;
        ws.write_with_format(12, col_offset + 5, "Hours", &fmt_border_thin)?;
        ws.write_with_format(12, col_offset + 6, "Minutes", &fmt_border_thin)?;

        // Data Rows
        let days_in_month = if month == 2 {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        } else if [4, 6, 9, 11].contains(&month) {
            30
        } else {
            31
        };

        let mut events_map: HashMap<u32, Vec<&AttendanceEvent>> = HashMap::new();
        for event in &events {
            let dt = event.timestamp.with_timezone(&chrono::Local);
            if dt.month() == month && dt.year() == year {
                events_map.entry(dt.day()).or_default().push(event);
            }
        }

        for day in 1..=31 {
            let row = 12 + day as u32;
            ws.write_with_format(row, col_offset, day as f64, &fmt_day)?;

            if day <= days_in_month {
                if let Some(day_events) = events_map.get(&(day as u32)) {
                    let mut sorted_events = day_events.clone();
                    sorted_events.sort_by_key(|e| e.timestamp);

                    let mut am_in: Option<String> = None;
                    let mut am_out: Option<String> = None;
                    let mut pm_in: Option<String> = None;
                    let mut pm_out: Option<String> = None;

                    for event in sorted_events {
                        let dt = event.timestamp.with_timezone(&chrono::Local);
                        let time = dt.format("%H:%M").to_string();
                        let hour = dt.hour();

                        if event.event_type == AttendanceType::In {
                            if hour < 12 {
                                am_in = Some(time);
                            } else {
                                pm_in = Some(time);
                            }
                        } else {
                            if hour < 13 {
                                am_out = Some(time);
                            } else {
                                pm_out = Some(time);
                            }
                        }
                    }

                    ws.write_with_format(
                        row,
                        col_offset + 1,
                        am_in.unwrap_or_default().as_str(),
                        &fmt_border_thin,
                    )?;
                    ws.write_with_format(
                        row,
                        col_offset + 2,
                        am_out.unwrap_or_default().as_str(),
                        &fmt_border_thin,
                    )?;
                    ws.write_with_format(
                        row,
                        col_offset + 3,
                        pm_in.unwrap_or_default().as_str(),
                        &fmt_border_thin,
                    )?;
                    ws.write_with_format(
                        row,
                        col_offset + 4,
                        pm_out.unwrap_or_default().as_str(),
                        &fmt_border_thin,
                    )?;
                } else {
                    for c in 1..=6 {
                        ws.write_with_format(row, col_offset + c as u16, "", &fmt_border_thin)?;
                    }
                }
            } else {
                for c in 1..=6 {
                    ws.write_with_format(row, col_offset + c as u16, "", &fmt_border_thin)?;
                }
            }
            ws.write_with_format(row, col_offset + 5, "", &fmt_border_thin)?;
            ws.write_with_format(row, col_offset + 6, "", &fmt_border_thin)?;
        }

        // Footer
        let footer_start_row = 44;
        ws.merge_range(
            footer_start_row,
            col_offset,
            footer_start_row,
            col_offset + 4,
            "TOTAL",
            &fmt_border_thin_left,
        )?;
        ws.write_with_format(footer_start_row, col_offset + 5, "", &fmt_border_thin)?;
        ws.write_with_format(footer_start_row, col_offset + 6, "", &fmt_border_thin)?;

        ws.merge_range(
            46,
            col_offset,
            46,
            col_offset + 7,
            "I certify on my honor that the above is a true and correct",
            &fmt_footer,
        )?;
        ws.merge_range(
            47,
            col_offset,
            47,
            col_offset + 7,
            "report of the hours of work performed, record of which was made",
            &fmt_footer,
        )?;
        ws.merge_range(
            48,
            col_offset,
            48,
            col_offset + 7,
            "daily at the time of arrival and departure from office.",
            &fmt_footer,
        )?;

        ws.merge_range(
            52,
            col_offset,
            52,
            col_offset + 7,
            "____________________________________",
            &fmt_header,
        )?;
        ws.merge_range(
            53,
            col_offset,
            53,
            col_offset + 7,
            "(Signature)",
            &fmt_small_italic,
        )?;

        ws.merge_range(
            55,
            col_offset,
            55,
            col_offset + 7,
            "Verified as to the prescribed office hours:",
            &fmt_footer,
        )?;
        ws.merge_range(
            58,
            col_offset,
            58,
            col_offset + 7,
            "____________________________________",
            &fmt_header,
        )?;
        ws.merge_range(
            59,
            col_offset,
            59,
            col_offset + 7,
            "In-Charge",
            &fmt_small_italic,
        )?;

        Ok(())
    };

    draw_side(worksheet, 0).map_err(|e| e.to_string())?; // Left side (A-H)
    draw_side(worksheet, 10).map_err(|e| e.to_string())?; // Right side (K-R)

    let buffer = workbook.save_to_buffer().map_err(|e| e.to_string())?;

    // Show save dialog
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Excel Files", &["xlsx"])
        .set_file_name(format!(
            "DTR-{}-{}-{}.xlsx",
            student.name.replace(" ", "_"),
            month_name,
            year
        ))
        .save_file(move |result| tx.send(result).unwrap());

    let file_path = rx
        .recv()
        .map_err(|e| format!("Failed to receive file path: {}", e))?
        .ok_or_else(|| "User cancelled save dialog".to_string())?;

    let file_path_buf = match file_path {
        tauri_plugin_dialog::FilePath::Path(path) => path,
        _ => return Err("Unsupported file path".to_string()),
    };

    fs::write(&file_path_buf, buffer).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(file_path_buf.to_string_lossy().to_string())
}

// ── Student Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn list_students(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    class_id: Option<String>,
) -> std::result::Result<Vec<Student>, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.list_by_class(class_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<Student, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    repo.get(student_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn find_student_by_card(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    serial: String,
) -> std::result::Result<Option<Student>, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.find_by_card(&serial).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    req: CreateStudentRequest,
) -> std::result::Result<Student, String> {
    let repo = StudentRepository::new(pool.inner().clone());
    repo.create(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
    req: UpdateStudentRequest,
) -> std::result::Result<Student, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    repo.update(student_id, req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<(), String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = StudentRepository::new(pool.inner().clone());
    repo.delete(student_id).map_err(|e| e.to_string())
}

// ── Class Commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_classes(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<Vec<Class>, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_class(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<Option<Class>, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.get(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_class(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    req: CreateClassRequest,
) -> std::result::Result<Class, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.create(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_class(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
    req: UpdateClassRequest,
) -> std::result::Result<Class, String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.update(&id, req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_class(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<(), String> {
    let repo = ClassRepository::new(pool.inner().clone());
    repo.delete(&id).map_err(|e| e.to_string())
}

// ── Event Commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_events(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<Vec<AttendanceEvent>, String> {
    let repo = EventRepository::new(pool.inner().clone());
    repo.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_events_for_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    student_id: String,
) -> std::result::Result<Vec<AttendanceEvent>, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&student_id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.list_for_student(student_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn last_event_for_student(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    student_id: String,
) -> std::result::Result<Option<AttendanceEvent>, String> {
    let student_id = StudentId(uuid::Uuid::parse_str(&student_id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.last_for_student(student_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_event(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    req: CreateEventRequest,
) -> std::result::Result<AttendanceEvent, String> {
    let repo = EventRepository::new(pool.inner().clone());
    repo.create(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_event(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    id: String,
) -> std::result::Result<(), String> {
    let event_id = EventId(uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?);
    let repo = EventRepository::new(pool.inner().clone());
    repo.delete(event_id).map_err(|e| e.to_string())
}

// ── Settings Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn get_settings(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<Settings, String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.get().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    settings: Settings,
) -> std::result::Result<Settings, String> {
    let repo = SettingsRepository::new(pool.inner().clone());
    repo.update(settings).map_err(|e| e.to_string())
}

// ── Export/Import Commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn export_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<ExportData, String> {
    let student_repo = StudentRepository::new(pool.inner().clone());
    let class_repo = ClassRepository::new(pool.inner().clone());
    let event_repo = EventRepository::new(pool.inner().clone());
    let settings_repo = SettingsRepository::new(pool.inner().clone());

    let students = student_repo.list().map_err(|e| e.to_string())?;
    let classes = class_repo.list().map_err(|e| e.to_string())?;
    let events = event_repo.list().map_err(|e| e.to_string())?;
    let settings = vec![settings_repo.get().map_err(|e| e.to_string())?];

    Ok(ExportData {
        students,
        classes,
        events,
        settings,
        exported_at: chrono::Utc::now().timestamp(),
    })
}

#[tauri::command]
pub fn import_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
    payload: ExportData,
) -> std::result::Result<(), String> {
    let student_repo = StudentRepository::new(pool.inner().clone());
    let class_repo = ClassRepository::new(pool.inner().clone());
    let event_repo = EventRepository::new(pool.inner().clone());
    let settings_repo = SettingsRepository::new(pool.inner().clone());

    // Import classes first (students may reference them)
    for class in payload.classes {
        let req = CreateClassRequest {
            name: class.name,
            room: class.room,
            day_start: class.day_start,
            day_end: class.day_end,
            late_after: class.late_after,
            sessions: class.sessions,
            days: class.days,
        };
        class_repo.create(req).map_err(|e| e.to_string())?;
    }

    // Import students
    for student in payload.students {
        let req = CreateStudentRequest {
            name: student.name,
            student_number: student.student_number,
            card_serial: student.card_serial,
            class_id: student.class_id,
        };
        student_repo.create(req).map_err(|e| e.to_string())?;
    }

    // Import events
    for event in payload.events {
        let req = CreateEventRequest {
            student_id: event.student_id,
            class_id: event.class_id,
            event_type: event.event_type,
            note: event.note,
        };
        event_repo.create(req).map_err(|e| e.to_string())?;
    }

    // Import settings (only the first one)
    if let Some(settings) = payload.settings.into_iter().next() {
        settings_repo.update(settings).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn wipe_all(
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<(), String> {
    let conn = pool.get().map_err(|e| e.to_string())?;

    // Clear all tables
    conn.execute("DELETE FROM events", [])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM students", [])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM classes", [])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM settings", [])
        .map_err(|e| e.to_string())?;

    // Re-insert default settings
    conn.execute(
        "INSERT OR IGNORE INTO settings (id, day_start, day_end, late_after) VALUES ('app', '08:30', '15:30', '08:45')",
        []
    ).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn export_database(
    app: tauri::AppHandle,
    _pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<String, String> {
    // Get database path
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let db_path = app_dir.join("attendance.db");

    if !db_path.exists() {
        return Err("Database file not found".to_string());
    }

    // Show save dialog to let user choose location and filename
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("SQLite Database", &["db"])
        .set_file_name(format!(
            "attendance-backup-{}.db",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
        .save_file(move |result| tx.send(result).unwrap());

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

    // Copy database file to chosen location
    fs::copy(&db_path, &file_path_buf).map_err(|e| format!("Failed to copy database: {}", e))?;

    Ok(file_path_buf.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_json_with_folder(
    app: tauri::AppHandle,
    pool: tauri::State<'_, Pool<SqliteConnectionManager>>,
) -> std::result::Result<String, String> {
    // Get export data
    let export_data = export_all(pool)?;

    // Show save dialog to let user choose location and filename
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("JSON Files", &["json"])
        .set_file_name(format!(
            "attendance-backup-{}.json",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ))
        .save_file(move |result| tx.send(result).unwrap());

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

    // Write JSON data to file
    let json_content = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize data: {}", e))?;

    fs::write(&file_path_buf, json_content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(file_path_buf.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_csv_with_folder(
    app: tauri::AppHandle,
    events: Vec<AttendanceEvent>,
    students: Vec<Student>,
    classes: Vec<Class>,
    _global_late_after: String,
) -> std::result::Result<String, String> {
    // Import CSV generation function from frontend logic
    // We need to recreate the CSV logic here since it's in the frontend
    let mut csv_content = String::new();

    // Header
    csv_content.push_str("Date,Class,Room,Student #,Name,Check-in,Check-out,Hours,Late\n");

    // Group events by student and date
    use std::collections::HashMap;
    let mut groups: HashMap<String, (Student, Vec<AttendanceEvent>)> = HashMap::new();

    let student_map: HashMap<StudentId, Student> =
        students.into_iter().map(|s| (s.id, s)).collect();

    let class_map: HashMap<String, Class> =
        classes.into_iter().map(|c| (c.id.clone(), c)).collect();

    for event in events {
        if let Some(student) = student_map.get(&event.student_id) {
            let date = event.timestamp.format("%Y-%m-%d").to_string();
            let key = format!("{}|{}", event.student_id, date);

            let entry = groups.entry(key).or_insert((student.clone(), Vec::new()));
            entry.1.push(event);
        }
    }

    // Generate CSV rows
    for (student, events) in groups.values_mut() {
        events.sort_by_key(|a| a.timestamp);

        let mut check_in_time: Option<String> = None;
        let mut check_out_time: Option<String> = None;
        let mut duration_hours = String::new();
        let mut is_late = String::new();

        for event in &*events {
            let time_str = event.timestamp.format("%H:%M").to_string();

            if event.event_type == AttendanceType::In {
                if check_in_time.is_none() || time_str < *check_in_time.as_ref().unwrap() {
                    check_in_time = Some(time_str.clone());

                    // Check if late
                    if let Some(class) = student.class_id.as_ref().and_then(|id| class_map.get(id))
                    {
                        let event_time = event
                            .timestamp
                            .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());

                        // Find matching session or use default
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
                            .ok_or("Invalid time")?
                            .and_utc();
                        if event_time > late_time {
                            is_late = "Yes".to_string();
                        } else {
                            is_late = "No".to_string();
                        }
                    }
                }
            } else if event.event_type == AttendanceType::Out {
                check_out_time = Some(time_str.clone());
            }
        }

        // Calculate duration
        if let (Some(check_in), Some(check_out)) = (&check_in_time, &check_out_time) {
            if let (Ok(in_time), Ok(out_time)) = (
                chrono::NaiveTime::parse_from_str(check_in, "%H:%M"),
                chrono::NaiveTime::parse_from_str(check_out, "%H:%M"),
            ) {
                let duration = out_time.signed_duration_since(in_time);
                duration_hours = format!("{:.2}", duration.num_seconds() as f64 / 3600.0);
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
            .map(|e| e.timestamp.format("%Y-%m-%d").to_string())
            .unwrap_or_default();

        csv_content.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            date,
            class_name,
            room_name,
            student.student_number,
            student.name,
            check_in_time.unwrap_or_default(),
            check_out_time.unwrap_or_default(),
            duration_hours,
            is_late
        ));
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
        .save_file(move |result| tx.send(result).unwrap());

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

    Ok(file_path_buf.to_string_lossy().to_string())
}

// ── Updater Commands ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub current_version: String,
}

#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current_version = app.package_info().version.to_string();

    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(update) => Ok(UpdateInfo {
            available: true,
            version: Some(update.version.clone()),
            notes: update.body.clone(),
            pub_date: update.date.map(|d| d.to_string()),
            current_version,
        }),
        None => Ok(UpdateInfo {
            available: false,
            version: None,
            notes: None,
            pub_date: None,
            current_version,
        }),
    }
}

#[tauri::command]
pub async fn download_and_install(app: tauri::AppHandle) -> Result<String, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update available".to_string())?;

    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|e| e.to_string())?;

    Ok("Update installed. The app will restart shortly.".to_string())
}
