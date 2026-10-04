import { getDriver } from '$lib/db';
import { normalizeLearnerName } from '$lib/features/sf2/logic';
import { EMPTY_ROSTER_ANALYSIS_MESSAGE } from './templates';
import { appError } from '$lib/db';

/**
 * `sf2_month_student_mappings` — the port of
 * `src-tauri/src/sf2/month/student_repo.rs`.
 *
 * Identity, in the order the spec sets out:
 *
 * 1. `sf2LearnerId` — the school's own record. Survives a rename and a roster
 *    reshuffle between two months.
 * 2. `normalizedName` — survives a row move, not a rename.
 * 3. `rowIndex` — position in one file. Says nothing about the learner, and is
 *    exactly what silently re-points a student's X marks at somebody else when
 *    the roster was re-sorted between months.
 *
 * `matchRosterLearner()` implements that order.
 */

/** One learner row of one month file. */
export interface Sf2MonthStudentMapping {
	templateId: string;
	studentId: string;
	workbookName: string;
	normalizedName: string;
	rowIndex: number;
	genderBlock?: string;
	/**
	 * The DepEd learner ID read from the workbook, when the workbook had one to
	 * give. `undefined` for a workbook whose learner-ID cell is the merged "No."
	 * cell — see {@link depedLearnerIdFromCells}.
	 */
	sf2LearnerId?: string;
}

/** The workbook half of a learner, as a read path needs it to be identified. */
export interface Sf2WorkbookLearnerIdentity {
	name: string;
	rowIndex: number;
	sf2LearnerId?: string;
}

/** Which rule identified a learner. */
export type Sf2LearnerMatchKind = 'learnerId' | 'normalizedName' | 'rowIndex';

/** One identified learner, and which rule found them. */
export interface Sf2LearnerMatch {
	templateId: string;
	studentId: string;
	rowIndex: number;
	matchedBy: Sf2LearnerMatchKind;
}

/**
 * The longest value accepted as a DepEd learner ID. A real LRN is a short digit
 * string; anything longer is a name or a header that landed in the column, and
 * storing it would create an identity that can never match.
 */
const MAX_LEARNER_ID_LEN = 32;

const ROSTER_COLUMNS = `template_id, student_id, workbook_name, normalized_name,
    row_index, gender_block, sf2_learner_id`;

/**
 * Rust's `sf2::logic::normalize_learner_name`, imported rather than restated: a
 * second definition of "the same name" would split every roster match in two.
 */
export { normalizeLearnerName };

/**
 * Could `candidate` be a DepEd learner ID rather than a name or a stray value?
 *
 * A DepEd ID is a single token made of letters, digits and the odd separator
 * (`13672845021`, `LRN-0001`, `LRNS-2024-0012345`), and it always carries at
 * least one digit. A learner's name does not: it carries spaces, and punctuation
 * like the comma in `DELA CRUZ, JUAN`. Requiring a digit as well rejects the
 * all-letters cases a separator check alone would let through.
 */
function looksLikeALearnerId(candidate: string): boolean {
	return /[0-9]/.test(candidate) && /^[A-Za-z0-9\-_/.]+$/.test(candidate);
}

/**
 * The DepEd learner ID in a workbook cell, or `undefined` when the cell does not
 * actually hold one.
 *
 * **Read from column 2** — the learner-ID slot of the DepEd SF2 form. The bundled
 * `TEMPLATE_AUTOMATED_SF2.xlsx` merges that cell into the "No." cell
 * (`A8:B8` holds the item number, `C8:E8` holds the name), so reading column 2 on
 * that template returns the item number. When the two cells read identically the
 * value is the item number, not an ID, and it is rejected: storing "1", "2",
 * "3"... as learner IDs would give every month file the same positional identity,
 * which is the exact fragility this column exists to remove.
 *
 * A workbook that has a real DepEd ID in an unmerged column B reads it here and
 * keeps it; a learner with no ID falls through to the name, then to the row.
 *
 * A mis-aligned column is rejected too. A name that landed in the ID slot is worse
 * than no ID at all: it is a plausible string, so it would be stored and then
 * matched as if it were the school's own record — outranking the name match and
 * re-pointing a learner's marks.
 */
export function depedLearnerIdFromCells(
	learnerIdCell: string,
	itemNumberCell: string
): string | undefined {
	const candidate = learnerIdCell.trim();
	if (candidate === '' || candidate.length > MAX_LEARNER_ID_LEN) return undefined;
	if (candidate === itemNumberCell.trim()) return undefined;
	if (!looksLikeALearnerId(candidate)) return undefined;
	return candidate;
}

/**
 * Identify a workbook learner against the mappings already on record.
 *
 * `existing` is any month's mapping list — in practice the previous month's,
 * which is the whole point: the learner is being re-arranged into a new file. The
 * first rule that matches wins, so a DepEd ID outranks a name that a different
 * learner also answers to.
 */
export function matchRosterLearner(
	existing: Sf2MonthStudentMapping[],
	learner: Sf2WorkbookLearnerIdentity
): Sf2LearnerMatch | undefined {
	const learnerId = learner.sf2LearnerId?.trim();
	if (learnerId) {
		const byId = existing.find((mapping) => mapping.sf2LearnerId === learnerId);
		if (byId !== undefined) return matchFrom(byId, 'learnerId');
	}

	const normalizedName = normalizeLearnerName(learner.name);
	const byName = existing.find((mapping) => mapping.normalizedName === normalizedName);
	if (byName !== undefined) return matchFrom(byName, 'normalizedName');

	const byRow = existing.find((mapping) => mapping.rowIndex === learner.rowIndex);
	return byRow === undefined ? undefined : matchFrom(byRow, 'rowIndex');
}

function matchFrom(
	mapping: Sf2MonthStudentMapping,
	matchedBy: Sf2LearnerMatchKind
): Sf2LearnerMatch {
	return {
		templateId: mapping.templateId,
		studentId: mapping.studentId,
		rowIndex: mapping.rowIndex,
		matchedBy
	};
}

interface RosterRow {
	template_id: string;
	student_id: string;
	workbook_name: string;
	normalized_name: string;
	row_index: number;
	gender_block: string | null;
	sf2_learner_id: string | null;
}

function toMapping(row: RosterRow): Sf2MonthStudentMapping {
	return {
		templateId: row.template_id,
		studentId: row.student_id,
		workbookName: row.workbook_name,
		normalizedName: row.normalized_name,
		rowIndex: row.row_index,
		genderBlock: row.gender_block ?? undefined,
		sf2LearnerId: row.sf2_learner_id ?? undefined
	};
}

/**
 * Replace one month file's roster with `students`, in a single transaction.
 *
 * An empty roster is rejected rather than committed: committing it would unmap
 * every learner in the month file, and a file whose marks are then written through
 * an empty mapping is how marks end up on the wrong row.
 */
export async function replaceMonthRoster(
	templateId: string,
	students: Sf2MonthStudentMapping[]
): Promise<void> {
	if (students.length === 0) throw appError('InvalidInput', EMPTY_ROSTER_ANALYSIS_MESSAGE);

	const driver = getDriver();
	await driver.transaction(async () => {
		await driver.execute('DELETE FROM sf2_month_student_mappings WHERE template_id = ?', [
			templateId
		]);
		for (const student of students) {
			await driver.execute(
				`INSERT OR REPLACE INTO sf2_month_student_mappings (${ROSTER_COLUMNS})
				 VALUES (?, ?, ?, ?, ?, ?, ?)`,
				[
					student.templateId,
					student.studentId,
					student.workbookName,
					student.normalizedName,
					student.rowIndex,
					student.genderBlock ?? null,
					student.sf2LearnerId ?? null
				]
			);
		}
	});
}

/** One month file's roster, in workbook row order. */
export async function monthRosterForTemplate(
	templateId: string
): Promise<Sf2MonthStudentMapping[]> {
	const rows = await getDriver().query<RosterRow>(
		`SELECT ${ROSTER_COLUMNS} FROM sf2_month_student_mappings
		 WHERE template_id = ? ORDER BY row_index ASC`,
		[templateId]
	);
	return rows.map(toMapping);
}

/**
 * Every mapping that already knows this DepEd learner ID, across months.
 *
 * The query a split or a roster sync runs first, because it is the only one that
 * survives a learner moving to a different row.
 */
export async function monthMappingsForLearnerId(
	sf2LearnerId: string
): Promise<Sf2MonthStudentMapping[]> {
	const rows = await getDriver().query<RosterRow>(
		`SELECT ${ROSTER_COLUMNS} FROM sf2_month_student_mappings
		 WHERE sf2_learner_id = ? ORDER BY row_index ASC`,
		[sf2LearnerId]
	);
	return rows.map(toMapping);
}

/** The mapping for one normalized name in one month file. */
export async function monthMappingForNormalizedName(
	templateId: string,
	normalizedName: string
): Promise<Sf2MonthStudentMapping | undefined> {
	const row = await getDriver().queryOne<RosterRow>(
		`SELECT ${ROSTER_COLUMNS} FROM sf2_month_student_mappings
		 WHERE template_id = ? AND normalized_name = ?`,
		[templateId, normalizedName]
	);
	return row === undefined ? undefined : toMapping(row);
}

/**
 * Store DepEd learner IDs on students, the backfill the split performs.
 *
 * Returns how many students were written. A pair that was refused — because the
 * ID already belongs to another learner, or the student already carries a
 * different ID — is simply not counted, so the caller can report
 * `pairs.length - written` as the number of identities that need a human.
 */
export async function setStudentLearnerIds(pairs: [string, string][]): Promise<number> {
	if (pairs.length === 0) return 0;

	const driver = getDriver();
	return driver.transaction(async () => {
		let written = 0;
		for (const [studentId, sf2LearnerId] of pairs) {
			const learnerId = sf2LearnerId.trim();
			if (learnerId === '') continue;
			const updated = await driver.execute(
				`UPDATE students
				 SET sf2_learner_id = ?
				 WHERE id = ?
				   AND (sf2_learner_id IS NULL OR sf2_learner_id = '' OR sf2_learner_id = ?)
				   AND NOT EXISTS (
				       SELECT 1 FROM students AS other
				       WHERE other.id <> ? AND other.sf2_learner_id = ?
				   )`,
				[learnerId, studentId, learnerId, studentId, learnerId]
			);
			written += updated;
		}
		return written;
	});
}

/** Drop one month file's roster and report how many rows went. */
export async function deleteMonthRoster(templateId: string): Promise<number> {
	return getDriver().execute('DELETE FROM sf2_month_student_mappings WHERE template_id = ?', [
		templateId
	]);
}
