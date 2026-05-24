use crate::domain::error::Result;
use crate::infrastructure::database::DbPool;
use crate::sf2::models::{
    Sf2DateMappingRecord, Sf2StudentMappingRecord, Sf2TemplateRecord, Sf2TemplateSummary,
};
use rusqlite::{params, OptionalExtension};

const FIND_TEMPLATE_SQL: &str = include_str!("../../sql/sf2/find_template.sql");
const UPSERT_TEMPLATE_SQL: &str = include_str!("../../sql/sf2/upsert_template.sql");
const UPDATE_TEMPLATE_SQL: &str = include_str!("../../sql/sf2/update_template.sql");
const DELETE_STUDENT_MAPPINGS_SQL: &str = include_str!("../../sql/sf2/delete_student_mappings.sql");
const DELETE_DATE_MAPPINGS_SQL: &str = include_str!("../../sql/sf2/delete_date_mappings.sql");
const INSERT_STUDENT_MAPPING_SQL: &str = include_str!("../../sql/sf2/insert_student_mapping.sql");
const INSERT_DATE_MAPPING_SQL: &str = include_str!("../../sql/sf2/insert_date_mapping.sql");
const LIST_TEMPLATES_SQL: &str = include_str!("../../sql/sf2/list_templates.sql");
const LATEST_TEMPLATE_FOR_CLASS_SQL: &str =
    include_str!("../../sql/sf2/latest_template_for_class.sql");
const CLOSE_DAY_SQL: &str = include_str!("../../sql/sf2/close_day.sql");
const CLOSED_DAYS_FOR_CLASS_SQL: &str = include_str!("../../sql/sf2/closed_days_for_class.sql");
const STUDENT_MAPPINGS_FOR_TEMPLATE_SQL: &str =
    include_str!("../../sql/sf2/student_mappings_for_template.sql");
const DATE_MAPPINGS_FOR_TEMPLATE_SQL: &str =
    include_str!("../../sql/sf2/date_mappings_for_template.sql");

pub struct Sf2Repository {
    pool: DbPool,
}

impl Sf2Repository {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn find_template(
        &self,
        source_hash: &str,
        grade_level: &str,
        section: &str,
    ) -> Result<Option<Sf2TemplateRecord>> {
        let conn = self.pool.get()?;
        conn.query_row(
            FIND_TEMPLATE_SQL,
            params![source_hash, grade_level, section],
            read_template_record,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn upsert_template_with_mappings(
        &self,
        template: &Sf2TemplateRecord,
        students: &[Sf2StudentMappingRecord],
        dates: &[Sf2DateMappingRecord],
    ) -> Result<()> {
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;

        transaction.execute(
            UPSERT_TEMPLATE_SQL,
            params![
                template.id,
                template.source_path,
                template.source_hash,
                template.school_id,
                template.school_name,
                template.school_year,
                template.report_month,
                template.grade_level,
                template.section,
                template.adviser_name,
                template.school_head_name,
                template.layout_fingerprint,
                template.active_class_id,
                template.imported_at,
            ],
        )?;
        transaction.execute(DELETE_STUDENT_MAPPINGS_SQL, params![template.id])?;
        transaction.execute(DELETE_DATE_MAPPINGS_SQL, params![template.id])?;

        {
            let mut statement = transaction.prepare(INSERT_STUDENT_MAPPING_SQL)?;
            for student in students {
                statement.execute(params![
                    student.template_id,
                    student.student_id,
                    student.workbook_name,
                    student.normalized_name,
                    student.row_index,
                    student.gender_block,
                ])?;
            }
        }

        {
            let mut statement = transaction.prepare(INSERT_DATE_MAPPING_SQL)?;
            for date in dates {
                statement.execute(params![
                    date.template_id,
                    date.sheet_name,
                    date.date,
                    date.column_letter,
                    date.column_index,
                ])?;
            }
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn update_template_with_mappings(
        &self,
        template: &Sf2TemplateRecord,
        students: &[Sf2StudentMappingRecord],
        dates: &[Sf2DateMappingRecord],
    ) -> Result<()> {
        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;

        let rows_updated = transaction.execute(
            UPDATE_TEMPLATE_SQL,
            params![
                template.id,
                template.source_path,
                template.source_hash,
                template.school_id,
                template.school_name,
                template.school_year,
                template.report_month,
                template.grade_level,
                template.section,
                template.adviser_name,
                template.school_head_name,
                template.layout_fingerprint,
                template.active_class_id,
                template.imported_at,
            ],
        )?;
        if rows_updated == 0 {
            return Err(crate::domain::error::AppError::InvalidInput(
                "Selected SF2 workbook was not found".to_string(),
            ));
        }

        transaction.execute(DELETE_STUDENT_MAPPINGS_SQL, params![template.id])?;
        transaction.execute(DELETE_DATE_MAPPINGS_SQL, params![template.id])?;

        {
            let mut statement = transaction.prepare(INSERT_STUDENT_MAPPING_SQL)?;
            for student in students {
                statement.execute(params![
                    student.template_id,
                    student.student_id,
                    student.workbook_name,
                    student.normalized_name,
                    student.row_index,
                    student.gender_block,
                ])?;
            }
        }

        {
            let mut statement = transaction.prepare(INSERT_DATE_MAPPING_SQL)?;
            for date in dates {
                statement.execute(params![
                    date.template_id,
                    date.sheet_name,
                    date.date,
                    date.column_letter,
                    date.column_index,
                ])?;
            }
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn list_templates(&self) -> Result<Vec<Sf2TemplateSummary>> {
        let conn = self.pool.get()?;
        let mut statement = conn.prepare(LIST_TEMPLATES_SQL)?;
        let rows = statement.query_map([], |row| {
            let record = read_template_record(row)?;
            Ok(template_summary(record))
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn latest_template_for_class(&self, class_id: &str) -> Result<Option<Sf2TemplateRecord>> {
        let conn = self.pool.get()?;
        conn.query_row(
            LATEST_TEMPLATE_FOR_CLASS_SQL,
            params![class_id],
            read_template_record,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn close_day(&self, class_id: &str, date: &str, closed_at: i64) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(CLOSE_DAY_SQL, params![class_id, date, closed_at])?;
        Ok(())
    }

    pub fn closed_days_for_class(&self, class_id: &str) -> Result<Vec<String>> {
        let conn = self.pool.get()?;
        let mut statement = conn.prepare(CLOSED_DAYS_FOR_CLASS_SQL)?;
        let rows = statement.query_map(params![class_id], |row| row.get::<_, String>(0))?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn student_mappings_for_template(
        &self,
        template_id: &str,
    ) -> Result<Vec<Sf2StudentMappingRecord>> {
        let conn = self.pool.get()?;
        let mut statement = conn.prepare(STUDENT_MAPPINGS_FOR_TEMPLATE_SQL)?;
        let rows = statement.query_map(params![template_id], |row| {
            Ok(Sf2StudentMappingRecord {
                template_id: row.get(0)?,
                student_id: row.get(1)?,
                workbook_name: row.get(2)?,
                normalized_name: row.get(3)?,
                row_index: row.get::<_, u32>(4)?,
                gender_block: row.get(5)?,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn date_mappings_for_template(
        &self,
        template_id: &str,
    ) -> Result<Vec<Sf2DateMappingRecord>> {
        let conn = self.pool.get()?;
        let mut statement = conn.prepare(DATE_MAPPINGS_FOR_TEMPLATE_SQL)?;
        let rows = statement.query_map(params![template_id], |row| {
            Ok(Sf2DateMappingRecord {
                template_id: row.get(0)?,
                sheet_name: row.get(1)?,
                date: row.get(2)?,
                column_letter: row.get(3)?,
                column_index: row.get::<_, u32>(4)?,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}

#[must_use]
pub fn template_summary(record: Sf2TemplateRecord) -> Sf2TemplateSummary {
    Sf2TemplateSummary {
        id: record.id,
        source_path: record.source_path,
        school_id: record.school_id,
        school_name: record.school_name,
        school_year: record.school_year,
        report_month: record.report_month,
        grade_level: record.grade_level,
        section: record.section,
        adviser_name: record.adviser_name,
        school_head_name: record.school_head_name,
        class_id: record.active_class_id,
        imported_at: record.imported_at,
    }
}

fn read_template_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<Sf2TemplateRecord> {
    Ok(Sf2TemplateRecord {
        id: row.get(0)?,
        source_path: row.get(1)?,
        source_hash: row.get(2)?,
        school_id: row.get(3)?,
        school_name: row.get(4)?,
        school_year: row.get(5)?,
        report_month: row.get(6)?,
        grade_level: row.get(7)?,
        section: row.get(8)?,
        adviser_name: row.get(9)?,
        school_head_name: row.get(10)?,
        layout_fingerprint: row.get(11)?,
        active_class_id: row.get(12)?,
        imported_at: row.get(13)?,
    })
}
