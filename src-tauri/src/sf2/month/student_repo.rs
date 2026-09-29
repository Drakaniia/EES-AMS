//! `sf2_month_student_mappings` - one month file's roster, and the DepEd
//! learner ID that identifies a learner across month files (spec §6.3).
//!
//! Identity, in the order the spec sets out:
//!
//! 1. `sf2_learner_id` - the school's own record. Survives a rename and a
//!    roster reshuffle between two months (E7).
//! 2. `normalized_name` - survives a row move, not a rename.
//! 3. `row_index` - position in one file. Says nothing about the learner, and
//!    is exactly what silently re-pointed a student's X marks at somebody else
//!    when the roster was re-sorted between months.
//!
//! [`match_roster_learner`] implements that order. It is not wired into the
//! roster sync yet - the split phase owns that caller - so it is exposed here
//! with the shape that caller needs.

use crate::domain::error::{AppError, Result};
use crate::infrastructure::database::DbPool;
use crate::sf2::logic::normalize_learner_name;
use crate::sf2::models::Sf2WorkbookLearner;
use crate::sf2::month::{Sf2MonthStudentMapping, EMPTY_ROSTER_ANALYSIS_MESSAGE};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

const INSERT_STUDENT_MAPPING_SQL: &str = include_str!("../sql/month_insert_student_mapping.sql");
const STUDENT_MAPPINGS_FOR_TEMPLATE_SQL: &str =
    include_str!("../sql/month_student_mappings_for_template.sql");
const STUDENT_MAPPINGS_BY_LEARNER_ID_SQL: &str =
    include_str!("../sql/month_student_mappings_by_learner_id.sql");
const STUDENT_MAPPING_BY_NORMALIZED_NAME_SQL: &str =
    include_str!("../sql/month_student_mapping_by_normalized_name.sql");
const DELETE_STUDENT_MAPPINGS_SQL: &str = include_str!("../sql/month_delete_student_mappings.sql");
const SET_STUDENT_LEARNER_ID_SQL: &str = include_str!("../sql/month_set_student_learner_id.sql");

/// The longest value accepted as a DepEd learner ID. A real LRN is a short
/// digit string; anything longer is a name or a header that landed in the
/// column, and storing it would create an identity that can never match.
const MAX_LEARNER_ID_LEN: usize = 32;

/// Which rule identified a learner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sf2LearnerMatchKind {
    /// Matched on the DepEd learner ID read from the workbook.
    LearnerId,
    /// Matched on the normalized name.
    NormalizedName,
    /// Fell back to the row position. Position is not identity: a caller that
    /// lands here should log it, because a reshuffled roster is a silent
    /// mis-attachment waiting to happen.
    RowIndex,
}

/// One identified learner, and which rule found them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sf2LearnerMatch {
    pub template_id: String,
    pub student_id: String,
    pub row_index: u32,
    pub matched_by: Sf2LearnerMatchKind,
}

/// The DepEd learner ID in a workbook cell, or `None` when the cell does not
/// actually hold one.
///
/// **Read from column 2** - the learner-ID slot of the DepEd SF2 form. The
/// bundled `TEMPLATE_AUTOMATED_SF2.xls` merges that cell into the "No." cell
/// (`A8:B8` holds the item number, `C8:E8` holds the name), so reading column 2
/// on that template returns the item number. When the two cells read
/// identically the value is the item number, not an ID, and it is rejected:
/// storing "1", "2", "3"... as learner IDs would give every month file the
/// same positional identity, which is the exact fragility this column exists
/// to remove.
///
/// A workbook that has a real DepEd ID in an unmerged column B reads it here
/// and keeps it; a learner with no ID falls through to the name, then to the
/// row.
///
/// A mis-aligned column is rejected too, by [`looks_like_a_learner_id`]. A name
/// that landed in the ID slot is worse than no ID at all: it is a plausible
/// string, so it would be stored and then matched as if it were the school's
/// own record - outranking the name match and re-pointing a learner's marks.
#[must_use]
pub fn deped_learner_id_from_cells(
    learner_id_cell: &str,
    item_number_cell: &str,
) -> Option<String> {
    let candidate = learner_id_cell.trim();
    if candidate.is_empty() || candidate.len() > MAX_LEARNER_ID_LEN {
        return None;
    }
    if candidate == item_number_cell.trim() {
        return None;
    }
    if !looks_like_a_learner_id(candidate) {
        return None;
    }
    Some(candidate.to_string())
}

/// Could `candidate` be a DepEd learner ID rather than a name or a stray value?
///
/// A DepEd ID is a single token made of letters, digits and the odd separator
/// (`13672845021`, `LRN-0001`, `LRNS-2024-0012345`), and it always carries at
/// least one digit. A learner's name does not: it carries spaces, and
/// punctuation like the comma in `DELA CRUZ, JUAN`. Requiring a digit as well
/// rejects the all-letters cases a separator check alone would let through.
fn looks_like_a_learner_id(candidate: &str) -> bool {
    candidate.chars().any(|ch| ch.is_ascii_digit())
        && candidate.chars().all(|ch| {
            ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '/' || ch == '.'
        })
}

/// Identify a workbook learner against the mappings already on record.
///
/// `existing` is any month's mapping list - in practice the previous month's,
/// which is the whole point: the learner is being re-deranged into a new file.
/// The first rule that matches wins, so a DepEd ID outranks a name that a
/// different learner also answers to.
#[must_use]
pub fn match_roster_learner(
    existing: &[Sf2MonthStudentMapping],
    learner: &Sf2WorkbookLearner,
) -> Option<Sf2LearnerMatch> {
    if let Some(learner_id) = learner
        .sf2_learner_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        if let Some(mapping) = existing
            .iter()
            .find(|mapping| mapping.sf2_learner_id.as_deref() == Some(learner_id))
        {
            return Some(match_from(mapping, Sf2LearnerMatchKind::LearnerId));
        }
    }

    let normalized_name = normalize_learner_name(&learner.name);
    if let Some(mapping) = existing
        .iter()
        .find(|mapping| mapping.normalized_name == normalized_name)
    {
        return Some(match_from(mapping, Sf2LearnerMatchKind::NormalizedName));
    }

    existing
        .iter()
        .find(|mapping| mapping.row_index == learner.row_index)
        .map(|mapping| match_from(mapping, Sf2LearnerMatchKind::RowIndex))
}

fn match_from(
    mapping: &Sf2MonthStudentMapping,
    matched_by: Sf2LearnerMatchKind,
) -> Sf2LearnerMatch {
    Sf2LearnerMatch {
        template_id: mapping.template_id.clone(),
        student_id: mapping.student_id.clone(),
        row_index: mapping.row_index,
        matched_by,
    }
}

/// CRUD for a month file's roster.
pub struct Sf2MonthStudentRepo {
    pool: DbPool,
}

impl Sf2MonthStudentRepo {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Replace one month file's roster with `students`, in a single
    /// transaction.
    ///
    /// An empty roster is rejected rather than committed: committing it would
    /// unmap every learner in the month file, and a file whose marks are then
    /// written through an empty mapping is how marks end up on the wrong row.
    pub fn replace_for_template(
        &self,
        template_id: &str,
        students: &[Sf2MonthStudentMapping],
    ) -> Result<()> {
        if students.is_empty() {
            return Err(AppError::InvalidInput(
                EMPTY_ROSTER_ANALYSIS_MESSAGE.to_string(),
            ));
        }

        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        transaction.execute(DELETE_STUDENT_MAPPINGS_SQL, params![template_id])?;
        {
            let mut statement = transaction.prepare(INSERT_STUDENT_MAPPING_SQL)?;
            for student in students {
                statement.execute(params![
                    &student.template_id,
                    student.student_id,
                    student.workbook_name,
                    student.normalized_name,
                    student.row_index,
                    student.gender_block,
                    student.sf2_learner_id,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// One month file's roster, in workbook row order.
    pub fn for_template(&self, template_id: &str) -> Result<Vec<Sf2MonthStudentMapping>> {
        let conn = self.pool.get()?;
        let mut statement = conn.prepare(STUDENT_MAPPINGS_FOR_TEMPLATE_SQL)?;
        let rows = statement.query_map(params![template_id], |row| {
            Ok(Sf2MonthStudentMapping {
                template_id: row.get(0)?,
                student_id: row.get(1)?,
                workbook_name: row.get(2)?,
                normalized_name: row.get(3)?,
                row_index: row.get::<_, u32>(4)?,
                gender_block: row.get(5)?,
                sf2_learner_id: row.get(6)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    /// Every mapping that already knows this DepEd learner ID, across months.
    ///
    /// The query a split or a roster sync runs first, because it is the only
    /// one that survives a learner moving to a different row.
    pub fn for_learner_id(&self, sf2_learner_id: &str) -> Result<Vec<Sf2MonthStudentMapping>> {
        let conn = self.pool.get()?;
        let mut statement = conn.prepare(STUDENT_MAPPINGS_BY_LEARNER_ID_SQL)?;
        let rows = statement.query_map(params![sf2_learner_id], |row| {
            Ok(Sf2MonthStudentMapping {
                template_id: row.get(0)?,
                student_id: row.get(1)?,
                workbook_name: row.get(2)?,
                normalized_name: row.get(3)?,
                row_index: row.get::<_, u32>(4)?,
                gender_block: row.get(5)?,
                sf2_learner_id: row.get(6)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    /// The mapping for one normalized name in one month file.
    pub fn for_normalized_name(
        &self,
        template_id: &str,
        normalized_name: &str,
    ) -> Result<Option<Sf2MonthStudentMapping>> {
        let conn = self.pool.get()?;
        conn.query_row(
            STUDENT_MAPPING_BY_NORMALIZED_NAME_SQL,
            params![template_id, normalized_name],
            |row| {
                Ok(Sf2MonthStudentMapping {
                    template_id: row.get(0)?,
                    student_id: row.get(1)?,
                    workbook_name: row.get(2)?,
                    normalized_name: row.get(3)?,
                    row_index: row.get::<_, u32>(4)?,
                    gender_block: row.get(5)?,
                    sf2_learner_id: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    /// Store DepEd learner IDs on students, the backfill the split performs.
    ///
    /// Returns how many students were written. A pair that was refused, because
    /// the ID already belongs to another learner or the student already has a
    /// different ID, is simply not counted. The caller can report
    /// `len - written` as the number of identities that need a human.
    pub fn set_student_learner_ids(&self, pairs: &[(String, String)]) -> Result<usize> {
        if pairs.is_empty() {
            return Ok(0);
        }

        let mut conn = self.pool.get()?;
        let transaction = conn.transaction()?;
        let mut written = 0usize;
        {
            let mut statement = transaction.prepare(SET_STUDENT_LEARNER_ID_SQL)?;
            for (student_id, sf2_learner_id) in pairs {
                let learner_id = sf2_learner_id.trim();
                if learner_id.is_empty() {
                    continue;
                }
                written += statement.execute(params![student_id, learner_id])?;
            }
        }
        transaction.commit()?;
        Ok(written)
    }

    /// Drop one month file's roster and report how many rows went.
    pub fn delete_for_template(&self, template_id: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        conn.execute(DELETE_STUDENT_MAPPINGS_SQL, params![template_id])
            .map_err(Into::into)
    }
}

#[cfg(test)]
#[path = "__tests__/student_repo_tests.rs"]
mod tests;
