/**
 * `sf2::preview::export_preview` — the single implementation of what an `X` in the
 * SF2 reports grid means.
 *
 * ## Why this is a builder, not a query
 *
 * Two reads produce a grid: `getSf2MonthPreview` (the per-month model, the live
 * one) and the pre-split fallback that reads `sf2_templates` on an install that has
 * not been migrated. If each computed cell status itself, "an X means the same
 * thing whichever read produced the grid" would be a promise rather than a
 * property. So the month service resolves the mappings and the events and hands them
 * here, and {@link buildExportPreview} decides every cell.
 *
 * That is why this module takes no database and no clock: everything it answers from
 * is passed in. It is also what makes {@link registerSf2Preview} a two-line
 * registration rather than a dependency to be satisfied.
 *
 * ## Determinism
 *
 * Rust's `HashMap`/`HashSet` iteration order was arbitrary, so the order students
 * and warnings appeared in was whatever the hash seed produced. Every set here is
 * built in iteration order over `roster` / `classStudents`, and every warning is
 * appended at the point it is discovered — so two runs over the same data produce
 * the same grid in the same order, which is what makes it assertable.
 */

import {
	absentStudentIds,
	presentStudentIds
} from '$lib/features/sf2/attendance/attendance-events';
import type { MonthGridInput } from '$lib/features/sf2/month/month';
import { useMonthGridBuilder } from '$lib/features/sf2/month/month';
import type { Sf2ExportPreview, Sf2PreviewCellStatus, Sf2PreviewStudentRow } from '$lib/types';

/** The roster gender blocks a grid row can fall back to, and their labels. */
const GENDER_LABELS: Record<string, string> = { MALE: 'Male', FEMALE: 'Female' };

/**
 * What a cell in the SF2 preview means.
 *
 * - `absent` — the learner has an explicit absent record → an `X`.
 * - `present` — the day has attendance taken and the learner has no absent record
 *   → an empty cell, because the form is present-by-default.
 * - `open` — the day has no records at all → attendance was never taken, so the
 *   cell is editable and nothing is claimed about it.
 */
export function previewCellStatus(
	isAbsent: boolean,
	dayHasAttendance: boolean
): Sf2PreviewCellStatus {
	if (isAbsent) return 'absent';
	return dayHasAttendance ? 'present' : 'open';
}

/**
 * The label a grid row shows for a learner.
 *
 * The database's own gender wins; the roster's gender block is the fallback for a
 * learner the class record says nothing about. An unrecognised block is passed
 * through as typed rather than guessed at, which is what
 * `Sf2PreviewStudentRow.gender` being optional is for.
 */
export function previewGender(
	gender: string | undefined,
	genderBlock: string | undefined
): string | undefined {
	const stated = gender?.trim();
	if (stated !== undefined && stated !== '') return stated;
	const block = genderBlock?.trim();
	if (block === undefined || block === '') return undefined;
	return GENDER_LABELS[block.toUpperCase()] ?? block;
}

/**
 * Build the export preview from pre-resolved data. No database, no clock, no IO.
 *
 * `readiness` supplies the counts and the problems the caller already worked out;
 * this function only decides what each cell shows and what the grid counts. That
 * split is what lets `month.ts` keep owning "is this month ready" and this module
 * keep owning "what does an X mean".
 */
export function buildExportPreview(input: MonthGridInput): Sf2ExportPreview {
	const { template, roster, dates, className, classStudents, events, readiness } = input;
	const classId = template.classId;
	const studentsById = new Map(classStudents.map((student) => [student.id, student]));

	// A cell is `X` only because of an explicit absent record. Everyone else stays
	// blank, so marking one learner absent never touches another learner's cell.
	const absentByDay = new Map(
		dates.map((date) => [date.date, absentStudentIds(events, classStudents, classId, date.date)])
	);

	// A day counts as "attendance taken" when *any* record exists for it — an `in` or
	// an explicit `absent`. A day with none stays `open`, so every cell is editable
	// and nothing is claimed.
	const takenByDay = new Map(
		dates.map((date) => [
			date.date,
			presentStudentIds(events, classStudents, classId, date.date).size > 0 ||
				(absentByDay.get(date.date)?.size ?? 0) > 0
		])
	);

	const warnings = [...readiness.warnings];
	const students: Sf2PreviewStudentRow[] = [];
	const absentList: Sf2ExportPreview['absentList'] = [];
	const mappedStudentIds = new Set<string>();
	let presentCount = 0;
	let absenceCount = 0;

	for (const mapping of roster) {
		mappedStudentIds.add(mapping.studentId);
		const student = studentsById.get(mapping.studentId);
		// A mapped row whose student has left the class keeps the workbook's name: it is
		// the only thing that still identifies the row on the form.
		const stored = student?.name.trim();
		const studentName = stored === undefined || stored === '' ? mapping.workbookName : stored;

		const rowWarnings: string[] = [];
		if (student === undefined) {
			rowWarnings.push('This SF2 row points to a student record that is no longer in the class.');
			warnings.push(
				`${mapping.workbookName} is mapped in the SF2 workbook but is not in the selected class.`
			);
		}

		let rowPresent = 0;
		let rowAbsent = 0;
		const cells = dates.map((date) => {
			const status = previewCellStatus(
				absentByDay.get(date.date)?.has(mapping.studentId) ?? false,
				takenByDay.get(date.date) ?? false
			);
			if (status === 'present') {
				rowPresent += 1;
				presentCount += 1;
			} else if (status === 'absent') {
				rowAbsent += 1;
				absenceCount += 1;
				absentList.push({
					studentId: mapping.studentId,
					studentName,
					date: date.date,
					rowIndex: mapping.rowIndex
				});
			}
			return { date: date.date, status, editable: student !== undefined };
		});

		students.push({
			studentId: mapping.studentId,
			studentName,
			workbookName: mapping.workbookName,
			gender: previewGender(student?.gender, mapping.genderBlock),
			rowIndex: mapping.rowIndex,
			mapped: true,
			presentCount: rowPresent,
			absentCount: rowAbsent,
			warnings: rowWarnings,
			cells
		});
	}

	let unmappedStudentCount = 0;
	for (const student of classStudents) {
		if (mappedStudentIds.has(student.id)) continue;
		unmappedStudentCount += 1;
		warnings.push(
			`${student.name} is in the class roster but is not mapped to an SF2 learner row.`
		);
		students.push({
			studentId: student.id,
			studentName: student.name,
			// No workbook row, so there is nothing to put on the form.
			workbookName: '',
			gender: previewGender(student.gender, undefined),
			// Row 0 is the "not linked to a workbook row" placeholder. Every write path
			// filters it out, which is why it can never be written to.
			rowIndex: 0,
			mapped: false,
			presentCount: 0,
			absentCount: 0,
			warnings: ['Not linked to an SF2 workbook row. Update the workbook roster before export.'],
			// Present on every day and not editable: an unmapped row cannot show an X at
			// all, so claiming otherwise would be a grid that lies.
			cells: dates.map((date) => ({
				date: date.date,
				status: 'present' as const,
				editable: false
			}))
		});
	}

	if (roster.length === 0) {
		warnings.push('No learners are mapped to this SF2 workbook.');
	}

	return {
		template,
		classId,
		className,
		sourcePath: template.sourcePath,
		dates: [...dates],
		students,
		absentList,
		mappedStudents: readiness.mappedStudents,
		mappedDates: readiness.mappedDates,
		presentCount,
		absenceCount,
		unmappedStudentCount,
		canExport: readiness.issues.length === 0,
		issues: [...readiness.issues],
		warnings
	};
}

/**
 * Register this builder with the month service.
 *
 * Call once at app startup, alongside `useDriver()` and `useFileSystem()`:
 *
 * ```ts
 * import { registerSf2Preview } from '$lib/features/sf2/preview';
 * registerSf2Preview();
 * ```
 *
 * Until it is called, `getSf2MonthPreview` throws — deliberately, so a missing
 * registration is a startup error rather than a grid that renders empty and reads as
 * "nobody was ever marked absent".
 */
export function registerSf2Preview(): void {
	useMonthGridBuilder(buildExportPreview);
}

export type { MonthGridInput, Sf2ExportPreview };
