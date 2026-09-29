//! The roster matcher, wired into the roster-sync path (spec Â§6.3, D6, E7).
//!
//! The order is `sf2_learner_id` â†’ `normalized_name` â†’ `row_index`, and it is
//! owned by [`match_roster_learner`] rather than by a row-index map built inline
//! at the call site. These tests pin what that order buys the caller.
//!
//! One honest limitation, so it is written down rather than discovered later: the
//! pre-split `sf2_student_mappings` rows this path is handed have no DepEd
//! learner-ID column, so the ID branch cannot fire *here*. It fires in the split
//! (`month::split`), which resolves each month against the v22-backfilled month
//! mappings - those do carry a learner ID, and the split threads the IDs it
//! discovers into the next month. What this path gains today is the shared
//! order, the name-before-row precedence, and a claim guard that stops two
//! workbook rows resolving to one student.

use crate::domain::models::{CreateStudentRequest, StudentGender, StudentId};
use crate::infrastructure::database::{init_db, StudentRepository};
use crate::sf2::models::{Sf2StudentMappingRecord, Sf2WorkbookLearner};
use crate::sf2::roster::sync_workbook_learner_mappings_with_old;
use uuid::Uuid;

const CLASS_ID: &str = "class-1";

/// A migrated, empty database in a throwaway directory, with two students.
fn seeded_repo() -> (StudentRepository, String, String) {
    let dir = tempfile::tempdir().expect("temp dir for the test database");
    let pool = init_db(dir.path().join("attendance.db")).expect("migrate test database");
    std::mem::forget(dir);

    let repo = StudentRepository::new(pool.clone());
    {
        let conn = pool.get().expect("connection");
        conn.execute(
            "INSERT OR IGNORE INTO classes (id, name, day_start, day_end, late_after, created_at)
             VALUES (?1, 'Grade 1', '07:00', '13:00', '07:30', 1)",
            rusqlite::params![CLASS_ID],
        )
        .expect("insert class");
    }

    let juan = repo
        .create(CreateStudentRequest {
            name: "CRUZ, JUAN".to_string(),
            gender: Some(StudentGender::Male),
            card_serial: None,
            class_id: Some(CLASS_ID.to_string()),
        })
        .expect("create juan");
    let maria = repo
        .create(CreateStudentRequest {
            name: "REYES, MARIA".to_string(),
            gender: Some(StudentGender::Female),
            card_serial: None,
            class_id: Some(CLASS_ID.to_string()),
        })
        .expect("create maria");
    (repo, juan.id.to_string(), maria.id.to_string())
}

/// The previous template's mappings: JUAN on row 8, MARIA on row 9.
fn old_mappings(juan: &str, maria: &str) -> Vec<Sf2StudentMappingRecord> {
    vec![
        mapping("old-template", juan, "CRUZ, JUAN", 8),
        mapping("old-template", maria, "REYES, MARIA", 9),
    ]
}

fn mapping(
    template_id: &str,
    student_id: &str,
    name: &str,
    row_index: u32,
) -> Sf2StudentMappingRecord {
    Sf2StudentMappingRecord {
        template_id: template_id.to_string(),
        student_id: student_id.to_string(),
        workbook_name: name.to_string(),
        normalized_name: crate::sf2::logic::normalize_learner_name(name),
        row_index,
        gender_block: None,
    }
}

fn learner(row_index: u32, name: &str, gender_block: &str) -> Sf2WorkbookLearner {
    Sf2WorkbookLearner {
        row_index,
        name: name.to_string(),
        gender_block: Some(gender_block.to_string()),
        sf2_learner_id: None,
    }
}

#[test]
fn a_reshuffled_roster_keeps_every_student_their_own_marks() {
    // E7, the failure this order exists to prevent. The workbook's rows 8 and 9
    // now hold MARIA and JUAN the other way round. Matching on the row alone
    // would hand each student the other's row - and therefore the other's X
    // marks - for the rest of the year.
    let (repo, juan, maria) = seeded_repo();
    let learners = vec![
        learner(8, "REYES, MARIA", "MALE"),
        learner(9, "CRUZ, JUAN", "MALE"),
    ];

    let sync = sync_workbook_learner_mappings_with_old(
        &repo,
        CLASS_ID,
        "new-template",
        &learners,
        &old_mappings(&juan, &maria),
    )
    .expect("sync");

    let row_eight = sync
        .student_mappings
        .iter()
        .find(|mapping| mapping.row_index == 8)
        .expect("a mapping for row 8");
    let row_nine = sync
        .student_mappings
        .iter()
        .find(|mapping| mapping.row_index == 9)
        .expect("a mapping for row 9");

    assert_eq!(row_eight.student_id, maria, "row 8 holds MARIA");
    assert_eq!(row_nine.student_id, juan, "row 9 holds JUAN");
    assert_eq!(sync.students_created, 0, "both learners were already known");
    assert_eq!(
        sync.students_updated, 0,
        "nobody was renamed: the name is the identity here"
    );
}

#[test]
fn a_rename_the_app_does_not_know_about_still_lands_on_the_right_student() {
    // The name no longer answers to anything in the class, so the matcher falls
    // through to the row. That is the last resort, and it is the only thing left
    // to go on - so the student is reused and renamed rather than duplicated.
    let (repo, juan, maria) = seeded_repo();
    let learners = vec![learner(8, "CRUZ, JUANBERTO", "MALE")];

    let sync = sync_workbook_learner_mappings_with_old(
        &repo,
        CLASS_ID,
        "new-template",
        &learners,
        &old_mappings(&juan, &maria),
    )
    .expect("sync");

    assert_eq!(sync.student_mappings.len(), 1);
    assert_eq!(sync.student_mappings[0].student_id, juan);
    assert_eq!(sync.students_updated, 1, "the student was renamed");
    assert_eq!(sync.students_created, 0);
    assert_eq!(
        repo.get(StudentId(Uuid::parse_str(&juan).expect("a uuid")))
            .expect("juan")
            .name,
        "CRUZ, JUANBERTO"
    );
}

#[test]
fn a_learner_the_app_has_never_seen_is_created_rather_than_matched_by_position() {
    // Nothing about this learner matches, and inventing a match on a row would
    // be exactly the mis-attachment E7 is about.
    let (repo, _juan, _maria) = seeded_repo();
    let learners = vec![learner(20, "BAUTISTA, LIZA", "FEMALE")];

    let sync =
        sync_workbook_learner_mappings_with_old(&repo, CLASS_ID, "new-template", &learners, &[])
            .expect("sync");

    assert_eq!(sync.students_created, 1);
    assert_eq!(sync.student_mappings[0].workbook_name, "BAUTISTA, LIZA");
    assert_eq!(sync.student_mappings[0].row_index, 20);
}

#[test]
fn one_student_never_ends_up_claimed_by_two_workbook_rows() {
    // `sf2_student_mappings` keys on `(template_id, student_id)`. If two rows
    // resolved to one student the insert would fail and take the whole import
    // with it, so the second row is left to be created as its own student.
    let (repo, juan, _maria) = seeded_repo();
    let old = vec![mapping("old-template", &juan, "CRUZ, JUAN", 8)];
    // Two rows, neither name in the class, and the second falls through to the
    // same old mapping because the first already took it.
    let learners = vec![
        learner(8, "CRUZ, JUANBERTO", "MALE"),
        learner(12, "CRUZ, JUANBERTO", "MALE"),
    ];

    let sync =
        sync_workbook_learner_mappings_with_old(&repo, CLASS_ID, "new-template", &learners, &old)
            .expect("sync");

    let mut student_ids = sync
        .student_mappings
        .iter()
        .map(|mapping| mapping.student_id.clone())
        .collect::<Vec<_>>();
    student_ids.sort();
    student_ids.dedup();
    assert_eq!(
        student_ids.len(),
        sync.student_mappings.len(),
        "one student was mapped to two rows"
    );
    assert!(
        sync.student_mappings
            .iter()
            .any(|mapping| mapping.student_id == juan),
        "the first row still resolved to the student it always did"
    );
}

#[test]
fn a_mapping_whose_student_no_longer_exists_does_not_veto_a_new_student() {
    // A mapping can outlive its student. The learner's name answers to nothing in
    // the class, so the matcher falls through to the row, finds a mapping whose
    // student has been deleted, and must then create rather than reuse an id
    // that would fail the foreign key and take the whole import with it.
    let (repo, _juan, _maria) = seeded_repo();
    let ghost = Uuid::new_v4().to_string();
    let old = vec![mapping("old-template", &ghost, "CRUZ, JUAN", 8)];
    let learners = vec![learner(8, "CRUZ, JUANBERTO", "MALE")];

    let sync =
        sync_workbook_learner_mappings_with_old(&repo, CLASS_ID, "new-template", &learners, &old)
            .expect("sync");

    assert_eq!(
        sync.students_created, 1,
        "a new student, not the ghost's id"
    );
    assert_ne!(sync.student_mappings[0].student_id, ghost);
    assert_eq!(sync.student_mappings[0].workbook_name, "CRUZ, JUANBERTO");
}

#[test]
fn a_mapping_with_an_unreadable_student_id_is_ignored_rather_than_trusted() {
    let (repo, _juan, _maria) = seeded_repo();
    let old = vec![mapping("old-template", "not-a-uuid", "CRUZ, JUAN", 8)];
    let learners = vec![learner(8, "CRUZ, JUANBERTO", "MALE")];

    let sync =
        sync_workbook_learner_mappings_with_old(&repo, CLASS_ID, "new-template", &learners, &old)
            .expect("sync");

    assert_eq!(sync.students_created, 1);
    assert_eq!(sync.student_mappings[0].workbook_name, "CRUZ, JUANBERTO");
}
