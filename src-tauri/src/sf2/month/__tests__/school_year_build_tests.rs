//! An end-to-end check of the twelve-sheet build, against a real workbook.
//!
//! This is the one test that exercises the whole COM path the app depends on:
//! add twelve worksheets, copy the form layout onto each, empty it, write the day
//! grid, the header, the roster and the marks, delete everything that is not one
//! of the twelve, make them all visible, and save.
//!
//! It runs against a **copy** of the bundled template in a temp directory. It
//! never touches an install.

use super::*;
use crate::sf2::month::merge::female_block_start;
use crate::sf2::month::workbook_builder::{
    build_school_year_workbook, MonthAbsence, MonthBuildRequest, MonthHeader, MonthLearnerWrite,
    MonthSheetBuild,
};
use crate::sf2::month::workbook_sheets::month_sheet_name;
use std::path::Path;

fn scratch_workbook() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("SF2-1-A-a4f5d22a.xls");
    std::fs::write(&path, crate::sf2::workbook_files::BUNDLED_TEMPLATE_BYTES)
        .expect("write the bundled template");
    (dir, path)
}

fn learner(
    student_id: &str,
    row_index: u32,
    name: &str,
    item: u32,
    block: &str,
) -> MonthLearnerWrite {
    MonthLearnerWrite {
        student_id: student_id.to_string(),
        row_index,
        name: name.to_string(),
        item_number: item,
        gender_block: Some(block.to_string()),
    }
}

fn request_for(
    month: &str,
    year: i32,
    first_school_day: u32,
    learners: Vec<MonthLearnerWrite>,
    absences: Vec<MonthAbsence>,
) -> MonthSheetBuild {
    MonthSheetBuild {
        request: MonthBuildRequest {
            template_id: format!("template-{month}"),
            report_month: month.to_string(),
            report_year: year,
            first_school_day,
            header: MonthHeader {
                school_id: "132839".to_string(),
                school_name: "Espiritu Elementary School".to_string(),
                school_year: "2026-2027".to_string(),
                report_month: month.to_string(),
                grade_level: "Grade 3".to_string(),
                section: "MATAPAT".to_string(),
                adviser_name: "ALISTAIR M YBANEZ".to_string(),
                school_head_name: "ARNYL R. ARONES".to_string(),
            },
            learners,
            absences,
            source_female_start_row: 0,
        },
        remove_stale_sheets: true,
    }
}

/// The roster the affected install has: 17 males from row 8, 10 females from
/// row 30, which is what `female_block_start(17)` puts the second block on.
fn roster() -> Vec<MonthLearnerWrite> {
    let mut learners = Vec::new();
    for index in 0..17u32 {
        learners.push(learner(
            &format!("m{index}"),
            8 + index,
            &format!("DELA CRUZ,JUAN,{index}"),
            index + 1,
            "MALE",
        ));
    }
    for index in 0..10u32 {
        learners.push(learner(
            &format!("f{index}"),
            female_block_start(17) + index,
            &format!("SANTOS,MARIA,{index}"),
            index + 1,
            "FEMALE",
        ));
    }
    learners
}

fn school_year_builds() -> Vec<MonthSheetBuild> {
    crate::sf2::workbook_files::school_year_month_files("2026-2027", 2026)
        .into_iter()
        .map(|(month, year)| {
            let month_number = crate::sf2::calendar::sf2_month_number(&month).unwrap_or_default();
            // Only SEPTEMBER and DECEMBER carry absences here; the other ten must
            // come out with a grid and no marks, which is the whole of the "the
            // nine missing months" requirement.
            let absences = if month == "SEPTEMBER" {
                vec![
                    MonthAbsence {
                        student_id: "m0".to_string(),
                        date: format!("{year:04}-09-01"),
                    },
                    MonthAbsence {
                        student_id: "f0".to_string(),
                        date: format!("{year:04}-09-02"),
                    },
                    MonthAbsence {
                        student_id: "m3".to_string(),
                        date: format!("{year:04}-09-16"),
                    },
                ]
            } else {
                Vec::new()
            };
            request_for(
                &month,
                year,
                // An undated month is anchored on the 1st, so every recorded
                // absence has a column. See `merge::grid_anchor_day`.
                crate::sf2::month::merge::grid_anchor_day(None, year, month_number),
                roster(),
                absences,
            )
        })
        .collect()
}

#[test]
fn the_build_writes_twelve_visible_month_worksheets_and_removes_everything_else() {
    let (_dir, path) = scratch_workbook();

    let report =
        build_school_year_workbook(&path, &school_year_builds()).expect("build the school year");

    assert!(
        report.verification.is_verified(),
        "the build did not verify: {:?}",
        report.verification.mismatch_reason()
    );
    assert_eq!(report.months.len(), 12, "twelve months of a school year");
    assert_eq!(
        report.total_marks(),
        3,
        "only September has absences, and it has three"
    );

    // The names are one per month, in school-year order, and each carries its own
    // grid.
    let names = report.sheet_names();
    assert_eq!(names.first().map(String::as_str), Some("SEPTEMBER 2026"));
    assert_eq!(names.last().map(String::as_str), Some("AUGUST 2027"));
    for ((month, year), build) in
        crate::sf2::workbook_files::school_year_month_files("2026-2027", 2026)
            .iter()
            .zip(&report.months)
    {
        let month_number = crate::sf2::calendar::sf2_month_number(month).unwrap_or_default();
        let expected = month_sheet_name(month_number, *year);
        assert_eq!(build.sheet_name, expected, "{month} is on the wrong sheet");
        assert!(
            !build.dates.is_empty(),
            "{month} came out with no day columns, which is the state SEPTEMBER 2026 was in"
        );
        for date in &build.dates {
            assert_eq!(
                date.resolved_sheet_name(),
                expected,
                "{} is recorded against the wrong worksheet",
                date.date
            );
        }
    }

    // The template's own sample sheets, and its 36 sample marks, are gone.
    assert!(
        report.removed_sheets.iter().any(|name| name == "JUNE 2025"),
        "the bundled template's sample sheet must not survive: {:?}",
        report.removed_sheets
    );
    // `COMPLETE DAYS` is *not* a helper, and treating it as one is a trap worth
    // naming: the bundled template copies the whole SF2 form onto it - same title,
    // 173 formulas, sample marks - rather than holding anything this app reads. It
    // carries no cross-sheet formula from any month sheet, so removing it breaks
    // nothing, and keeping it would leave a sixth sheet of a fictional class that
    // no read path can reach but that a teacher opening the file would see.
    assert!(
        report
            .removed_sheets
            .iter()
            .any(|name| name == "COMPLETE DAYS"),
        "the template's second copy of the form must not survive: {:?}",
        report.removed_sheets
    );
    assert!(
        report.kept_helper_sheets.is_empty(),
        "nothing in the bundled template is a real helper sheet: {:?}",
        report.kept_helper_sheets
    );

    // And the file on disk really has twelve visible month sheets and no hidden
    // ones. Read back through Excel, because that is what the user will open.
    let written = read_sheet_names(&path);
    assert_eq!(written.len(), 12, "twelve months and nothing else");
    for (name, visible) in &written {
        if name == "COMPLETE DAYS" {
            continue;
        }
        assert!(visible, "`{name}` is not visible");
        assert!(
            !is_hidden_sheet_name(name),
            "`{name}` is a leftover of the retired hide/rename/clear cycle"
        );
    }
    for month in crate::sf2::workbook_files::school_year_month_files("2026-2027", 2026) {
        let (name, year) = month;
        let expected = month_sheet_name(
            crate::sf2::calendar::sf2_month_number(&name).unwrap_or_default(),
            year,
        );
        assert!(
            written.iter().any(|(sheet, _)| *sheet == expected),
            "`{expected}` is not in the file"
        );
    }
}

#[test]
fn a_built_worksheet_carries_its_own_roster_and_marks_and_no_others() {
    let (_dir, path) = scratch_workbook();
    build_school_year_workbook(&path, &school_year_builds()).expect("build");

    // The two months that have absences, and one that does not. Reading them back
    // through the same bulk read the guard uses is the only way to be sure the
    // marks landed on the right sheet rather than merely being written somewhere.
    let september = count_marks_on(&path, "SEPTEMBER 2026");
    let october = count_marks_on(&path, "OCTOBER 2026");
    let december = count_marks_on(&path, "DECEMBER 2026");
    assert!(
        september > 0,
        "September's three absences are not on its own worksheet"
    );
    assert_eq!(october, 0, "October has no absences and must have no marks");
    assert_eq!(
        december, 0,
        "December has no absences and must have no marks"
    );

    // The roster is on every sheet, so one row index means one learner on all
    // twelve - which is what lets a single roster mapping set serve them all.
    for name in ["SEPTEMBER 2026", "AUGUST 2027"] {
        let learners = read_learner_names(&path, name);
        assert_eq!(
            learners.len(),
            roster().len(),
            "{name} has the wrong roster"
        );
        assert!(learners.contains(&"DELA CRUZ,JUAN,0".to_string()));
        assert!(learners.contains(&"SANTOS,MARIA,0".to_string()));
    }
}

/// Every worksheet in `path`, with whether it is visible.
fn read_sheet_names(path: &Path) -> Vec<(String, bool)> {
    crate::sf2::month::workbook_sheets::read_sheet_names(path)
        .expect("read the written workbook's sheets")
}

/// How many `X` marks the named worksheet's learner rows hold.
fn count_marks_on(path: &Path, sheet_name: &str) -> usize {
    crate::sf2::month::workbook_sheets::count_absent_marks_on_sheet(path, sheet_name)
        .expect("count the marks on one worksheet")
}

/// The learner names on the named worksheet, in row order.
fn read_learner_names(path: &Path, sheet_name: &str) -> Vec<String> {
    crate::sf2::month::workbook_sheets::learner_names_on_sheet(path, sheet_name)
        .expect("read the roster off one worksheet")
}

// ── The affected install, on a copy ─────────────────────────────────────────

/// Rebuild the real install's school year, on a copy, and prove it comes out.
///
/// `#[ignore]`d and pointed at the real files by
/// `EES_AMS_REAL_WORKBOOK` / `EES_AMS_REAL_DB`, because it needs *that* install's
/// 27-learner roster and its 37 absences to be worth running - a fixture would
/// only prove the fixture again. It **copies** the workbook first and builds the
/// copy, so the file the teacher depends on is never opened for writing.
///
/// ```text
/// $env:EES_AMS_REAL_WORKBOOK="$env:APPDATA\com.ees.ams\sf2-workbooks\SF2-GRADE-3-MATAPAT-3b635890.xls"
/// $env:EES_AMS_REAL_DB="$env:APPDATA\com.ees.ams\attendance.db"
/// cargo test --lib -- --ignored real_install
/// ```
#[test]
#[ignore = "needs the real install's workbook and database"]
fn the_real_install_rebuilds_on_a_copy() {
    let Ok(source) = std::env::var("EES_AMS_REAL_WORKBOOK") else {
        panic!("EES_AMS_REAL_WORKBOOK must point at the install's workbook");
    };
    let database = std::env::var("EES_AMS_REAL_DB")
        .expect("EES_AMS_REAL_DB must point at the install's database");
    let source = Path::new(&source);

    // The absences, read straight out of the install's own `events`, so the month
    // totals this proves are the teacher's real ones. `timestamp` is UTC epoch
    // seconds and `event_type` is the absence flag, exactly as the repository
    // reads them - a date column does not exist and inventing one would prove
    // nothing.
    let conn = rusqlite::Connection::open_with_flags(
        &database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open the install's database read-only");
    let mut statement = conn
        .prepare(
            "SELECT student_id, timestamp FROM events \
             WHERE event_type = 'absent' ORDER BY timestamp",
        )
        .expect("read the absences");
    let absences = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .expect("list the absences")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read the absences")
        .into_iter()
        .map(|(student_id, timestamp)| MonthAbsence {
            student_id,
            date: chrono::DateTime::from_timestamp(timestamp, 0)
                .expect("the install holds a sane timestamp")
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d")
                .to_string(),
        })
        .collect::<Vec<_>>();
    assert!(
        absences.len() >= 30,
        "the install holds {} absences, which is not the data this was written for",
        absences.len()
    );

    // A copy, so the build's save cannot touch the file the school depends on.
    let dir = tempfile::tempdir().expect("temp dir");
    let copy = dir.path().join("copy.xls");
    std::fs::copy(source, &copy).expect("copy the workbook");
    let original_bytes = std::fs::read(source).expect("read the original for comparison");

    // The real roster, from the install's own roster mapping - the table that
    // says which app student is on which row of the form. Reading the rows off the
    // sheet instead would be circular: a build that put the roster on the wrong
    // rows would be graded against the rows it chose.
    let mut roster_statement = conn
        .prepare(
            "SELECT student_id, row_index, workbook_name, gender_block \
             FROM sf2_month_student_mappings ORDER BY row_index",
        )
        .expect("read the real roster mapping");
    let writes = roster_statement
        .query_map([], |row| {
            Ok(crate::sf2::month::workbook_builder::MonthLearnerWrite {
                student_id: row.get(0)?,
                row_index: row.get::<_, i64>(1)? as u32,
                name: row.get(2)?,
                item_number: 0,
                gender_block: row.get(3)?,
            })
        })
        .expect("list the real roster mapping")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("read the real roster mapping");
    let writes = crate::sf2::month::merge::number_the_roster(writes);
    assert!(
        writes.len() >= 25,
        "the install maps {} learners",
        writes.len()
    );

    let builds = crate::sf2::workbook_files::school_year_month_files("2026-2027", 2026)
        .iter()
        .map(|(month, year)| {
            let month_number = crate::sf2::calendar::sf2_month_number(month).unwrap_or_default();
            let month_absences = absences
                .iter()
                .filter(|absence| {
                    absence
                        .date
                        .starts_with(&format!("{year:04}-{:02}", month_number))
                })
                .cloned()
                .collect::<Vec<_>>();
            request_for(
                month,
                *year,
                // The app's own anchor rule with no stored day: every recorded
                // absence gets a column, which is what makes this a check of the
                // real roster and the real marks rather than of the school
                // calendar.
                crate::sf2::month::merge::grid_anchor_day(None, *year, month_number),
                writes.clone(),
                month_absences,
            )
        })
        .collect::<Vec<_>>();

    let report = build_school_year_workbook(&copy, &builds).expect("build the real school year");
    assert!(
        report.verification.is_verified(),
        "the real install did not verify: {:?}",
        report.verification.mismatch_reason()
    );
    assert_eq!(report.months.len(), 12);
    assert!(
        report.total_marks() > 0,
        "the install's {} absences produced no marks at all",
        absences.len()
    );

    // The twelve worksheets, and nothing of the old design left behind.
    let written = read_sheet_names(&copy);
    assert_eq!(written.len(), 12, "{written:?}");
    for (name, visible) in &written {
        assert!(visible, "`{name}` is not visible");
        assert!(!is_hidden_sheet_name(name), "`{name}` is a leftover");
    }
    assert!(
        report
            .removed_sheets
            .iter()
            .any(|name| name == "__SF2_HIDDEN_1"),
        "the retired hide/rename leftovers are gone: {:?}",
        report.removed_sheets
    );

    // Every mark is on the worksheet of its own month.
    for month in &report.months {
        let expected = month.written_marks;
        assert_eq!(
            crate::sf2::month::workbook_sheets::count_absent_marks_on_sheet(
                &copy,
                &month.sheet_name
            )
            .unwrap_or_default(),
            expected,
            "{} does not hold its own {} marks",
            month.sheet_name,
            expected
        );
    }

    // And the install's own file was never opened for writing.
    assert_eq!(
        std::fs::read(source).expect("re-read the original"),
        original_bytes,
        "the build modified the install's workbook"
    );
}
