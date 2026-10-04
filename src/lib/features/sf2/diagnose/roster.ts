/**
 * Turning what the database holds into the rows a month's comparison runs on.
 *
 * Three questions live here, and all three are about *attribution* - deciding which of
 * the things the database holds belong to the workbook being diagnosed:
 *
 * - {@link rosterFor}: which roster rows describe this month's learner rows, and which
 *   table they came from;
 * - {@link classAbsences}: which absences belong to this workbook's class;
 * - {@link matchRosterByName}: which learner a worksheet row is, when the worksheet is
 *   on some *other* file and a row index therefore means nothing.
 *
 * Getting any of them wrong produces a month full of differences that are only
 * differences of layout, which is why each is a named rule rather than a filter
 * somebody can move.
 */

import { normalizeLearnerName } from '$lib/features/sf2/logic';
import type { DbSnapshot } from './db-read';
import type { AbsentRecord, MappingSource, RosterResolution, RosterRow } from './model';
import type { RawSheet } from './probe';

/**
 * The roster rows the comparison should use for `templateId`.
 *
 * Per-month first, legacy second. The reverse order would be wrong: the legacy roster
 * is the one the *current* file was imported with, and the per-month roster only exists
 * once a split has run for that month.
 */
export function rosterFor(
	snapshot: DbSnapshot,
	templateId: string
): { rows: RosterRow[]; source: MappingSource } {
	const perMonth = snapshot.monthRosters.get(templateId);
	if (perMonth !== undefined && perMonth.length > 0) {
		return { rows: perMonth, source: 'perMonthTables' };
	}
	const legacy = snapshot.legacyRosters.get(templateId);
	if (legacy !== undefined && legacy.length > 0) {
		return { rows: legacy, source: 'legacyTables' };
	}
	return { rows: [], source: 'none' };
}

/**
 * The roster rows a month is compared with, and where they came from.
 *
 * A worksheet on the referenced file uses the database's own row mappings: they were
 * derived from that exact sheet, so the row indices mean the same thing. A worksheet on
 * *any other* file does not - a row index means nothing across workbooks - so its roster
 * is resolved by matching the worksheet's own `NAME` column against the database's
 * students.
 */
export function rosterForSheet(
	sheet: RawSheet,
	snapshot: DbSnapshot,
	templateId: string,
	isReferenced: boolean
): { roster: RosterRow[]; mappingSource: MappingSource; rosterResolution: RosterResolution } {
	if (isReferenced) {
		const { rows, source } = rosterFor(snapshot, templateId);
		return {
			roster: rows,
			mappingSource: source,
			rosterResolution: rows.length === 0 ? 'unresolved' : 'databaseRowMappings'
		};
	}
	const matched = matchRosterByName(sheet.rosterNames, snapshot);
	return {
		roster: matched,
		mappingSource: 'none',
		rosterResolution: matched.length === 0 ? 'unresolved' : 'workbookNameMatch'
	};
}

/**
 * The absences that belong to this workbook's class.
 *
 * Same test as the app's `event_belongs_to_class`: the event names the class, or the
 * student is in it. An absence that fails both is counted in `absentEventsWithoutClass`
 * rather than dropped, so it is never quietly folded into a month's number.
 */
export function classAbsences(snapshot: DbSnapshot, classId: string): AbsentRecord[] {
	const classStudents = new Set(
		snapshot.students.filter((student) => student.classId === classId).map((student) => student.id)
	);
	return snapshot.absentEvents.filter(
		(record) => record.classId === classId || classStudents.has(record.studentId)
	);
}

/** Match a worksheet's own learner names against the database's students. */
export function matchRosterByName(
	sheetNames: readonly RosterRow[],
	snapshot: DbSnapshot
): RosterRow[] {
	const byName = new Map<string, string>();
	for (const student of snapshot.students) {
		// First row for a name wins, matching the `students` table's `ORDER BY name`.
		const key = normalizeLearnerName(student.name);
		if (!byName.has(key)) byName.set(key, student.id);
	}
	const matched: RosterRow[] = [];
	for (const row of sheetNames) {
		const studentId = byName.get(normalizeLearnerName(row.workbookName));
		if (studentId === undefined) continue;
		matched.push({ studentId, workbookName: row.workbookName, rowIndex: row.rowIndex });
	}
	return matched;
}
