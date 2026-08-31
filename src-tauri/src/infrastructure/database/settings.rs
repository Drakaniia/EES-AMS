use super::{
    audit::{insert_audit_event, AuditEventDraft},
    rows::serialize_audit_payload,
    DbPool,
};
use crate::domain::{error::Result, models::*};
use rusqlite::{params, OptionalExtension};

/// Settings repository
pub struct SettingsRepository {
    pool: DbPool,
}

impl SettingsRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get settings
    pub fn get(&self) -> Result<Settings> {
        let conn = self.pool.get()?;
        let mut settings = conn
            .query_row(
                "SELECT id, day_start, day_end, late_after, quarter, q1_start, q1_end, q2_start, q2_end, q3_start, q3_end, attendance_mode, school_id, school_name, school_year, report_month, grade_level, section, adviser_name, school_head_name, branding_logo_path, branding_title FROM settings WHERE id = 'app'",
                [],
                |row| {
                    let attendance_mode = row.get::<_, String>(11)?;
                    Ok(Settings {
                        id: row.get(0)?,
                        day_start: row.get(1)?,
                        day_end: row.get(2)?,
                        late_after: row.get(3)?,
                        quarter: row.get(4)?,
                        q1_start: row.get(5)?,
                        q1_end: row.get(6)?,
                        q2_start: row.get(7)?,
                        q2_end: row.get(8)?,
                        q3_start: row.get(9)?,
                        q3_end: row.get(10)?,
                        attendance_mode: AttendanceMode::from_db(&attendance_mode),
                        school_id: row.get(12)?,
                        school_name: row.get(13)?,
                        school_year: row.get(14)?,
                        report_month: row.get(15)?,
                        grade_level: row.get(16)?,
                        section: row.get(17)?,
                        adviser_name: row.get(18)?,
                        school_head_name: row.get(19)?,
                        branding_logo_path: row.get(20)?,
                        branding_title: row.get::<_, Option<String>>(21)?.unwrap_or_else(|| "EES AMS".to_string()),
                    })
                },
            )
            .optional()?
            .unwrap_or_default();

        if !matches!(
            settings.quarter.as_str(),
            "1st Quarter" | "2nd Quarter" | "3rd Quarter"
        ) {
            settings.quarter = "3rd Quarter".to_string();
        }
        settings.attendance_mode = settings.attendance_mode.normalize();

        Ok(settings)
    }

    /// Update settings
    pub fn update(&self, settings: Settings) -> Result<Settings> {
        let before = self.get()?;
        let mut settings = settings;
        if !matches!(
            settings.quarter.as_str(),
            "1st Quarter" | "2nd Quarter" | "3rd Quarter"
        ) {
            settings.quarter = "3rd Quarter".to_string();
        }
        settings.attendance_mode = settings.attendance_mode.normalize();

        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        let branding_title = if settings.branding_title.trim().is_empty() {
            "EES AMS".to_string()
        } else {
            settings.branding_title.clone()
        };
        // Sanitize: remove control characters, trim whitespace
        let branding_title: String = branding_title
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>()
            .trim()
            .to_string();
        let branding_title = if branding_title.is_empty() {
            "EES AMS".to_string()
        } else {
            branding_title
        };
        let branding_title = if branding_title.len() > 40 {
            branding_title[..40].to_string()
        } else {
            branding_title
        };
        settings.branding_title = branding_title;

        transaction.execute(
            "INSERT OR REPLACE INTO settings (id, day_start, day_end, late_after, quarter, q1_start, q1_end, q2_start, q2_end, q3_start, q3_end, attendance_mode, school_id, school_name, school_year, report_month, grade_level, section, adviser_name, school_head_name, branding_logo_path, branding_title)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
            params![
                settings.id.as_str(),
                settings.day_start.as_str(),
                settings.day_end.as_str(),
                settings.late_after.as_str(),
                settings.quarter.as_str(),
                settings.q1_start.as_deref(),
                settings.q1_end.as_deref(),
                settings.q2_start.as_deref(),
                settings.q2_end.as_deref(),
                settings.q3_start.as_deref(),
                settings.q3_end.as_deref(),
                settings.attendance_mode.as_str(),
                settings.school_id.as_deref(),
                settings.school_name.as_deref(),
                settings.school_year.as_deref(),
                settings.report_month.as_deref(),
                settings.grade_level.as_deref(),
                settings.section.as_deref(),
                settings.adviser_name.as_deref(),
                settings.school_head_name.as_deref(),
                settings.branding_logo_path.as_deref(),
                settings.branding_title.as_str(),
            ],
        )?;
        let before_json = serialize_audit_payload("settings audit before payload", &before)?;
        let after_json = serialize_audit_payload("settings audit after payload", &settings)?;
        insert_audit_event(
            &transaction,
            AuditEventDraft::new(
                "settings",
                Some(settings.id.clone()),
                "update",
                "Updated global settings",
            )
            .before_json(before_json)
            .after_json(after_json),
        )?;
        transaction.commit()?;

        Ok(settings)
    }
}
