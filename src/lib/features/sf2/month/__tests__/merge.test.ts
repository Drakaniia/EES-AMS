/**
 * The merge: the decision rules, the roster arithmetic, the read-only comparison,
 * and the finished workbook a real bundled template turns into.
 *
 * Nothing here needs Excel. The workbook tests open the real converted DepEd
 * template off disk, write it to a `MemoryFileSystem`, and reopen the result - so
 * what is asserted is what a teacher would open.
 */

import type { Workbook, Worksheet } from 'exceljs';
import { beforeEach, describe, expect, it } from 'vitest';
import {
	getCellText,
	formulaOf,
	formulaResult,
	writableDayColumns
} from '$lib/features/excel/workbook';
import { bundledTemplateTotalRows } from '$lib/features/excel/constants';
import { SF2_ABSENT_MARK } from '$lib/features/sf2/logic';
import { useSf2WorkbookDir } from '$lib/features/sf2/workbook-files';
import type { Sf2MonthStudentMapping } from '$lib/features/sf2/month/students';
import type { StudentRecord } from '$lib/domain/models';
import type { Sf2SplitMonthOutcome, Sf2SplitOutcome } from '$lib/types';
import {
	TEMPLATE_ROSTER,
	loadTemplate,
	type TemplateFixture
} from '../../../excel/__tests__/template-fixture';
import {
	SPLIT_MONTH_COUNT,
	femaleBlockStart,
	isAccountedFor,
	isMergeComplete,
	loadAbsences,
	mergeSummaryMessage,
	needsAttentionLabels,
	resolveRoster,
	unprovenMonths,
	verifiedCount,
	type MergeJob
} from '../merge';
import { writeSummaryBlock } from '../summary-block';
import { db, insertMonthTemplate, useMonthTestDb } from './schema';

useMonthTestDb();

/** createStudent writes an audit row, which the month fixture does not declare. */
const AUDIT_EVENTS = `
CREATE TABLE audit_events (
	id TEXT PRIMARY KEY NOT NULL, entity_type TEXT NOT NULL, entity_id TEXT,
	action TEXT NOT NULL, summary TEXT NOT NULL, before_json TEXT,
	after_json TEXT, metadata_json TEXT, created_at INTEGER NOT NULL,
	actor TEXT NOT NULL DEFAULT 'admin'
);`;

const CLASS_ID = 'class-3-matapat';
const LEGACY_ID = 'template-legacy';
const SCHOOL_YEAR = '2026-2027';

const COUNTS = {
	AR: { lateEnrolment: 1, droppedOut: 0, transferredOut: 0, transferredIn: 1 },
	AS: { lateEnrolment: 0, droppedOut: 1, transferredOut: 1, transferredIn: 0 },
	AT: { lateEnrolment: 1, droppedOut: 1, transferredOut: 1, transferredIn: 1 }
};

function month(
	name: string,
	year: number,
	status: Sf2SplitMonthOutcome['status']
): Sf2SplitMonthOutcome {
	return {
		reportMonth: name,
		reportYear: year,
		sheetName: `${name} ${year}`,
		fileName: 'SF2-GRADE-3-MATAPAT-3b635890.xlsx',
		status,
		xMarks: 0,
		learnerRows: 27,
		detail: null
	};
}

/** The twelve months of `2026-2027`, SEPTEMBER -> AUGUST, as the school year runs them. */
const SCHOOL_YEAR_MONTHS: { month: string; reportYear: number }[] = [
	{ month: 'SEPTEMBER', reportYear: 2026 },
	{ month: 'OCTOBER', reportYear: 2026 },
	{ month: 'NOVEMBER', reportYear: 2026 },
	{ month: 'DECEMBER', reportYear: 2026 },
	{ month: 'JANUARY', reportYear: 2027 },
	{ month: 'FEBRUARY', reportYear: 2027 },
	{ month: 'MARCH', reportYear: 2027 },
	{ month: 'APRIL', reportYear: 2027 },
	{ month: 'MAY', reportYear: 2027 },
	{ month: 'JUNE', reportYear: 2027 },
	{ month: 'JULY', reportYear: 2027 },
	{ month: 'AUGUST', reportYear: 2027 }
];

function allVerified(): Sf2SplitMonthOutcome[] {
	return SCHOOL_YEAR_MONTHS.map(({ month: name, reportYear }) =>
		month(name, reportYear, 'verified')
	);
}

// ── Completeness ─────────────────────────────────────────────────────────────

describe('completeness', () => {
	it('is complete only when all twelve months verify', () => {
		const months = allVerified();
		expect(months).toHaveLength(SPLIT_MONTH_COUNT);
		expect(isMergeComplete(months)).toBe(true);
	});

	it('leaves one unproven month incomplete', () => {
		const months = allVerified();
		months[3].status = 'needsAttention';
		expect(isMergeComplete(months)).toBe(false);
	});

	it('counts a month an earlier run already merged as accounted for', () => {
		const months = allVerified();
		months[0].status = 'alreadyMerged';
		expect(isMergeComplete(months)).toBe(true);
	});

	it('is never complete on a short month list', () => {
		expect(isMergeComplete(allVerified().slice(0, 6))).toBe(false);
	});

	it('counts every month of a finished merge as verified', () => {
		expect(verifiedCount(allVerified())).toBe(SPLIT_MONTH_COUNT);
		expect(isAccountedFor(month('JUNE', 2027, 'alreadyMerged'))).toBe(true);
		expect(isAccountedFor(month('JUNE', 2027, 'needsAttention'))).toBe(false);
	});
});

// ── The sentence ────────────────────────────────────────────────────────────

describe('the summary sentence', () => {
	it('says one workbook with twelve visible worksheets', () => {
		const message = mergeSummaryMessage(1, allVerified(), 'SF2-1-A-1a2b3c4d.xlsx');
		expect(message).toContain('All 12 months are on one workbook');
		expect(message).toContain('its own visible worksheet');
	});

	it('always says where the original workbook is kept', () => {
		for (const completed of [1, null]) {
			const message = mergeSummaryMessage(completed, allVerified(), 'SF2-1-A-1a2b3c4d.xlsx');
			expect(message).toContain('_legacy');
			expect(message).toContain('SF2-1-A-1a2b3c4d.xlsx');
		}
	});

	it('names the months that need attention, in school-year order', () => {
		const months = allVerified();
		months[2].status = 'needsAttention';
		months[7].status = 'needsAttention';
		const message = mergeSummaryMessage(null, months, 'SF2-1-A-1a2b3c4d.xlsx');
		expect(message).toContain('2 needs attention');
		// The third and eighth months of 2026-2027 are NOVEMBER 2026 and APRIL 2027,
		// not the calendar-order months, which would name the wrong year.
		expect(message).toContain('NOVEMBER 2026');
		expect(message).toContain('APRIL 2027');
		expect(needsAttentionLabels(months)).toEqual(['NOVEMBER 2026', 'APRIL 2027']);
	});
});

// ── The roster layout ────────────────────────────────────────────────────────

describe('the female block', () => {
	it('starts at row 30 for any roster of 21 or fewer males', () => {
		expect(femaleBlockStart(0)).toBe(30);
		expect(femaleBlockStart(17)).toBe(30);
		expect(femaleBlockStart(21)).toBe(30);
	});

	it('grows only past 21 males', () => {
		expect(femaleBlockStart(25)).toBe(34);
		expect(femaleBlockStart(40)).toBe(49);
	});

	it('agrees with where the build puts the MALE TOTAL row', () => {
		for (const males of [0, 1, 17, 21, 25, 40]) {
			expect(femaleBlockStart(males)).toBe(bundledTemplateTotalRows(males, 10).maleTotalRow + 1);
		}
	});
});

function mapping(
	studentId: string,
	workbookName: string,
	rowIndex: number,
	genderBlock?: string,
	sf2LearnerId?: string
): Sf2MonthStudentMapping {
	return {
		templateId: LEGACY_ID,
		studentId,
		workbookName,
		normalizedName: workbookName.toUpperCase(),
		rowIndex,
		genderBlock,
		sf2LearnerId
	};
}

function student(id: string, name: string, gender: 'male' | 'female'): StudentRecord {
	return {
		id,
		name,
		gender,
		classId: CLASS_ID,
		createdAt: '2026-01-01T00:00:00.000Z'
	};
}

describe('resolving the roster', () => {
	beforeEach(async () => {
		await db().script(AUDIT_EVENTS);
		await db().script(`
			INSERT INTO students (id, name, class_id, created_at, gender)
			VALUES ('student-existing', 'DELA CRUZ, JUAN', '${CLASS_ID}', 1, 'male'),
			       ('student-missing', 'SANTOS, MARIA', '${CLASS_ID}', 1, 'female');
		`);
	});

	it('reuses a student the app already has rather than duplicating them', async () => {
		const roster = await resolveRoster(
			CLASS_ID,
			[
				mapping('student-existing', 'DELA CRUZ, JUAN', 8, 'MALE'),
				mapping('student-missing', 'SANTOS, MARIA', 30, 'FEMALE')
			],
			[
				student('student-existing', 'DELA CRUZ, JUAN', 'male'),
				student('student-missing', 'SANTOS, MARIA', 'female')
			]
		);
		expect(roster.writes.map((write) => write.studentId)).toEqual([
			'student-existing',
			'student-missing'
		]);
		expect(
			await db().queryOne<{ count: number }>('SELECT COUNT(*) AS count FROM students')
		).toMatchObject({ count: 2 });
	});

	it('creates a learner the workbook only knows', async () => {
		const roster = await resolveRoster(
			CLASS_ID,
			[mapping('student-from-workbook', 'REYES, ANA', 8, 'MALE')],
			[]
		);
		expect(roster.writes[0].studentId).not.toBe('student-from-workbook');
		const created = await db().queryOne<{ name: string; class_id: string }>(
			'SELECT name, class_id FROM students WHERE id = ?',
			[roster.writes[0].studentId]
		);
		expect(created).toMatchObject({ name: 'REYES, ANA', class_id: CLASS_ID });
	});

	it('lays males out from row 8 and females from just after the MALE TOTAL row', async () => {
		const roster = await resolveRoster(
			CLASS_ID,
			[
				mapping('s1', 'SANTOS, MARIA', 30, 'FEMALE'),
				mapping('s2', 'DELA CRUZ, JUAN', 8, 'MALE'),
				mapping('s3', 'REYES, ANA', 31, 'FEMALE')
			],
			[
				student('s2', 'DELA CRUZ, JUAN', 'male'),
				student('s1', 'SANTOS, MARIA', 'female'),
				student('s3', 'REYES, ANA', 'female')
			]
		);
		expect(roster.writes).toEqual([
			{ studentId: 's2', rowIndex: 8, name: 'DELA CRUZ, JUAN', itemNumber: 1, genderBlock: 'MALE' },
			{
				studentId: 's1',
				rowIndex: 30,
				name: 'SANTOS, MARIA',
				itemNumber: 1,
				genderBlock: 'FEMALE'
			},
			{ studentId: 's3', rowIndex: 31, name: 'REYES, ANA', itemNumber: 2, genderBlock: 'FEMALE' }
		]);
	});

	it('gives a student the app added after the split a row of its own', async () => {
		// The gap this closes: `reference` is the pre-split workbook's roster, so a
		// student typed into the Students page was in neither `reference` nor the
		// workbook, and every sheet left them out.
		const roster = await resolveRoster(
			CLASS_ID,
			[mapping('student-existing', 'DELA CRUZ, JUAN', 8, 'MALE')],
			[
				student('student-existing', 'DELA CRUZ, JUAN', 'male'),
				student('student-missing', 'SANTOS, MARIA', 'female')
			]
		);
		expect(roster.writes).toEqual([
			{
				studentId: 'student-existing',
				rowIndex: 8,
				name: 'DELA CRUZ, JUAN',
				itemNumber: 1,
				genderBlock: 'MALE'
			},
			{
				studentId: 'student-missing',
				rowIndex: 30,
				name: 'SANTOS, MARIA',
				itemNumber: 1,
				genderBlock: 'FEMALE'
			}
		]);
	});

	it('gives two learners with the same name distinct normalized names', async () => {
		const roster = await resolveRoster(
			CLASS_ID,
			[mapping('s1', 'SANTOS, MARIA', 8, 'MALE'), mapping('s2', 'SANTOS, MARIA', 9, 'MALE')],
			[student('s1', 'SANTOS, MARIA', 'male'), student('s2', 'SANTOS, MARIA', 'male')]
		);
		expect(roster.mappings.map((entry) => entry.normalizedName)).toEqual([
			'SANTOS,MARIA',
			'SANTOS,MARIA#9'
		]);
		// Still two distinct students: a match already claimed is never reused.
		expect(roster.rowByStudent.size).toBe(2);
	});
});

// ── Absences and §0 A5 ───────────────────────────────────────────────────────

describe('the absences the school year can show', () => {
	beforeEach(async () => {
		await db().execute(
			`INSERT INTO events (id, student_id, class_id, event_type, timestamp, session_key)
			 VALUES (?, ?, ?, 'absent', ?, ?), (?, ?, ?, 'absent', ?, ?), (?, ?, ?, 'in', ?, ?)`,
			[
				'e1',
				's1',
				CLASS_ID,
				Math.floor(new Date(2026, 8, 7).getTime() / 1000),
				'k1',
				'e2',
				's2',
				'other-class',
				Math.floor(new Date(2026, 8, 8).getTime() / 1000),
				'k2',
				'e3',
				's3',
				CLASS_ID,
				Math.floor(new Date(2026, 8, 9).getTime() / 1000),
				'k3'
			]
		);
	});

	it('keys absences by the month of the school year they fall in', async () => {
		const { byMonth, outside } = await loadAbsences(CLASS_ID, SCHOOL_YEAR_MONTHS);
		expect(byMonth.get('SEPTEMBER')).toEqual([{ studentId: 's1', date: '2026-09-07' }]);
		expect(outside).toEqual([]);
	});

	it('reports absences no worksheet exists for, rather than dropping them', async () => {
		await db().execute(
			`INSERT INTO events (id, student_id, class_id, event_type, timestamp, session_key)
			 VALUES ('e4', 's1', ?, 'absent', ?, 'k4')`,
			[CLASS_ID, Math.floor(new Date(2026, 5, 3).getTime() / 1000)]
		);
		const { outside } = await loadAbsences(CLASS_ID, SCHOOL_YEAR_MONTHS);
		expect(outside).toEqual([{ studentId: 's1', date: '2026-06-03' }]);
	});
});

describe('the read-only comparison', () => {
	const snapshot = (reportMonth: string, reportYear: number) => ({
		sheetName: `${reportMonth} ${reportYear}`,
		reportMonth,
		reportYear,
		xCount: 1,
		femaleStartRow: 30,
		learners: [],
		dayByColumn: new Map([[6, 7]]),
		marks: [{ rowIndex: 8, columnIndex: 6, value: SF2_ABSENT_MARK }]
	});

	it('accepts a mark the database can produce', () => {
		const unproven = unprovenMonths(
			[snapshot('SEPTEMBER', 2026)],
			new Map([['SEPTEMBER', [{ studentId: 's1', date: '2026-09-07' }]]]),
			new Map([['s1', 8]])
		);
		expect([...unproven]).toEqual([]);
	});

	it('blocks a month holding a mark the database has no record of', () => {
		const unproven = unprovenMonths(
			[snapshot('SEPTEMBER', 2026)],
			new Map([['SEPTEMBER', []]]),
			new Map([['s1', 8]])
		);
		expect(unproven.get('SEPTEMBER')).toContain('X mark(s) the database has no record of');
		expect(unproven.get('SEPTEMBER')).toContain('row 8 (2026-09-07)');
		expect(unproven.get('SEPTEMBER')).toContain('Nothing was written');
	});

	it('ignores a mark in a column the sheet prints no day for', () => {
		// Nothing to compare it against is not the same as a missing record.
		const noDay = { ...snapshot('SEPTEMBER', 2026), dayByColumn: new Map<number, number>() };
		const unproven = unprovenMonths([noDay], new Map([['SEPTEMBER', []]]), new Map([['s1', 8]]));
		expect([...unproven]).toEqual([]);
	});

	it('says nothing about a month the pre-split workbook has no sheet for', () => {
		expect([...unprovenMonths([], new Map([['SEPTEMBER', []]]), new Map([['s1', 8]]))]).toEqual([]);
	});
});

// ── The finished workbook ────────────────────────────────────────────────────

/**
 * The eleven month rows the pre-split template's own roster came from, so the
 * merge has a class to write.
 */
async function seedClass(): Promise<void> {
	await db().execute(
		`INSERT INTO sf2_templates (id, source_path, source_hash, school_id, school_name,
		    school_year, report_month, grade_level, section, adviser_name, school_head_name,
		    active_class_id, imported_at)
		 VALUES (?, ?, 'hash-1', '132839', 'ESPIRITU ELEMENTARY SCHOOL', ?, 'JUNE',
		    'GRADE 3', 'MATAPAT', 'DELA CRUZ, JUAN', 'SANTOS, MARIA', ?, 1)`,
		[LEGACY_ID, '/workbooks/SF2-GRADE-3.xlsx', SCHOOL_YEAR, CLASS_ID]
	);
	const learners = [
		['s1', 'DELA CRUZ, JUAN', 8, 'MALE'],
		['s2', 'SANTOS, MARIA', 30, 'FEMALE'],
		['s3', 'REYES, ANA', 31, 'FEMALE']
	] as const;
	for (const [studentId, name, rowIndex, genderBlock] of learners) {
		await db().execute(
			'INSERT INTO sf2_month_student_mappings (template_id, student_id, workbook_name, normalized_name, row_index, gender_block) VALUES (?, ?, ?, ?, ?, ?)',
			[LEGACY_ID, studentId, name, name.toUpperCase(), rowIndex, genderBlock]
		);
		await db().execute(
			'INSERT INTO students (id, name, class_id, created_at, gender) VALUES (?, ?, ?, 1, ?)',
			[studentId, name, CLASS_ID, genderBlock === 'MALE' ? 'male' : 'female']
		);
	}
	await db().execute(
		`INSERT INTO events (id, student_id, class_id, event_type, timestamp, session_key)
		 VALUES ('e1', 's1', ?, 'absent', ?, 'k1'), ('e2', 's2', ?, 'absent', ?, 'k2')`,
		[
			CLASS_ID,
			Math.floor(new Date(2026, 8, 7).getTime() / 1000),
			CLASS_ID,
			Math.floor(new Date(2026, 8, 7).getTime() / 1000)
		]
	);
	await db().execute(
		`INSERT INTO settings (id, day_start, day_end, late_after) VALUES ('app', '08:00', '15:00', '08:45')`
	);
}

function job(fixture: TemplateFixture, overrides: Partial<MergeJob> = {}): MergeJob {
	return {
		identity: { gradeLevel: 'GRADE 3', section: 'MATAPAT', templateId: LEGACY_ID },
		workbookPath: fixture.path,
		summaryCounts: COUNTS,
		now: () => 2026,
		...overrides
	};
}

async function seedMonthRow(): Promise<void> {
	await insertMonthTemplate({
		id: 'month-september',
		classId: CLASS_ID,
		schoolYear: SCHOOL_YEAR,
		reportMonth: 'SEPTEMBER',
		reportYear: 2026,
		sourcePath: '/workbooks/SF2-GRADE-3.xlsx'
	});
}

let fixture: TemplateFixture;

beforeEach(async () => {
	// D13's Documents path goes through the Tauri plugin, so the in-memory one is injected.
	useSf2WorkbookDir('/workbooks');
	fixture = await loadTemplate();
});

describe('the counts and summary block on a real sheet', () => {
	it('writes rows 53-71 across AR / AS / AT with a number in every cell', async () => {
		const totals = bundledTemplateTotalRows(TEMPLATE_ROSTER.maleCount, TEMPLATE_ROSTER.femaleCount);
		// The bundled template's own layout, which the counts below are addressed by.
		expect(totals).toEqual({ maleTotalRow: 29, femaleTotalRow: 49, combinedTotalRow: 50 });

		const workbook = await fixture.open();
		writeSummaryBlock(
			workbook,
			[
				{
					removeStaleSheets: false,
					request: {
						templateId: 'month-june',
						reportMonth: 'JUNE',
						reportYear: 2025,
						firstSchoolDay: 2,
						header: {
							schoolId: '',
							schoolName: '',
							schoolYear: '',
							reportMonth: 'JUNE',
							gradeLevel: '',
							section: '',
							adviserName: '',
							schoolHeadName: ''
						},
						learners: [
							...Array.from({ length: TEMPLATE_ROSTER.maleCount }, (_, index) => ({
								studentId: `m${index}`,
								rowIndex: 8 + index,
								name: `BOY, ${index}`,
								itemNumber: index + 1,
								genderBlock: 'MALE'
							})),
							...Array.from({ length: TEMPLATE_ROSTER.femaleCount }, (_, index) => ({
								studentId: `f${index}`,
								rowIndex: 30 + index,
								name: `GIRL, ${index}`,
								itemNumber: index + 1,
								genderBlock: 'FEMALE'
							}))
						],
						absences: [],
						sourceFemaleStartRow: 30
					}
				}
			],
			COUNTS
		);
		await fixture.save(workbook);

		const after = (await fixture.open()).getWorksheet('JUNE 2025') as Worksheet;
		expect(formulaOf(after.getCell('AR59'))).toBe('AR53+AR55-AR67-AR69+AR71');
		// Boys: 12 enrolled, 1 late, 2 transferred in, none out.
		expect(after.getCell('AR53').value).toBe(TEMPLATE_ROSTER.maleCount);
		expect(formulaResult(after.getCell('AR59'))).toBe(14);
		expect(after.getCell('AR53').value).toBe(12);
		// Girls: 14 enrolled, none late, 1 dropped out, 1 transferred out.
		expect(formulaResult(after.getCell('AS59'))).toBe(12);
		expect(formulaOf(after.getCell('AT59'))).toBe('AT53+AT55-AT67-AT69+AT71');
		expect(formulaResult(after.getCell('AT61'))).toBeCloseTo(100, 10);
		expect(typeof formulaResult(after.getCell('AT63'))).toBe('number');
		expect(typeof formulaResult(after.getCell('AT65'))).toBe('number');
		// The merges the summary block lives inside survived the round trip.
		expect(after.model.merges.length).toBeGreaterThan(0);
		expect(writableDayColumns(after)).toContain('F');
		void totals;
	});
});

// ── The job, end to end ──────────────────────────────────────────────────────

describe('a merge of a real bundled template', () => {
	beforeEach(async () => {
		await seedClass();
		await seedMonthRow();
	});

	it('parses and saves the twelve-month workbook once, not once per stage', async () => {
		// ExcelJS needs seconds per parse and per serialize of a file this size, and the
		// webview main thread does the work. The summary block used to need a second
		// full parse and a second save of the same file, which is most of why creating
		// or importing a workbook looked like it would never finish. Pinned here so the
		// second pass cannot quietly come back.
		const fileSystem = fixture.fileSystem;
		let reads = 0;
		let writes = 0;
		const realRead = fileSystem.readFile.bind(fileSystem);
		const realWrite = fileSystem.writeFileAtomic.bind(fileSystem);
		fileSystem.readFile = async (path) => {
			reads += 1;
			return realRead(path);
		};
		fileSystem.writeFileAtomic = async (path, contents) => {
			writes += 1;
			return realWrite(path, contents);
		};

		const { mergeWorkbookYear } = await import('../merge');
		const outcome = await mergeWorkbookYear(job(fixture));

		// One read, one write: the whole school year is built in the workbook the build
		// already had open. Before, the summary block's own pass made this 3 and 2.
		expect(outcome.verifiedCount).toBeGreaterThan(0);
		expect(reads).toBe(1);
		expect(writes).toBe(1);
	});

	it('reports the two absences it wrote and nothing outside the school year', async () => {
		const { mergeWorkbookYear } = await import('../merge');
		const outcome: Sf2SplitOutcome = await mergeWorkbookYear(job(fixture));

		expect(outcome.absencesOutsideSchoolYear).toBe(0);
		expect(outcome.verifiedCount).toBeGreaterThan(0);
		// Every month names the one file.
		expect(new Set(outcome.months.map((month) => month.fileName)).size).toBe(1);
		expect(outcome.message).toContain('All 12 months are on one workbook');
	});

	it('writes the X count into the month and reads it back from the saved file', async () => {
		const { mergeWorkbookYear } = await import('../merge');
		const outcome = await mergeWorkbookYear(job(fixture));
		const september = outcome.months[0];
		expect(september.reportMonth).toBe('SEPTEMBER');
		expect(september.sheetName).toBe('SEPTEMBER 2026');

		const workbook = await fixture.open();
		const sheet = workbook.getWorksheet('SEPTEMBER 2026');
		expect(sheet).toBeDefined();
		// s1 is at row 8 and was absent on 2026-09-07. `AM8` is the ABSENT count, and it
		// carries the value Excel caches for its COUNTIF - so this proves the mark was
		// written *and* counted, not merely that a cell holds an X.
		expect(formulaOf((sheet as Worksheet).getCell('AM8'))).toBe('COUNTIF(F8:AL8,"X")');
		// One X, not two: the form merges `L8:M8`, and a count that read the slave as
		// well as the master would be a number Excel disagrees with on open.
		expect(formulaResult((sheet as Worksheet).getCell('AM8'))).toBe(1);
		// The absence falls on 2026-09-07, the second week of a grid anchored on the 1st,
		// so it sits in that week's Monday column.
		expect(getCellText(sheet as Worksheet, 8, 12)).toBe(SF2_ABSENT_MARK);
		expect(september.xMarks).toBeGreaterThan(0);
	});

	it('records the month row, its day grid and the roster', async () => {
		const { mergeWorkbookYear } = await import('../merge');
		await mergeWorkbookYear(job(fixture));

		const row = await db().queryOne<{ id: string; source_path: string; report_year: number }>(
			'SELECT id, source_path, report_year FROM sf2_month_templates WHERE report_month = ?',
			['SEPTEMBER']
		);
		expect(row).toMatchObject({ source_path: fixture.path, report_year: 2026 });
		// By the month row's own id, because that is the id a write resolves the grid
		// by. A grid filed under the build's provisional id is a grid nothing can read.
		expect(row?.id).toBeDefined();
		const dates = await db().query<{ column_letter: string; sheet_name: string }>(
			'SELECT column_letter, sheet_name FROM sf2_month_date_mappings WHERE template_id = ?',
			[row?.id ?? '']
		);
		expect(dates.length).toBeGreaterThan(0);
		expect(new Set(dates.map((date) => date.sheet_name))).toEqual(new Set(['SEPTEMBER 2026']));
		const roster = await db().query<{ workbook_name: string; row_index: number }>(
			'SELECT workbook_name, row_index FROM sf2_month_student_mappings ORDER BY row_index'
		);
		expect(roster[0]).toMatchObject({ workbook_name: 'DELA CRUZ, JUAN', row_index: 8 });
	});

	it('writes a learner added to the app after the first merge onto every sheet', async () => {
		// The whole bug in one test: add the student, re-merge, find them in the file.
		const { mergeWorkbookYear } = await import('../merge');
		await mergeWorkbookYear(job(fixture));
		const alreadyMerged = await mergeWorkbookYear(job(fixture));
		expect(alreadyMerged.splitCompletedAt).not.toBeNull();

		await db().execute(
			'INSERT INTO students (id, name, class_id, created_at, gender) VALUES (?, ?, ?, 1, ?)',
			['s4', 'CRUZ, PEDRO', CLASS_ID, 'male']
		);

		// Without `force` the merge reports itself done and writes nothing, which is
		// exactly why the learner never reached the workbook.
		await mergeWorkbookYear(job(fixture, { force: true }));

		const workbook = await fixture.open();
		for (const month of ['SEPTEMBER 2026', 'JUNE 2027']) {
			expect(getCellText(workbook.getWorksheet(month) as Worksheet, 9, 3)).toBe('CRUZ, PEDRO');
		}
		const roster = await db().query<{ student_id: string; row_index: number }>(
			'SELECT student_id, row_index FROM sf2_month_student_mappings WHERE student_id = ?',
			['s4']
		);
		// One mapping per month, all on the row the name was written to.
		expect(new Set(roster.map((row) => row.row_index))).toEqual(new Set([9]));
		expect(roster).toHaveLength(12);
	});

	it('leaves the file untouched when a month cannot be proven', async () => {
		const before = fixture.fileSystem.files.get(fixture.path);
		await db().execute(
			`INSERT INTO sf2_date_mappings (template_id, sheet_name, date, column_letter, column_index)
			 VALUES (?, 'SEPTEMBER 2026', '2026-09-07', 'F', 6)`,
			[LEGACY_ID]
		);
		// The pre-split workbook holds an X the database cannot produce, so the merge
		// must refuse rather than overwrite the only copy of it.
		const { mergeWorkbookYear } = await import('../merge');
		const legacySource = '/workbooks/legacy.xlsx';
		await fixture.fileSystem.writeFileAtomic(legacySource, before as Uint8Array);
		const outcome = await mergeWorkbookYear(job(fixture, { legacySourcePath: legacySource }));

		expect(outcome.needsAttentionCount).toBeGreaterThan(0);
		expect(
			outcome.months.some(
				(month) =>
					month.status === 'needsAttention' && (month.detail ?? '').includes('Nothing was written')
			)
		).toBe(true);
		expect(fixture.fileSystem.files.get(fixture.path)).toBe(before);
	});
});

describe('opening the template the merge builds from', () => {
	it('reads the ground truth the day-column rules depend on', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.getWorksheet('JUNE 2025') as Worksheet;
		// Only 25 of the F..AL columns are addressable: the form merges them in pairs.
		expect(writableDayColumns(sheet)).toHaveLength(25);
		// The day row the grid is written into is the form's own, not a guess.
		expect(getCellText(sheet, 6, 6)).not.toBe('');
		expect(getCellText(sheet, 7, 6)).toMatch(/^M/i);
	});
});
