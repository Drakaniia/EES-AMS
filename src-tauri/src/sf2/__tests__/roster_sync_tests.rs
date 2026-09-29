use super::*;

// â”€â”€ roster_sync_formula_marks â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
//
// Verifies that `roster_sync_formula_marks()` computes correct TOTAL Per Day
// formulas and Enrolment summary marks that a roster sync should write to the
// workbook.  Previously `sync_bundled_template_roster` computed `male_count` /
// `female_count` but never called `total_formula_marks` or `summary_formula_marks`
// to update the workbook's TOTAL Per Day rows (29, 49, 50) or enrolment summary
// (AR53, AS53, AT53), leaving stale counts after roster changes.

#[test]
fn roster_sync_formula_marks_computes_correct_marks_for_standard_roster() {
    // Standard bundled template: male rows 8-28, MALE TOTAL at 29
    //                            female rows 30-48, FEMALE TOTAL at 49
    //                            Combined TOTAL at 50
    let male_count = 15usize;
    let female_count = 10usize;
    let male_total_row = 29u32;
    let female_total_row = 49u32;
    let combined_total_row = 50u32;

    let date_mappings = vec![
        Sf2DateMappingRecord {
            template_id: "test".to_string(),
            sheet_name: "JUNE 2026".to_string(),
            date: "2026-06-01".to_string(),
            column_letter: "F".to_string(),
            column_index: 6,
        },
        Sf2DateMappingRecord {
            template_id: "test".to_string(),
            sheet_name: "JUNE 2026".to_string(),
            date: "2026-06-02".to_string(),
            column_letter: "G".to_string(),
            column_index: 7,
        },
    ];

    let (total_marks, summary_formula_marks, summary_static_marks) = roster_sync_formula_marks(
        male_count,
        female_count,
        male_total_row,
        female_total_row,
        combined_total_row,
        &date_mappings,
    );

    // â”€â”€ TOTAL Per Day formulas â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // 2 dates Ã- 3 marks (male, female, combined) = 6 total formula marks
    assert_eq!(
        total_marks.len(),
        6,
        "should have 6 total formula marks (2 dates Ã- 3 rows)"
    );

    // MALE TOTAL (F29): =15-COUNTIF(F8:F28,"X")
    let male_f = total_marks
        .iter()
        .find(|m| m.cell_address == "F29")
        .unwrap();
    assert_eq!(male_f.value, "=15-COUNTIF(F8:F28,\"X\")");
    assert_eq!(male_f.sheet_name, "JUNE 2026");

    // FEMALE TOTAL (F49): =10-COUNTIF(F30:F48,"X")
    let female_f = total_marks
        .iter()
        .find(|m| m.cell_address == "F49")
        .unwrap();
    assert_eq!(female_f.value, "=10-COUNTIF(F30:F48,\"X\")");
    assert_eq!(female_f.sheet_name, "JUNE 2026");

    // Combined TOTAL (F50): =F29+F49
    let combined_f = total_marks
        .iter()
        .find(|m| m.cell_address == "F50")
        .unwrap();
    assert_eq!(combined_f.value, "=F29+F49");
    assert_eq!(combined_f.sheet_name, "JUNE 2026");

    // â”€â”€ Summary section (static enrolment at Row 53) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
    // AR53=15 (male), AS53=10 (female), AT53=25 (total)
    let ar53 = summary_static_marks
        .iter()
        .find(|m| m.cell_address == "AR53")
        .unwrap();
    assert_eq!(ar53.value, "15");
    assert_eq!(ar53.sheet_name, "JUNE 2026");

    let as53 = summary_static_marks
        .iter()
        .find(|m| m.cell_address == "AS53")
        .unwrap();
    assert_eq!(as53.value, "10");
    assert_eq!(as53.sheet_name, "JUNE 2026");

    let at53 = summary_static_marks
        .iter()
        .find(|m| m.cell_address == "AT53")
        .unwrap();
    assert_eq!(at53.value, "25");
    assert_eq!(at53.sheet_name, "JUNE 2026");

    // Verify summary formula marks exist (12 per sheet: 4 rows Ã- 3 cols)
    assert_eq!(
        summary_formula_marks.len(),
        12,
        "should have 12 summary formula marks (4 rows Ã- 3 cols)"
    );

    // Row 63 ADA references correct total rows
    let ar63 = summary_formula_marks
        .iter()
        .find(|m| m.cell_address == "AR63")
        .unwrap();
    assert_eq!(
        ar63.value, "=IFERROR(AVERAGE(F29:AL29),0)",
        "male ADA should reference row 29"
    );

    let as63 = summary_formula_marks
        .iter()
        .find(|m| m.cell_address == "AS63")
        .unwrap();
    assert_eq!(
        as63.value, "=IFERROR(AVERAGE(F49:AL49),0)",
        "female ADA should reference row 49"
    );

    let at63 = summary_formula_marks
        .iter()
        .find(|m| m.cell_address == "AT63")
        .unwrap();
    assert_eq!(
        at63.value, "=IFERROR(AVERAGE(F50:AL50),0)",
        "combined ADA should reference row 50"
    );
}

#[test]
fn roster_sync_formula_marks_empty_date_mappings_returns_empty() {
    let date_mappings: Vec<Sf2DateMappingRecord> = vec![];

    let (total_marks, summary_formula_marks, summary_static_marks) =
        roster_sync_formula_marks(15, 10, 29, 49, 50, &date_mappings);

    assert!(
        total_marks.is_empty(),
        "no total marks when date_mappings is empty"
    );
    assert!(
        summary_formula_marks.is_empty(),
        "no summary formula marks when date_mappings is empty"
    );
    assert!(
        summary_static_marks.is_empty(),
        "no summary static marks when date_mappings is empty"
    );
}

#[test]
fn roster_sync_formula_marks_expanded_roster_uses_shifted_total_rows() {
    // Expanded roster: MALE TOTAL at 33, FEMALE TOTAL at 56, Combined at 57
    let male_count = 25usize;
    let female_count = 22usize;
    let male_total_row = 33u32;
    let female_total_row = 56u32;
    let combined_total_row = 57u32;

    let date_mappings = vec![Sf2DateMappingRecord {
        template_id: "test".to_string(),
        sheet_name: "JULY 2026".to_string(),
        date: "2026-07-15".to_string(),
        column_letter: "P".to_string(),
        column_index: 16,
    }];

    let (total_marks, _summary_formula, _summary_static) = roster_sync_formula_marks(
        male_count,
        female_count,
        male_total_row,
        female_total_row,
        combined_total_row,
        &date_mappings,
    );

    assert_eq!(
        total_marks.len(),
        3,
        "should have 3 total marks (1 date Ã- 3 rows)"
    );

    // MALE TOTAL at P33 = =25-COUNTIF(P8:P32,"X")
    let male_mark = total_marks
        .iter()
        .find(|m| m.cell_address == "P33")
        .unwrap();
    assert_eq!(male_mark.value, "=25-COUNTIF(P8:P32,\"X\")");

    // FEMALE TOTAL at P56 = =22-COUNTIF(P34:P55,"X")
    let female_mark = total_marks
        .iter()
        .find(|m| m.cell_address == "P56")
        .unwrap();
    assert_eq!(female_mark.value, "=22-COUNTIF(P34:P55,\"X\")");
}

// -- spec 6.3 / E7: the DepEd learner ID is the first matching rule ------

use crate::domain::models::StudentId;
use crate::infrastructure::database::{init_db, StudentRepository};
use crate::sf2::models::{Sf2StudentMappingRecord, Sf2WorkbookLearner};
use crate::sf2::roster::roster_sync_learner::sync_workbook_learner_mappings_with_old;

const CLASS_ID: &str = "class-1";
const STUDENT_ONE: &str = "11111111-1111-4111-8111-111111111111";
const STUDENT_TWO: &str = "22222222-2222-4222-8222-222222222222";

fn pool_with_two_students() -> crate::infrastructure::database::DbPool {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = init_db(dir.path().join("attendance.db")).expect("migrate test database");
    std::mem::forget(dir);

    let conn = pool.get().expect("connection");
    conn.execute(
        "INSERT INTO classes (id, name, day_start, day_end, late_after, created_at)
         VALUES (?1, 'Grade 1 - A', '07:00', '13:00', '07:30', 1)",
        [CLASS_ID],
    )
    .expect("insert class");
    // Seeded with fixed ids because the *old* mappings below have to name the
    // same students the roster sync reads out of the class.
    for (id, name, learner_id) in [
        (STUDENT_ONE, "SANTO, MARIA", "LRN-0001"),
        (STUDENT_TWO, "SANTOS, PEDRO", "LRN-0002"),
    ] {
        conn.execute(
            "INSERT INTO students (id, name, class_id, created_at, sf2_learner_id)
             VALUES (?1, ?2, ?3, 1, ?4)",
            rusqlite::params![id, name, CLASS_ID, learner_id],
        )
        .expect("insert student");
    }
    pool
}

fn old_mapping(student_id: &str, row_index: u32, name: &str) -> Sf2StudentMappingRecord {
    Sf2StudentMappingRecord {
        template_id: "old-template".to_string(),
        student_id: student_id.to_string(),
        workbook_name: name.to_string(),
        normalized_name: crate::sf2::logic::normalize_learner_name(name),
        row_index,
        gender_block: Some("MALE".to_string()),
    }
}

fn learner(row_index: u32, name: &str, learner_id: Option<&str>) -> Sf2WorkbookLearner {
    Sf2WorkbookLearner {
        row_index,
        name: name.to_string(),
        gender_block: Some("MALE".to_string()),
        sf2_learner_id: learner_id.map(str::to_string),
    }
}

#[test]
fn a_reshuffled_roster_matches_on_the_deped_learner_id_not_the_row() {
    // E7, the reason S6.3 exists. The new file spells Maria's name differently,
    // so no name matches, and she now sits in row 9 - the row Pedro used to
    // occupy. Matching on the row would hand her Pedro's identity, and with it
    // Pedro's X marks. The learner ID is the only thing that says who she is.
    let pool = pool_with_two_students();
    let repo = StudentRepository::new(pool.clone());

    let sync = sync_workbook_learner_mappings_with_old(
        &repo,
        CLASS_ID,
        "new-template",
        &[learner(9, "SANTO, MARIA L.", Some("LRN-0001"))],
        &[
            old_mapping(STUDENT_ONE, 8, "SANTO, MARIA"),
            old_mapping(STUDENT_TWO, 9, "SANTOS, PEDRO"),
        ],
    )
    .expect("roster sync");

    assert_eq!(
        sync.student_mappings[0].student_id, STUDENT_ONE,
        "the learner must resolve to the student the school recorded, not to whoever is in row 9"
    );
    assert_eq!(
        sync.students_created, 0,
        "a learner the app already knows must not be duplicated"
    );

    let students = repo.list_by_class(Some(CLASS_ID)).expect("list");
    assert_eq!(students.len(), 2, "no new student was created");
    let maria = students
        .iter()
        .find(|student| student.id.0.to_string() == STUDENT_ONE)
        .expect("Maria");
    assert_eq!(
        maria.name, "SANTO, MARIA L.",
        "the school is authoritative about how the learner is spelled"
    );
}

#[test]
fn a_workbook_with_no_learner_id_still_resolves_the_learner_by_name() {
    // The bundled template merges the learner-ID cell into the "No." cell, so
    // `sf2_learner_id` is NULL for every learner on it. That must keep working:
    // the same student comes back and no duplicate is created. The ID is an
    // improvement where the school supplies one, never a requirement - and the
    // plausibility guard that turns the bundled template's item number into a
    // "no ID" is doing exactly its job here.

    let pool = pool_with_two_students();
    let repo = StudentRepository::new(pool.clone());

    let sync = sync_workbook_learner_mappings_with_old(
        &repo,
        CLASS_ID,
        "new-template",
        &[learner(14, "SANTO, MARIA", None)],
        &[
            old_mapping(STUDENT_ONE, 8, "SANTO, MARIA"),
            old_mapping(STUDENT_TWO, 9, "SANTOS, PEDRO"),
        ],
    )
    .expect("roster sync");

    assert_eq!(
        sync.student_mappings[0].student_id, STUDENT_ONE,
        "with no ID to go on, the normalized name is the match"
    );
    assert_eq!(sync.students_created, 0);
}

#[test]
fn the_student_model_carries_the_deped_learner_id() {
    // The matcher can only consult the ID if the model carries it, and the
    // model can only carry it if the repository reads it. This is the seam that
    // was missing when S6.3 was written.
    let pool = pool_with_two_students();
    let repo = StudentRepository::new(pool.clone());

    let maria = repo
        .get(StudentId(uuid::Uuid::parse_str(STUDENT_ONE).expect("uuid")))
        .expect("read the student");

    assert_eq!(
        maria.sf2_learner_id.as_deref(),
        Some("LRN-0001"),
        "`Student.sf2_learner_id` is the school's own record of the child and is what the roster \
         matcher consults first"
    );
}

#[test]
fn renaming_a_student_keeps_their_deped_learner_id() {
    // The ID is written by the SF2 roster, not by the Students page, so a rename
    // must not clear it. A cleared ID silently demotes every future match from
    // `LearnerId` to `NormalizedName`, which is the bug this whole item is about.
    let pool = pool_with_two_students();
    let repo = StudentRepository::new(pool.clone());
    let id = StudentId(uuid::Uuid::parse_str(STUDENT_ONE).expect("uuid"));

    let renamed = repo
        .update(
            id,
            crate::domain::models::UpdateStudentRequest {
                name: Some("SANTO, MARIA L.".to_string()),
                gender: None,
                card_serial: None,
                class_id: None,
            },
        )
        .expect("rename");

    assert_eq!(renamed.sf2_learner_id.as_deref(), Some("LRN-0001"));
    assert_eq!(
        repo.get(id).expect("re-read").sf2_learner_id.as_deref(),
        Some("LRN-0001"),
        "the stored ID must survive a rename"
    );
}
