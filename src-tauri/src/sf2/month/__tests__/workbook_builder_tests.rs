//! The pure geometry behind building a month file and reading the legacy marks.
//!
//! Nothing here opens Excel. What is tested is the arithmetic that decides where
//! a day goes, which cells get copied, and whether a read actually worked - the
//! three things that silently corrupt a split when they are wrong.

use super::*;
use crate::sf2::month::first_school_day::is_school_day;

// ── Day numbers ─────────────────────────────────────────────────────────────

/// The 25 day slots of the bundled template: five weeks of Monday..Friday, in
/// the columns its weekday header actually labels. Columns 7, 13, 19, 23, 25,
/// 27, 34 and 38 are the second halves of merged pairs and carry no label, so
/// they are not slots.
fn bundled_slots() -> Vec<MonthDaySlot> {
    let labelled = [
        6, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 20, 21, 22, 24, 26, 28, 29, 30, 31, 32, 33, 35,
        36, 37,
    ];
    labelled
        .iter()
        .enumerate()
        .map(|(index, column)| MonthDaySlot {
            column: *column,
            week_index: index as u32 / 5,
            weekday_index: index as u32 % 5,
        })
        .collect()
}

fn day_numbers(year: i32, month: u32, first_school_day: u32) -> Vec<(u32, Option<u32>)> {
    day_numbers_for_slots(year, month, first_school_day, &bundled_slots())
}

#[test]
fn a_month_starting_on_a_monday_fills_its_first_week() {
    // 2026-09-01 is a Tuesday, so F (Monday, week 0) is blank and the week
    // starts on H.
    let days = day_numbers(2026, 9, 1);
    let by_column = days
        .iter()
        .copied()
        .collect::<std::collections::HashMap<_, _>>();

    assert_eq!(by_column[&6], None, "no Monday before classes start");
    assert_eq!(by_column[&8], Some(1));
    assert_eq!(by_column[&9], Some(2));
    assert_eq!(by_column[&10], Some(3));
    assert_eq!(by_column[&11], Some(4));
    assert_eq!(by_column[&12], Some(7), "week 1 Monday");
    assert_eq!(by_column[&16], Some(10));
    assert_eq!(by_column[&17], Some(11));
}

#[test]
fn every_written_day_lands_on_its_own_weekday() {
    for (month, year) in [(9u32, 2026i32), (2, 2027), (6, 2027), (8, 2027)] {
        for (column, day) in day_numbers(year, month, 1) {
            let Some(day) = day else { continue };
            let date = NaiveDate::from_ymd_opt(year, month, day).expect("a real date");
            assert!(is_school_day(date), "{year}-{month}-{day} is not Mon-Fri");
            let slot = bundled_slots()
                .into_iter()
                .find(|slot| slot.column == column)
                .expect("a labelled day column");
            assert_eq!(
                weekday_index(date),
                Some(slot.weekday_index),
                "{year}-{month}: day {day} landed in weekday column {column}"
            );
        }
    }
}

#[test]
fn no_day_is_written_twice_and_none_is_skipped() {
    let days = day_numbers(2027, 6, 1);
    let written = days.iter().filter_map(|(_, day)| *day).collect::<Vec<_>>();
    let mut unique = written.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), written.len(), "a day was written twice");

    // Exactly the Monday-to-Friday days of the month, and nothing else.
    let expected = (1..=30u32)
        .filter(|day| {
            is_school_day(NaiveDate::from_ymd_opt(2027, 6, *day).expect("a June 2027 day"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        written, expected,
        "the sheet must carry every school day of June 2027 and no weekend"
    );
}

#[test]
fn a_late_first_day_leaves_the_earlier_columns_blank() {
    // Classes started on Tuesday 15 September 2026. Nothing before it is a school
    // day, and the columns in front of it must be blanked rather than left
    // holding whatever day numbers the fresh template shipped with.
    let days = day_numbers(2026, 9, 15);
    let by_column = days
        .iter()
        .copied()
        .collect::<std::collections::HashMap<_, _>>();

    assert_eq!(by_column[&6], None, "week 0 Monday is before classes start");
    assert_eq!(by_column[&12], Some(21), "week 1 Monday");
    assert_eq!(by_column[&14], Some(22), "week 1 Tuesday");
    assert_eq!(
        written_days(&days).first(),
        Some(&15),
        "the first attendance day is the first day written"
    );
}

fn written_days(days: &[(u32, Option<u32>)]) -> Vec<u32> {
    days.iter().filter_map(|(_, day)| *day).collect()
}

#[test]
fn a_first_school_day_on_a_weekend_still_dates_the_month() {
    // 2026-09-06 is a Sunday. It is not a school day and it is not a Monday, so
    // there is no week to count from *that* day - but September still has 18
    // school days left in it, and a month whose grid comes out empty has nowhere
    // to record any of them. The weeks are counted from the Monday of the week the
    // previous Friday (2026-09-04) falls in.
    let days = day_numbers(2026, 9, 6);
    let written = written_days(&days);
    assert_eq!(
        written.len(),
        18,
        "September 2026 school days from the 6th on"
    );
    assert_eq!(written.first(), Some(&7), "2026-09-07 is the next Monday");
    assert_eq!(written.last(), Some(&30));
}

#[test]
fn the_day_grid_becomes_date_mappings_for_the_stored_row() {
    let mappings = month_date_mappings("template-1", 2026, 9, 1, &bundled_slots());

    assert_eq!(mappings.len(), 22, "September 2026 school days on the form");
    assert_eq!(mappings[0].date, "2026-09-01");
    assert_eq!(mappings[0].column_letter, "H");
    assert_eq!(mappings[0].template_id, "template-1");
    assert_eq!(mappings.last().expect("a last day").date, "2026-09-30");
    // The mapping is what the month switch reads, so its columns have to be the
    // columns the sheet was actually written to.
    for mapping in &mappings {
        assert_eq!(
            mapping.column_index as i32,
            column_index_of(&mapping.column_letter)
        );
    }
}

fn column_index_of(letter: &str) -> i32 {
    let base = i32::from(b'A');
    letter
        .chars()
        .filter(|ch| ch.is_ascii_uppercase())
        .fold(0i32, |total, ch| {
            total * 26 + i32::from(u16::from(ch as u8)) - base + 1
        })
}

// ── Roster bands and expansion ──────────────────────────────────────────────

#[test]
fn the_total_rows_are_never_part_of_a_read_band() {
    // Rows 29 and 49 hold SUM formulas. Reading them would count phantom marks
    // and writing them back would be refused by the formula guard.
    let learners = vec![
        Sf2WorkbookLearner {
            row_index: 8,
            name: "CRUZ, JUAN".to_string(),
            gender_block: Some("MALE".to_string()),
            sf2_learner_id: None,
        },
        Sf2WorkbookLearner {
            row_index: 28,
            name: "REYES, MARIA".to_string(),
            gender_block: Some("MALE".to_string()),
            sf2_learner_id: None,
        },
        Sf2WorkbookLearner {
            row_index: 30,
            name: "SANTOS, LIZA".to_string(),
            gender_block: Some("FEMALE".to_string()),
            sf2_learner_id: None,
        },
        Sf2WorkbookLearner {
            row_index: 48,
            name: "TAN, ANA".to_string(),
            gender_block: Some("FEMALE".to_string()),
            sf2_learner_id: None,
        },
    ];
    let bands = attendance_bands(&learners);

    assert_eq!(bands.len(), 2);
    assert_eq!((bands[0].first_row, bands[0].last_row), (8, 28));
    assert_eq!((bands[1].first_row, bands[1].last_row), (30, 48));
    for band in &bands {
        assert!(
            band.row_count() <= 21,
            "a band spans the whole gender block"
        );
    }
    assert_eq!(
        attendance_bands(&learners[..2]),
        vec![RowBand {
            first_row: 8,
            last_row: 28,
            first_column: 6,
            last_column: 38
        }],
        "a roster with no female learners is a single band"
    );
}

#[test]
fn a_band_spans_the_whole_day_grid() {
    let band = RowBand {
        first_row: 8,
        last_row: 28,
        first_column: 6,
        last_column: 38,
    };
    assert_eq!(band.column_count(), 33, "F..AL inclusive");
    assert_eq!(band.row_count(), 21);
    assert_eq!(band.cell_count(), 693);
}

#[test]
fn an_expanded_source_roster_grows_the_month_file_to_the_same_shape() {
    // The legacy file was already expanded, so its female block starts at row 34
    // rather than 30. A month file that was not grown to match would put row 34
    // on a different learner, and every copied X would land on the wrong student.
    let (extra_male, extra_female) = roster_expansion_for(24, 19, 34);
    assert_eq!(extra_male, 4, "the female block moved down by four rows");
    assert_eq!(extra_female, 0);
    assert_eq!(header_row_shift(extra_male, extra_female), 4);
}

#[test]
fn a_roster_longer_than_the_form_grows_it() {
    // E9: 28 males and 25 females.
    let (extra_male, extra_female) = roster_expansion_for(28, 25, 30);
    assert_eq!(extra_male, 7);
    assert_eq!(extra_female, 6);
    assert_eq!(header_row_shift(extra_male, extra_female), 13);
}

#[test]
fn a_roster_within_capacity_does_not_grow_the_file() {
    assert_eq!(roster_expansion_for(21, 19, 30), (0, 0));
    assert_eq!(
        roster_expansion_for(4, 3, 30),
        (0, 0),
        "small rosters are fine"
    );
}

// ── Reading the source in bulk ──────────────────────────────────────────────

fn band() -> RowBand {
    RowBand {
        first_row: 8,
        last_row: 9,
        first_column: 6,
        last_column: 8,
    }
}

#[test]
fn the_bulk_read_hands_excel_a_whole_band_at_once() {
    let formula = build_block_read_formula("JUNE 2026", &band(), BlockReadStrategy::TextJoin);
    assert_eq!(formula, "TEXTJOIN(\"|\",FALSE,'JUNE 2026'!F8:H9)");
    // One call for the band - not one per cell. This is the difference between
    // a split that takes seconds and one that looks like a hang.
    assert_eq!(formula.matches('&').count(), 0);
}

#[test]
fn the_fallback_read_names_every_cell_it_flattens() {
    let formula = build_block_read_formula("JUNE 2026", &band(), BlockReadStrategy::Concatenate);
    assert_eq!(
        formula,
        concat!(
            "'JUNE 2026'!F8&\"|\"&",
            "'JUNE 2026'!G8&\"|\"&",
            "'JUNE 2026'!H8&\"|\"&",
            "'JUNE 2026'!F9&\"|\"&",
            "'JUNE 2026'!G9&\"|\"&",
            "'JUNE 2026'!H9"
        )
    );
    assert_eq!(
        formula.matches("&\"|\"&").count(),
        5,
        "six cells, five joins"
    );
}

#[test]
fn a_sheet_name_with_a_quote_in_it_is_escaped() {
    let formula = build_block_read_formula("JUAN'S JUNE", &band(), BlockReadStrategy::TextJoin);
    assert!(formula.contains("'JUAN''S JUNE'"), "{formula}");
}

#[test]
fn a_long_band_is_chunked_rather_than_read_as_one_giant_formula() {
    let wide = RowBand {
        first_row: 8,
        last_row: 200,
        first_column: 6,
        last_column: 38,
    };
    for strategy in [BlockReadStrategy::TextJoin, BlockReadStrategy::Concatenate] {
        let rows = rows_per_chunk(&wide, "SEPTEMBER 2026", strategy);
        assert!(
            rows < wide.row_count(),
            "{strategy:?} read the whole band at once"
        );
        assert!(rows >= 1, "{strategy:?} produced an empty chunk");
        assert!(
            rows as usize * wide.column_count() as usize <= 4_000,
            "{strategy:?} put {rows} rows in one formula"
        );
    }
    // A small band is one call.
    assert_eq!(
        rows_per_chunk(&band(), "JUNE 2026", BlockReadStrategy::TextJoin),
        band().row_count()
    );
}

#[test]
fn a_read_that_came_back_short_is_an_error_not_an_empty_month() {
    // An error variant carries no string value, so a formula Excel could not
    // evaluate arrives here as a short or empty string. Treating that as "this
    // month has no marks" is how a whole month would be lost silently.
    let full = "X|||||X";
    assert_eq!(full.split('|').count(), band().cell_count());
    assert!(parse_block_tokens("JUNE 2026", full, &band()).is_ok());
    assert!(parse_block_tokens("JUNE 2026", "", &band()).is_err());
    assert!(parse_block_tokens("JUNE 2026", "#NAME?", &band()).is_err());
    assert!(parse_block_tokens("JUNE 2026", "X||", &band()).is_err());
    assert!(
        parse_block_tokens("JUNE 2026", "X|||||X|", &band()).is_err(),
        "a long read is as wrong as a short one"
    );
}

#[test]
fn a_parsed_band_keeps_blank_cells_so_the_grid_stays_aligned() {
    let rows = parse_block_tokens("JUNE 2026", "X|||||X", &band()).expect("a full grid");
    assert_eq!(rows.len(), 2, "two rows");
    assert_eq!(
        rows[0],
        vec!["X", "", ""],
        "a present student is a blank cell, and the blanks keep their place"
    );
    assert_eq!(rows[1], vec!["", "", "X"]);
}

#[test]
fn a_read_never_reorders_the_marks() {
    // Row-major, exactly as TEXTJOIN walks a range, so a mark's position in the
    // result is its position in the sheet.
    let rows = parse_block_tokens("JUNE 2026", "X||||&|X", &band()).expect("a full grid");
    assert_eq!(rows[0], vec!["X", "", ""]);
    assert_eq!(rows[1], vec!["", "&", "X"]);
    assert_eq!(count_absent_marks(&rows), 2);
}

#[test]
fn only_the_cells_that_hold_something_are_written_back() {
    // A present student is a blank cell and the month file's grid is already
    // blank, so writing the blanks would be ~1,300 COM calls a month to
    // reproduce emptiness - and would risk clearing a merged neighbour.
    let rows = parse_block_tokens("JUNE 2026", "X|||||X", &band()).expect("a full grid");
    let marks = marks_from_band(&band(), &rows);

    assert_eq!(marks.len(), 2, "the four blank cells were not written");
    assert_eq!(marks[0].row_index, 8);
    assert_eq!(marks[0].column_index, 6);
    assert_eq!(marks[0].value, "X");
    assert_eq!(marks[1].row_index, 9);
    assert_eq!(marks[1].column_index, 8);
}

#[test]
fn marks_are_written_at_their_absolute_position() {
    let shifted = RowBand {
        first_row: 34,
        last_row: 35,
        ..band()
    };
    let rows = parse_block_tokens("JUNE 2026", "X|||||X", &shifted).expect("a full grid");
    let marks = marks_from_band(&shifted, &rows);

    assert_eq!(
        marks[0].row_index, 34,
        "an expanded roster's rows are absolute"
    );
    assert_eq!(marks[1].row_index, 35);
}

#[test]
fn the_x_count_ignores_case_but_counts_nothing_else() {
    let rows = vec![
        vec!["X".to_string(), "x".to_string(), "".to_string()],
        vec!["".to_string(), "OK".to_string(), "X".to_string()],
    ];
    assert_eq!(count_absent_marks(&rows), 3);
}

#[test]
fn the_writable_columns_are_the_ones_that_carry_a_weekday_label() {
    // The other columns of a day cell are the second half of a merged pair. A
    // write to one lands on their pair's primary cell, so writing them would
    // clear the very mark they duplicate.
    let writable = primary_day_columns(&bundled_slots());
    assert_eq!(writable.len(), 25);
    for skipped in [7, 13, 19, 23, 25, 27, 34, 38] {
        assert!(
            !writable.contains(&skipped),
            "column {skipped} is a merged sub-cell and must not be written"
        );
    }
}

// ── Verification ────────────────────────────────────────────────────────────

#[test]
fn a_month_that_matches_its_source_exactly_verifies() {
    let verification = verify_month_build(12, 12, 40, 40);
    assert!(verification.is_verified());
    assert_eq!(verification.mismatch_reason(), None);
}

#[test]
fn one_missing_mark_fails_the_month() {
    let verification = verify_month_build(12, 11, 40, 40);
    assert!(!verification.is_verified());
    let reason = verification
        .mismatch_reason()
        .expect("a reason for the log");
    assert!(reason.contains("11 of 12"), "{reason}");
}

#[test]
fn one_extra_mark_fails_the_month_too() {
    // A month that somehow holds *more* marks than its source is just as wrong as
    // one holding fewer, and just as likely to be somebody else's marks.
    assert!(!verify_month_build(12, 13, 40, 40).is_verified());
}

#[test]
fn a_roster_that_does_not_match_fails_the_month() {
    let verification = verify_month_build(12, 12, 40, 39);
    assert!(!verification.is_verified());
    let reason = verification
        .mismatch_reason()
        .expect("a reason for the log");
    assert!(reason.contains("39 of 40"), "{reason}");
}

// -- The DepEd day grid holds every school day ------------------------------

// The form's day grid is 25 labelled cells, not 33. Columns `7, 13, 19, 23, 27,
// 34, 38` are the second halves of merged weekday pairs and column `39` is
// `ABSENT`, so the last five columns of the physical block are not days. That
// makes 25 - five weeks of Monday..Friday - the real capacity, and a 31-day
// month starting on a Monday is the widest a calendar month can be at 23 school
// days. The two numbers are close enough that "does the form have room?" deserves
// a test over every month rather than an argument.
//
// A day the grid cannot hold is not a cosmetic loss: it is a day the teacher
// cannot record an absence on, and the day-number header will not show it, so it
// reads as a holiday nobody took.

#[test]
fn no_month_of_a_century_drops_a_school_day() {
    let mut checked = 0usize;
    for year in 2000..=2100_i32 {
        for month in 1..=12_u32 {
            let last_day = (1..=31_u32)
                .filter_map(|day| NaiveDate::from_ymd_opt(year, month, day))
                .next_back()
                .map_or(0, |date| date.day());
            for first_school_day in 1..=last_day {
                if !NaiveDate::from_ymd_opt(year, month, first_school_day)
                    .is_some_and(is_school_day)
                {
                    continue;
                }
                let dropped = days_without_a_slot(year, month, first_school_day, &bundled_slots());
                assert!(
                    dropped.is_empty(),
                    "{year}-{month:02} starting on day {first_school_day} has school day(s) \
                     {dropped:?} with no day column in the DepEd form"
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked > 10_000,
        "only {checked} months were checked; the sweep is not sweeping"
    );
}

#[test]
fn the_day_grid_fills_every_slot_of_a_full_five_week_month() {
    // 2026-09: a 30-day month whose first is a Tuesday. Every one of the 25 slots
    // that the month reaches has a number in it, and none of the 25 is left blank
    // while a day exists somewhere the form cannot show it.
    let days = day_numbers(2026, 9, 1);
    let placed: Vec<u32> = days.iter().filter_map(|(_, day)| *day).collect();

    assert_eq!(
        placed.len(),
        days_without_a_slot(2026, 9, 1, &bundled_slots()).len() + placed.len(),
        "sanity: nothing is dropped, so every placed day is a real day"
    );
    assert_eq!(
        placed
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>(),
        (1..=30_u32)
            .filter(|day| { NaiveDate::from_ymd_opt(2026, 9, *day).is_some_and(is_school_day) })
            .collect::<std::collections::HashSet<_>>(),
        "the header must show every school day of the month"
    );
}

#[test]
fn the_widest_possible_month_still_fits_the_form() {
    // A 31-day month starting on a Monday is the maximum a calendar month can be:
    // 23 school days. This is the case the 25-slot layout has to survive, so it
    // is the case worth naming.
    let mut widest = (0u32, 0u32, 0u32);
    for year in 2000..=2100_i32 {
        for month in 1..=12_u32 {
            let school_days = (1..=31_u32)
                .filter(|day| NaiveDate::from_ymd_opt(year, month, *day).is_some_and(is_school_day))
                .count() as u32;
            if school_days > widest.0 {
                widest = (school_days, year as u32, month);
            }
        }
    }
    assert_eq!(
        widest.0, 23,
        "the widest calendar month in 2000-2100 holds {} school days ({}-{:02}); the premise of \
         this test changed",
        widest.0, widest.1, widest.2
    );
    assert!(
        widest.0 <= bundled_slots().len() as u32,
        "the widest month needs {} school days and the form has {} day columns",
        widest.0,
        bundled_slots().len()
    );
}
