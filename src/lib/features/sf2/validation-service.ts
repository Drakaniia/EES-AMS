/**
 * Import validation and the roster geometry an import writes with — the port of
 * `src-tauri/src/sf2/validation_service.rs`.
 *
 * The Rust file was the whole import: pick a file, read it, validate, then run ten
 * Excel operations in one COM session. The Excel half of that belongs to
 * `$lib/features/excel` and the roster sync to the roster module, so what is left
 * here is the two pieces that are neither — the database read that makes the
 * validation report possible, and the row arithmetic every writer needs.
 */

import { listClasses } from '$lib/db/repos/classes';
import { listStudents } from '$lib/db/repos/students';
import {
	SF2_FRESH_FEMALE_SLOTS,
	SF2_FRESH_MALE_SLOTS,
	bundledTemplateTotalRows,
	rosterExpansionNeeded
} from '$lib/features/excel/constants';
import type { Student } from '$lib/domain/models';
import { analysisClassName, validateStudentList } from './validation';
import type { Sf2ImportValidation } from './validation';
import type { Sf2WorkbookAnalysis } from './calendar';

/**
 * The validation report for a workbook, against the class it names.
 *
 * The class is found by name, case-insensitively, because the class name is
 * derived from the workbook's own grade-level and section cells and a workbook the
 * teacher typed by hand will not match the stored casing exactly. A workbook whose
 * class does not exist yet compares against no students at all, which is what makes
 * a first import open on an empty rather than a full-of-invented-mismatches report.
 */
export async function importValidationFromAnalysis(
	sourcePath: string,
	analysis: Sf2WorkbookAnalysis
): Promise<Sf2ImportValidation> {
	const name = analysisClassName(analysis);
	const classes = await listClasses();
	const existing = classes.find((candidate) => candidate.name.toLowerCase() === name.toLowerCase());
	const currentStudents: Student[] = existing ? await listStudents(existing.id) : [];

	return validateStudentList(sourcePath, existing?.id, name, currentStudents, analysis.learners);
}

/**
 * Every row number an import writes, derived from the class's roster.
 *
 * All of it comes from two counts, and all of it has to agree: the TOTAL rows the
 * formulas sum, the rows to hide because no learner sits in them, and the rows the
 * roster claims. A workbook whose TOTAL row and roster disagree prints a subtotal
 * of nothing, which is why this is computed once, here, instead of at each call
 * site.
 */
export type Sf2RosterGeometry = {
	maleCount: number;
	femaleCount: number;
	/** Rows the template has to grow by, per gender block. */
	extraMale: number;
	extraFemale: number;
	maleTotalRow: number;
	femaleTotalRow: number;
	combinedTotalRow: number;
	/** Row 29 shifted down by the male growth: the last slot to hide. */
	hideThroughMaleRow: number;
	/** Row 49 shifted down by both growths. */
	hideThroughFemaleRow: number;
	/** The learner rows the roster claims. */
	occupiedRows: Set<number>;
};

const HIDE_FIRST_SLOT_ROW = 29;
const HIDE_FIRST_FEMALE_SLOT_ROW = 49;

/**
 * The row geometry for a class's roster.
 *
 * `rows` are the row indexes the roster sync assigned, one per learner; the hide
 * arithmetic is derived from the counts alone so it can be computed before the
 * roster exists, which is the order the import works in.
 */
export function rosterGeometry(
	students: readonly Student[],
	rows: readonly number[]
): Sf2RosterGeometry {
	const maleCount = students.filter((student) => student.gender === 'male').length;
	const femaleCount = students.filter((student) => student.gender === 'female').length;
	const { extraMale, extraFemale } = rosterExpansionNeeded(maleCount, femaleCount);
	const extraMaleHide = Math.max(0, maleCount - SF2_FRESH_MALE_SLOTS);
	const extraFemaleHide = Math.max(0, femaleCount - SF2_FRESH_FEMALE_SLOTS);

	return {
		maleCount,
		femaleCount,
		extraMale,
		extraFemale,
		...bundledTemplateTotalRows(maleCount, femaleCount),
		hideThroughMaleRow: HIDE_FIRST_SLOT_ROW + extraMaleHide,
		hideThroughFemaleRow: HIDE_FIRST_FEMALE_SLOT_ROW + extraMaleHide + extraFemaleHide,
		occupiedRows: new Set(rows)
	};
}
