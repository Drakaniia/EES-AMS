//! The diagnostic's own tests.
//!
//! Two kinds live here, and the second kind is the one this task exists for:
//!
//! * the rules - what a verdict may claim, and what a month is allowed to report
//!   when it was not measured. No Excel, no user data.
//! * [`no_write`] - proof that a run leaves the database file's bytes and
//!   modification time exactly as they were, and so does the workbook.

use super::model::{
    incomparable_summary, verdict_for, verdict_reason, MarkCounts, MarkSourceStatus, MarkVerdict,
    MonthMarkComparison,
};

// ── fixtures ────────────────────────────────────────────────────────────

fn mark_cell(index: usize) -> super::model::MarkCell {
    super::model::MarkCell {
        student_name: format!("Learner {index}"),
        date: format!("2026-09-{:02}", index + 1),
        sheet_name: "SEPTEMBER 2026".to_string(),
        cell_address: format!("H{}", index + 8),
    }
}

/// A month the diagnostic actually measured.
fn measured(workbook_only: usize, database_only: usize) -> MonthMarkComparison {
    MonthMarkComparison {
        report_month: "SEPTEMBER".to_string(),
        report_year: 2026,
        source_status: MarkSourceStatus::Comparable,
        reason: "measured".to_string(),
        counts: MarkCounts {
            workbook_x_count: Some(workbook_only + 4),
            db_absent_count: Some(database_only + 4),
            db_mapped_absent_count: Some(database_only + 4),
            cells_scanned: Some(600),
            guard_x_count: Some(workbook_only + 4),
            guard_agrees: Some(true),
        },
        cells_only_in_workbook: (0..workbook_only).map(mark_cell).collect(),
        cells_only_in_database: (0..database_only).map(mark_cell).collect(),
        ..MonthMarkComparison::default()
    }
}

/// A month the diagnostic could not measure.
fn unmeasured(month: &str, status: MarkSourceStatus) -> MonthMarkComparison {
    MonthMarkComparison::unmeasured(month, 2026, status, "no worksheet for this month")
}

fn school_year() -> Vec<MonthMarkComparison> {
    [
        "JUNE",
        "JULY",
        "AUGUST",
        "SEPTEMBER",
        "OCTOBER",
        "NOVEMBER",
        "DECEMBER",
        "JANUARY",
        "FEBRUARY",
        "MARCH",
        "APRIL",
        "MAY",
    ]
    .into_iter()
    .map(|month| {
        let mut comparison = measured(0, 0);
        comparison.report_month = month.to_string();
        comparison
    })
    .collect()
}

// ── the verdict rules ───────────────────────────────────────────────────

#[test]
fn a_workbook_mark_the_database_lacks_outranks_every_other_answer() {
    let mut months = school_year();
    months[3] = measured(2, 0);

    assert_eq!(verdict_for(&months), MarkVerdict::WorkbookIsSourceOfTruth);
}

#[test]
fn an_unreadable_month_does_not_stop_a_missing_mark_being_reported() {
    let mut months = school_year();
    months[3] = measured(1, 0);
    months[0] = unmeasured("JUNE", MarkSourceStatus::ExcelUnavailable);

    assert_eq!(verdict_for(&months), MarkVerdict::WorkbookIsSourceOfTruth);
}

#[test]
fn the_database_is_only_the_source_of_truth_when_every_month_was_read() {
    let mut months = school_year();
    months[5] = unmeasured("NOVEMBER", MarkSourceStatus::NoSheet);

    assert_eq!(verdict_for(&months), MarkVerdict::Incomparable);
}

#[test]
fn absences_only_the_database_has_do_not_stop_the_database_being_authoritative() {
    let mut months = school_year();
    months[7] = measured(0, 3);

    assert_eq!(verdict_for(&months), MarkVerdict::DatabaseIsSourceOfTruth);
}

#[test]
fn a_school_year_with_no_measurable_month_is_incomparable() {
    let months: Vec<MonthMarkComparison> = school_year()
        .into_iter()
        .map(|mut month| {
            month.source_status = MarkSourceStatus::WorkbookMissing;
            month.cells_only_in_workbook.clear();
            month
        })
        .collect();

    assert_eq!(verdict_for(&months), MarkVerdict::Incomparable);
}

#[test]
fn an_empty_school_year_is_incomparable_rather_than_reassuring() {
    assert_eq!(verdict_for(&[]), MarkVerdict::Incomparable);
}

#[test]
fn only_the_database_being_authoritative_permits_a_write() {
    assert!(MarkVerdict::DatabaseIsSourceOfTruth.permits_write());
    assert!(!MarkVerdict::WorkbookIsSourceOfTruth.permits_write());
    assert!(!MarkVerdict::Incomparable.permits_write());
}

#[test]
fn the_reason_for_a_stale_verdict_names_the_number_of_missing_marks() {
    let mut months = school_year();
    months[3] = measured(7, 0);

    let reason = verdict_reason(verdict_for(&months), &months, 0);

    assert!(
        reason.contains("7 X mark(s)"),
        "the reason must carry the count a teacher can act on: {reason}"
    );
}

#[test]
fn the_reason_for_an_incomparable_verdict_names_the_months_it_could_not_read() {
    let mut months = school_year();
    months[2] = unmeasured("AUGUST", MarkSourceStatus::NoSheet);
    months[5] = unmeasured("NOVEMBER", MarkSourceStatus::WorkbookMissing);

    let reason = verdict_reason(verdict_for(&months), &months, 0);

    assert!(reason.contains("2 of 12 month(s)"), "{reason}");
    assert!(reason.contains("AUGUST 2026 (NoSheet)"), "{reason}");
    assert!(
        reason.contains("NOVEMBER 2026 (WorkbookMissing)"),
        "{reason}"
    );
    assert!(reason.contains("No conclusion is drawn"), "{reason}");
}

#[test]
fn the_reason_for_an_incomparable_verdict_reports_the_unplaceable_cells_too() {
    let months = school_year();

    let reason = verdict_reason(MarkVerdict::Incomparable, &months, 36);

    assert!(reason.contains("A further 36 X cell(s)"), "{reason}");
}

#[test]
fn an_unmeasured_month_reports_its_status_and_its_reason() {
    let month = unmeasured("AUGUST", MarkSourceStatus::NoSheet);

    assert_eq!(
        incomparable_summary(&month),
        "AUGUST 2026: NoSheet - no worksheet for this month"
    );
}

// ── the false-zero rule ─────────────────────────────────────────────────

#[test]
fn an_unmeasured_month_reports_no_count_at_all() {
    for status in [
        MarkSourceStatus::WorkbookMissing,
        MarkSourceStatus::ExcelUnavailable,
        MarkSourceStatus::NoSheet,
        MarkSourceStatus::NoMappings,
    ] {
        let month = unmeasured("AUGUST", status);

        assert_eq!(month.counts.workbook_x_count, None, "{status:?}");
        assert_eq!(month.counts.db_absent_count, None, "{status:?}");
        assert_eq!(month.counts.db_mapped_absent_count, None, "{status:?}");
        assert_eq!(month.counts.cells_scanned, None, "{status:?}");
        assert!(month.cells_only_in_workbook.is_empty(), "{status:?}");
        assert!(month.cells_only_in_database.is_empty(), "{status:?}");
    }
}

#[test]
fn only_a_comparable_month_is_comparable() {
    assert!(MarkSourceStatus::Comparable.is_comparable());
    assert!(!MarkSourceStatus::NoSheet.is_comparable());
    assert!(!MarkSourceStatus::NoMappings.is_comparable());
    assert!(!MarkSourceStatus::ExcelUnavailable.is_comparable());
    assert!(!MarkSourceStatus::WorkbookMissing.is_comparable());
}

// ── the database-side join ──────────────────────────────────────────────

mod join {
    use crate::sf2::diagnose::compare::{
        absent_count_in_month, build_date_mappings, build_student_mappings, database_x_cells,
        diff_cells, workbook_x_cells_in_scope,
    };
    use crate::sf2::diagnose::model::{AbsentRecord, RosterRow};
    use crate::sf2::diagnose::workbook_probe::RawMark;
    use crate::sf2::models::Sf2DateMappingRecord;

    fn roster() -> Vec<RosterRow> {
        vec![
            RosterRow {
                student_id: "s1".to_string(),
                workbook_name: "Alvarado, Zyron Jay  E.".to_string(),
                row_index: 8,
            },
            RosterRow {
                student_id: "s2".to_string(),
                workbook_name: "BAPTISMA,SOSDFFIA".to_string(),
                row_index: 9,
            },
        ]
    }

    fn day_grid() -> Vec<Sf2DateMappingRecord> {
        build_date_mappings("t1", "SEPTEMBER 2026", 2026, 9, &[(8, 1), (9, 2), (10, 3)])
    }

    #[test]
    fn a_september_day_grid_dates_its_own_columns() {
        let grid = day_grid();

        assert_eq!(grid.len(), 3);
        assert_eq!(grid[0].date, "2026-09-01");
        assert_eq!(grid[0].column_letter, "H");
        assert_eq!(grid[2].date, "2026-09-03");
        assert_eq!(grid[2].column_letter, "J");
    }

    #[test]
    fn a_day_number_that_is_not_a_date_is_dropped_rather_than_guessed() {
        // February 2026 has 28 days. A 31 in a February column is not a date,
        // and turning it into one would place a mark on the wrong day.
        let grid = build_date_mappings("t1", "S", 2026, 2, &[(8, 27), (9, 28), (10, 31)]);

        assert_eq!(
            grid.iter().map(|m| m.date.as_str()).collect::<Vec<_>>(),
            vec!["2026-02-27", "2026-02-28"]
        );
    }

    #[test]
    fn a_day_number_of_zero_is_dropped() {
        let grid = build_date_mappings("t1", "S", 2026, 9, &[(8, 0), (9, 1)]);

        assert_eq!(
            grid.iter().map(|m| m.date.as_str()).collect::<Vec<_>>(),
            vec!["2026-09-01"]
        );
    }

    #[test]
    fn an_absence_on_a_day_the_month_has_no_column_is_not_a_cell() {
        let absent = vec![AbsentRecord {
            student_id: "s1".to_string(),
            class_id: Some("c1".to_string()),
            date: "2026-09-04".to_string(),
        }];

        assert!(database_x_cells(&roster(), &day_grid(), &absent).is_empty());
    }

    #[test]
    fn an_absence_for_a_learner_with_no_roster_row_is_not_a_cell() {
        let absent = vec![AbsentRecord {
            student_id: "unknown".to_string(),
            class_id: Some("c1".to_string()),
            date: "2026-09-01".to_string(),
        }];

        assert!(database_x_cells(&roster(), &day_grid(), &absent).is_empty());
    }

    #[test]
    fn an_absence_the_grid_can_hold_is_a_cell() {
        let absent = vec![AbsentRecord {
            student_id: "s2".to_string(),
            class_id: Some("c1".to_string()),
            date: "2026-09-02".to_string(),
        }];

        let cells = database_x_cells(&roster(), &day_grid(), &absent);

        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].address(), "I9");
    }

    #[test]
    fn two_absence_rows_for_one_learner_and_day_are_one_cell() {
        let absent = vec![
            AbsentRecord {
                student_id: "s1".to_string(),
                class_id: Some("c1".to_string()),
                date: "2026-09-01".to_string(),
            },
            AbsentRecord {
                student_id: "s1".to_string(),
                class_id: Some("c1".to_string()),
                date: "2026-09-01".to_string(),
            },
        ];

        assert_eq!(database_x_cells(&roster(), &day_grid(), &absent).len(), 1);
    }

    #[test]
    fn the_two_sides_of_the_difference_are_named_not_just_counted() {
        let roster = roster();
        let grid = day_grid();
        let database = database_x_cells(
            &roster,
            &grid,
            &[AbsentRecord {
                student_id: "s1".to_string(),
                class_id: Some("c1".to_string()),
                date: "2026-09-01".to_string(),
            }],
        );
        let workbook = vec![
            crate::sf2::attendance_marks::Sf2GridCell {
                sheet_name: "SEPTEMBER 2026".to_string(),
                column_letter: "I".to_string(),
                row_index: 8,
            },
            crate::sf2::attendance_marks::Sf2GridCell {
                sheet_name: "SEPTEMBER 2026".to_string(),
                column_letter: "H".to_string(),
                row_index: 8,
            },
        ];

        let (only_in_workbook, only_in_database) = diff_cells(&roster, &grid, &workbook, &database);

        assert_eq!(only_in_workbook.len(), 1);
        assert_eq!(only_in_workbook[0].student_name, "Alvarado, Zyron Jay  E.");
        assert_eq!(only_in_workbook[0].date, "2026-09-02");
        assert_eq!(only_in_workbook[0].cell_address, "I8");
        assert!(only_in_database.is_empty());
    }

    #[test]
    fn a_month_count_never_spills_into_an_adjacent_month() {
        let absent = vec![
            AbsentRecord {
                student_id: "s1".to_string(),
                class_id: None,
                date: "2026-08-31".to_string(),
            },
            AbsentRecord {
                student_id: "s1".to_string(),
                class_id: None,
                date: "2026-09-01".to_string(),
            },
            AbsentRecord {
                student_id: "s1".to_string(),
                class_id: None,
                date: "2026-10-01".to_string(),
            },
        ];

        assert_eq!(absent_count_in_month(&absent, 2026, 9), 1);
        assert_eq!(absent_count_in_month(&absent, 2026, 8), 1);
        assert_eq!(absent_count_in_month(&absent, 2026, 10), 1);
        assert_eq!(absent_count_in_month(&absent, 2027, 9), 0);
    }

    #[test]
    fn student_mappings_carry_the_row_the_database_recorded() {
        let mappings = build_student_mappings("t1", &roster());

        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].row_index, 8);
        assert_eq!(mappings[0].student_id, "s1");
        assert_eq!(mappings[1].row_index, 9);
    }

    // ── the workbook side ─────────────────────────────────────────────

    fn sheet_marks(cells: &[(u32, u32)]) -> Vec<RawMark> {
        cells
            .iter()
            .map(|(row_index, column_index)| RawMark {
                row_index: *row_index,
                column_index: *column_index,
            })
            .collect()
    }

    #[test]
    fn a_mark_in_scope_is_counted() {
        let students = build_student_mappings("t1", &roster());
        let grid = day_grid();
        let scope = crate::sf2::attendance_marks::attendance_scope_cells(&students, &grid);

        let cells = workbook_x_cells_in_scope(&sheet_marks(&[(8, 8)]), "SEPTEMBER 2026", &scope);

        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].address(), "H8");
    }

    #[test]
    fn a_mark_on_a_day_the_month_has_no_column_is_outside_the_scope() {
        let students = build_student_mappings("t1", &roster());
        let grid = day_grid();
        let scope = crate::sf2::attendance_marks::attendance_scope_cells(&students, &grid);

        // Column M (13) is the second half of a merged day pair: no date behind
        // it, so no mapping covers it.
        let cells = workbook_x_cells_in_scope(
            &sheet_marks(&[(8, 13)]),
            "SEPTEMBER 2026",
            &grid_free_scope(&students),
        );

        assert_eq!(cells.len(), 0, "{scope:?}");
    }

    fn grid_free_scope(
        students: &[crate::sf2::models::Sf2StudentMappingRecord],
    ) -> Vec<crate::sf2::attendance_marks::Sf2GridCell> {
        students
            .iter()
            .map(|student| crate::sf2::attendance_marks::Sf2GridCell {
                sheet_name: "SEPTEMBER 2026".to_string(),
                column_letter: "H".to_string(),
                row_index: student.row_index,
            })
            .collect()
    }

    #[test]
    fn a_mark_on_a_row_the_roster_does_not_cover_is_outside_the_scope() {
        let students = build_student_mappings("t1", &roster());
        let scope = crate::sf2::attendance_marks::attendance_scope_cells(&students, &day_grid());

        // Row 29 is the MALE TOTAL row on the DepEd form: a formula, not a mark.
        let cells = workbook_x_cells_in_scope(&sheet_marks(&[(29, 8)]), "SEPTEMBER 2026", &scope);

        assert!(cells.is_empty());
    }

    #[test]
    fn the_same_cell_is_only_counted_once() {
        let students = build_student_mappings("t1", &roster());
        let scope = crate::sf2::attendance_marks::attendance_scope_cells(&students, &day_grid());

        let cells =
            workbook_x_cells_in_scope(&sheet_marks(&[(8, 8), (8, 8)]), "SEPTEMBER 2026", &scope);

        assert_eq!(cells.len(), 1);
    }

    #[test]
    fn a_whole_sheet_of_marks_with_an_empty_scope_counts_nothing_rather_than_everything() {
        let cells = workbook_x_cells_in_scope(
            &sheet_marks(&[(8, 8), (9, 8), (30, 8), (29, 8)]),
            "SEPTEMBER 2026",
            &[],
        );

        assert!(cells.is_empty());
    }
}

// ── zero writes ─────────────────────────────────────────────────────────

/// Proves the diagnostic wrote to nothing.
///
/// The database is hashed and its modification time read before and after a
/// full run, and the same for the workbook. Any write - an `INSERT`, a `DELETE`,
/// a `PRAGMA user_version = 22`, a workbook saved with a dirty flag - moves at
/// least one of those.
mod no_write {
    use crate::sf2::diagnose::diagnose_sf2_marks;
    use crate::sf2::diagnose::model::MappingSource;
    use rusqlite::Connection;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// A database file with the shape the diagnostic reads, and nothing else.
    ///
    /// Written with a plain connection, then *closed* before the run, so the
    /// bytes being compared are the file's own and not a live journal's.
    fn seeded_database(path: &Path) {
        let conn = Connection::open(path).expect("create the seed database");
        conn.execute_batch(
            r#"
            CREATE TABLE classes (id TEXT PRIMARY KEY, name TEXT, day_start TEXT,
                day_end TEXT, late_after TEXT, created_at INTEGER);
            CREATE TABLE students (id TEXT PRIMARY KEY, name TEXT, card_serial TEXT,
                class_id TEXT, created_at INTEGER, gender TEXT, sf2_learner_id TEXT);
            CREATE TABLE events (id TEXT PRIMARY KEY, student_id TEXT, class_id TEXT,
                event_type TEXT, timestamp INTEGER, note TEXT, session_key TEXT,
                override_reason TEXT, updated_at INTEGER);
            CREATE TABLE sf2_templates (id TEXT PRIMARY KEY, source_path TEXT,
                source_hash TEXT, school_id TEXT, school_name TEXT, school_year TEXT,
                report_month TEXT, grade_level TEXT, section TEXT, adviser_name TEXT,
                school_head_name TEXT, layout_fingerprint TEXT, active_class_id TEXT,
                imported_at INTEGER, last_synced_at INTEGER);
            CREATE TABLE sf2_student_mappings (template_id TEXT, student_id TEXT,
                workbook_name TEXT, normalized_name TEXT, row_index INTEGER,
                gender_block TEXT, PRIMARY KEY (template_id, student_id));
            CREATE TABLE sf2_date_mappings (template_id TEXT, sheet_name TEXT,
                date TEXT, column_letter TEXT, column_index INTEGER,
                PRIMARY KEY (template_id, date));
            CREATE TABLE sf2_month_templates (id TEXT PRIMARY KEY, active_class_id TEXT,
                school_year TEXT, report_month TEXT, report_year INTEGER,
                source_path TEXT, source_hash TEXT, school_id TEXT, school_name TEXT,
                grade_level TEXT, section TEXT, adviser_name TEXT, school_head_name TEXT,
                first_school_day INTEGER, imported_at INTEGER, last_synced_at INTEGER,
                workbook_x_count INTEGER, workbook_scanned_at INTEGER,
                first_school_day_override INTEGER);
            CREATE TABLE sf2_month_student_mappings (template_id TEXT, student_id TEXT,
                workbook_name TEXT, normalized_name TEXT, row_index INTEGER,
                gender_block TEXT, sf2_learner_id TEXT,
                PRIMARY KEY (template_id, student_id));
            CREATE TABLE sf2_month_date_mappings (template_id TEXT, date TEXT,
                column_letter TEXT, column_index INTEGER,
                PRIMARY KEY (template_id, date));

            INSERT INTO classes VALUES ('c1', 'Grade 3 - MATAPAT', '08:00', '15:00',
                '08:45', 1);
            INSERT INTO students VALUES ('s1', 'Alvarado, Zyron Jay  E.', NULL, 'c1', 1,
                'male', NULL);
            INSERT INTO students VALUES ('s2', 'BAPTISMA,SOSDFFIA', NULL, 'c1', 1,
                'male', NULL);
            INSERT INTO sf2_templates VALUES
                ('t1', 'C:\\nowhere\\SF2-GRADE-3-MATAPAT-0000t1.xls', 'hash', '132839',
                 'Espiritu Elementary School', '2026 - 2027', 'OCTOBER', 'Grade 3',
                 'MATAPAT', 'ADVISER', 'HEAD', 'fingerprint', 'c1', 1, 1);
            INSERT INTO sf2_student_mappings VALUES
                ('t1', 's1', 'Alvarado, Zyron Jay  E.', 'ALVARADO,ZYRON JAY E.', 8, 'MALE');
            INSERT INTO sf2_student_mappings VALUES
                ('t1', 's2', 'BAPTISMA,SOSDFFIA', 'BAPTISMA,SOSDFFIA', 9, 'MALE');
            INSERT INTO sf2_date_mappings VALUES
                ('t1', 'OCTOBER 2026', '2026-10-01', 'J', 10);
            INSERT INTO sf2_date_mappings VALUES
                ('t1', 'OCTOBER 2026', '2026-10-02', 'K', 11);
            INSERT INTO sf2_month_templates
                (id, active_class_id, school_year, report_month, report_year,
                 source_path, source_hash, school_id, school_name, grade_level,
                 section, adviser_name, school_head_name, first_school_day,
                 imported_at, last_synced_at, workbook_x_count,
                 workbook_scanned_at, first_school_day_override)
            VALUES
                ('t1', 'c1', '2026 - 2027', 'OCTOBER', 2026,
                 'C:\\nowhere\\SF2-GRADE-3-MATAPAT-0000t1.xls', 'hash', '132839',
                 'Espiritu Elementary School', 'Grade 3', 'MATAPAT', 'ADVISER',
                 'HEAD', 1, 1, NULL, 0, NULL, NULL);
            INSERT INTO sf2_month_student_mappings VALUES
                ('t1', 's1', 'Alvarado, Zyron Jay  E.', 'ALVARADO,ZYRON JAY E.', 8, 'MALE',
                 NULL);
            INSERT INTO sf2_month_date_mappings VALUES
                ('t1', '2026-10-01', 'J', 10);
            INSERT INTO sf2_month_date_mappings VALUES
                ('t1', '2026-10-02', 'K', 11);
            INSERT INTO events VALUES ('e1', 's1', 'c1', 'absent', 1790000000, NULL,
                NULL, NULL, NULL);
            -- An absence for a class the diagnostic was not told about, and one
            -- with no class at all. Both name a student who *is* in the class,
            -- so both are still attributable - the app's own rule is
            -- `event_belongs_to_class`, and it accepts either the class or the
            -- membership. Only the last one is attributable to nothing.
            INSERT INTO events VALUES ('e2', 's1', 'other-class', 'absent', 1790000100,
                NULL, NULL, NULL, NULL);
            INSERT INTO events VALUES ('e3', 's1', NULL, 'absent', 1790000200, NULL,
                NULL, NULL, NULL);
            INSERT INTO students VALUES ('s3', 'Dela Cruz, Juan', NULL, 'other-class', 1,
                'male', NULL);
            INSERT INTO events VALUES ('e5', 's3', 'other-class', 'absent', 1790000400,
                NULL, NULL, NULL, NULL);
            INSERT INTO events VALUES ('e4', 's2', 'c1', 'in', 1790000300, NULL,
                NULL, NULL, NULL);
            PRAGMA user_version = 18;
            "#,
        )
        .expect("seed the schema");
    }

    /// A scratch directory that cleans itself up.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("ees-ams-diagnose-{label}-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&path).expect("create the scratch directory");
            Self(path)
        }

        fn db(&self) -> PathBuf {
            self.0.join("attendance.db")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Bytes and modification time, the pair a write cannot leave both of.
    fn fingerprint(path: &Path) -> (u64, u64, std::time::SystemTime) {
        let bytes = fs::read(path).expect("read the file");
        let length = bytes.len() as u64;
        let modified = fs::metadata(path)
            .expect("stat the file")
            .modified()
            .expect("read the modification time");
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in &bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        (length, hash, modified)
    }

    /// A full diagnostic run leaves the database file byte-for-byte identical.
    ///
    /// The database is named at a path that does not exist, so the diagnostic
    /// runs its real code - including the read-only open, every roster and grid
    /// read, and the whole twelve-month loop - and finds a workbook missing for
    /// every month. That is the ordinary "no workbook yet" shape, and it is the
    /// run that must not touch the file.
    ///
    /// The seed carries rows in *both* the legacy and the per-month mapping
    /// tables, which is what makes this the test that catches a `SELECT` whose
    /// column list does not match the Rust reader beside it: that failure looks
    /// like an unrelated column *type* error, and it happened twice while this
    /// was being written.
    #[test]
    fn a_full_run_does_not_write_to_the_database() {
        let scratch = Scratch::new("no-write-db");
        let db_path = scratch.db();
        seeded_database(&db_path);

        let before = fingerprint(&db_path);
        let diagnostic = diagnose_sf2_marks(&db_path).expect("the diagnostic runs");
        let after = fingerprint(&db_path);

        assert_eq!(before, after, "the diagnostic changed the database file");
        assert_eq!(diagnostic.total_absent_events, 4, "e1, e2, e3 and e5");
        assert_eq!(diagnostic.schema_version, Some(18));
        assert_eq!(
            diagnostic.absent_events_without_class, 1,
            "e5 is for a student of another class with no class of its own that \
             matches, so nothing will ever place it on this workbook's grid"
        );
        assert_eq!(
            diagnostic
                .tables
                .legacy
                .iter()
                .map(|table| (table.table.as_str(), table.rows))
                .collect::<Vec<_>>(),
            vec![
                ("sf2_templates", Some(1)),
                ("sf2_student_mappings", Some(2)),
                ("sf2_date_mappings", Some(2)),
            ]
        );
        assert_eq!(
            diagnostic
                .tables
                .per_month
                .iter()
                .map(|table| (table.table.as_str(), table.rows))
                .collect::<Vec<_>>(),
            vec![
                ("sf2_month_templates", Some(1)),
                ("sf2_month_student_mappings", Some(1)),
                ("sf2_month_date_mappings", Some(2)),
            ]
        );
        assert_eq!(
            diagnostic.tables.legacy_date_mapping_sheets[0].day_columns, 2,
            "the legacy grid reader and the per-month grid reader share one Rust \
             reader, so both column lists have to match it"
        );
        assert_eq!(diagnostic.tables.month_date_mapping_grids[0].day_columns, 2);
        assert_eq!(diagnostic.mapping_source, MappingSource::PerMonthTables);
    }

    /// The read-only handle is the reason, and it is asserted directly: a
    /// write statement on the connection the diagnostic uses is refused.
    #[test]
    fn the_diagnostic_connection_refuses_writes() {
        let scratch = Scratch::new("read-only-conn");
        let db_path = scratch.db();
        seeded_database(&db_path);

        let conn = Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .expect("open read-only, as db_read does");

        let refused = conn
            .execute("DELETE FROM events WHERE id = 'e1'", [])
            .is_err();
        let refused_update = conn
            .execute("UPDATE students SET name = 'x' WHERE id = 's1'", [])
            .is_err();
        let refused_migration = conn.execute("PRAGMA user_version = 22", []).is_err();

        assert!(refused, "a DELETE must be refused");
        assert!(refused_update, "an UPDATE must be refused");
        assert!(refused_migration, "a PRAGMA write must be refused");
    }

    /// Every statement the diagnostic's read side uses is a `SELECT` or a
    /// `PRAGMA` that only reads. A grep is the check; this is the assertion that
    /// the grep keeps meaning what it says.
    #[test]
    fn no_statement_in_the_read_side_writes() {
        let sql_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/sf2/diagnose/sql");
        let mut checked = 0;
        for entry in fs::read_dir(sql_dir).expect("the diagnostic's sql directory") {
            let path = entry.expect("a directory entry").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("sql") {
                continue;
            }
            let body = fs::read_to_string(&path).expect("read the statement");
            let statements: String = body
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .to_ascii_uppercase();
            for forbidden in [
                "INSERT",
                "UPDATE ",
                "DELETE",
                "DROP ",
                "ALTER ",
                "REPLACE",
                "CREATE",
                "VACUUM",
                "REINDEX",
                "ATTACH",
                "DETACH",
                "PRAGMA USER_VERSION =",
                "JOURNAL_MODE",
                "WAL",
            ] {
                assert!(
                    !statements.contains(forbidden),
                    "{} contains `{forbidden}`",
                    path.display()
                );
            }
            checked += 1;
        }
        assert!(
            checked >= 10,
            "expected the whole read side to be checked, only saw {checked} statement file(s)"
        );
    }
}
