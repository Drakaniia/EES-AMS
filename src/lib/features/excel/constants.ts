/**
 * Fixed layout of the DepEd School Form 2 (SF2) workbook.
 *
 * These are the numbers the official form is built from; nothing in the workbook is
 * discoverable at runtime, so they live here rather than being re-derived per call
 * site.
 */

/**
 * The title in `A1` of an SF2 worksheet.
 *
 * Matched as a substring so a school's own variant - the same title with a
 * different suffix - is still recognised. This is what tells a form worksheet
 * from a school's own working sheet, and why `COMPLETE DAYS`, which *is* a full
 * copy of the form, counts as a form and not as a helper.
 */
export const SF2_FORM_TITLE = 'School Form 2 (SF2)';

/** The `No.` column of the DepEd SF2 form, which is the item number. */
export const SF2_ITEM_NUMBER_COLUMN = 1;

/**
 * The DepEd learner-ID column of the SF2 form, i.e. column `B`.
 *
 * Verified against the bundled `TEMPLATE_AUTOMATED_SF2.xlsx`: the learner row is
 * `A8:B8` (item number, header `A5:B7` = "No.") merged with `C8:E8` (name,
 * header `C5:E7` = "NAME"). Column 2 is therefore the DepEd ID's slot but the
 * bundled template does not use it - reading it returns the merged item number,
 * so it must be rejected as an identity rather than trusted.
 */
export const SF2_DEPED_LEARNER_ID_COLUMN = 2;

/** The `NAME (Last Name, First Name, Middle Name)` column, `C`. */
export const SF2_NAME_COLUMN = 3;

/** The row the day numbers are printed in, 6. */
export const SF2_DAY_ROW = 6;

/** The row the weekday header is printed in, 7. */
export const SF2_WEEKDAY_ROW = 7;

/** The first learner row of a fresh bundled template, 8. */
export const SF2_FIRST_LEARNER_ROW = 8;

/** Male learner slots a fresh bundled template ships with, 21. */
export const SF2_FRESH_MALE_SLOTS = 21;

/** Female learner slots a fresh bundled template ships with, 19. */
export const SF2_FRESH_FEMALE_SLOTS = 19;

/** The MALE TOTAL row of a fresh, unexpanded bundled template, 29. */
export const SF2_FRESH_MALE_TOTAL_ROW = 29;

/** The first FEMALE learner row of a fresh, unexpanded bundled template, 30. */
export const SF2_FRESH_FEMALE_START_ROW = 30;

/** The FEMALE TOTAL row of a fresh, unexpanded bundled template, 49. */
export const SF2_FRESH_FEMALE_TOTAL_ROW = 49;

/** The Combined TOTAL row, one below the FEMALE TOTAL, 50. */
export const SF2_FRESH_COMBINED_TOTAL_ROW = 50;

/** First day column of the SF2 attendance block - column `F`. */
export const SF2_ATTENDANCE_FIRST_COLUMN = 6;

/** Last day column of the SF2 attendance block - column `AL`. */
export const SF2_ATTENDANCE_LAST_COLUMN = 38;

/**
 * `TOTAL NO. OF DAYS`, merged `AW5:AY6`.
 *
 * `$AW$5` is what every `PRESENT` formula multiplies by, so it has to hold the
 * real mapped school-day count rather than the template's stale value.
 */
export const SF2_TOTAL_DAYS_CELL = 'AW5';

/**
 * The absolute reference to {@link SF2_TOTAL_DAYS_CELL}, as the SF2 formulas
 * spell it. Every `PRESENT` formula multiplies by it, so the value has to track
 * the real mapped school-day count.
 */
export const SF2_TOTAL_DAYS_REF = '$AW$5';

/** The three summary columns: Boys (`AR`), Girls (`AS`), Combined (`AT`). */
export const SF2_SUMMARY_COLUMNS = ['AR', 'AS', 'AT'] as const;

/** One of the three summary columns. */
type Sf2SummaryColumn = (typeof SF2_SUMMARY_COLUMNS)[number];

/** Rows 55, 67, 69 and 71 of one summary column, as the form labels them. */
type Sf2SummaryCounts = {
	/** Late enrolment. */
	lateEnrolment: number;
	/** Dropped out (NLS). */
	droppedOut: number;
	/** Transferred out. */
	transferredOut: number;
	/** Transferred in. */
	transferredIn: number;
};

/** Those counts for each summary column - they differ between boys and girls. */
export type Sf2SummaryCountsByColumn = Record<Sf2SummaryColumn, Sf2SummaryCounts>;

/** Summary rows the app writes, paired with what the form labels them. */
export const SF2_SUMMARY_ROWS = {
	enrolment: 53,
	lateEnrolment: 55,
	droppedOut: 67,
	transferredOut: 69,
	transferredIn: 71,
	registeredLearners: 59,
	percentageOfEnrolment: 61,
	averageDailyAttendance: 63,
	percentageOfAttendance: 65
} as const;

/**
 * The metadata block, as `{ row, column }` pairs.
 *
 * The form spells each field into a merged cell whose top-left corner is the
 * only cell that can be written: school id `F3:I3`, school year `M3:R3`, report
 * month `AA3:AG3`, school name `F4:R4`, grade level `AA4:AG4`, section
 * `AM4:AU4`, adviser signature `AN76:AU77`, adviser printed name `Z82:AK82`,
 * school head printed name `AN82:AT82`.
 */
export const SF2_METADATA_CELLS = {
	schoolId: { row: 3, column: 6 },
	schoolYear: { row: 3, column: 13 },
	reportMonth: { row: 3, column: 27 },
	schoolName: { row: 4, column: 6 },
	gradeLevel: { row: 4, column: 27 },
	section: { row: 4, column: 39 },
	adviserSignature: { row: 76, column: 40 },
	adviserPrintedName: { row: 82, column: 26 },
	schoolHeadPrintedName: { row: 82, column: 40 }
} as const;

/**
 * The MALE TOTAL, FEMALE TOTAL and Combined TOTAL rows of a bundled-template
 * workbook, derived from the slot layout instead of hardcoded results.
 *
 * | Section        | Rows                                    | Count             |
 * |----------------|-----------------------------------------|-------------------|
 * | Male slots     | 8 … (7 + maleCapacity)                 | `maleCapacity`    |
 * | MALE TOTAL     | 8 + maleCapacity                       | —                 |
 * | Female slots   | (30 + extraMale) …                     | `femaleCapacity`  |
 * | FEMALE TOTAL   | 30 + extraMale + femaleCapacity        | —                 |
 * | Combined TOTAL | FEMALE TOTAL + 1                       | —                 |
 */
export function bundledTemplateTotalRows(
	maleCount: number,
	femaleCount: number
): { maleTotalRow: number; femaleTotalRow: number; combinedTotalRow: number } {
	const maleCapacity = Math.max(maleCount, SF2_FRESH_MALE_SLOTS);
	const femaleCapacity = Math.max(femaleCount, SF2_FRESH_FEMALE_SLOTS);
	const extraMale = maleCapacity - SF2_FRESH_MALE_SLOTS;
	return {
		maleTotalRow: SF2_FIRST_LEARNER_ROW + maleCapacity,
		femaleTotalRow: SF2_FRESH_FEMALE_START_ROW + extraMale + femaleCapacity,
		combinedTotalRow: SF2_FRESH_FEMALE_START_ROW + extraMale + femaleCapacity + 1
	};
}

/**
 * Extra male and female rows needed to fit `maleCount` / `femaleCount` learners
 * beyond the template's 21 male / 19 female slots.
 */
export function rosterExpansionNeeded(
	maleCount: number,
	femaleCount: number
): { extraMale: number; extraFemale: number } {
	return {
		extraMale: Math.max(0, maleCount - SF2_FRESH_MALE_SLOTS),
		extraFemale: Math.max(0, femaleCount - SF2_FRESH_FEMALE_SLOTS)
	};
}
