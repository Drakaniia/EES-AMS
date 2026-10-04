import { db, useTestDb } from '$lib/db/repos/__tests__/schema';
import { errorMessage } from '$lib/db';
import type { AppError } from '$lib/db';
import {
	dateMappingsForTemplate,
	findTemplate,
	latestTemplateForClass,
	legacyDateMappingsInMonth,
	listTemplates,
	setLastSyncedAt,
	studentMappingsForTemplate,
	templateSummary,
	updateTemplateWithMappings,
	upsertTemplateWithMappings
} from '../repository';
import { beforeEach, describe, expect, test } from 'vitest';
import type { Sf2DateMapping } from '../metadata';
import type { Sf2StudentMappingRecord, Sf2TemplateRecord } from '../repository';

/**
 * The pre-split tables, in the shape the migration chain leaves them.
 *
 * The shared repo fixture declares only the two columns the other repos touch, and it
 * declares `sf2_student_mappings.id` as a primary key the real table does not have, so
 * both tables are recreated here rather than patched. That fixture's convention - a
 * test declares the columns its own SQL reads - still holds; the DDL just has to be
 * the real one for the SQL to mean anything.
 */
const SF2_SCHEMA = `
DROP TABLE sf2_student_mappings;
CREATE TABLE sf2_student_mappings (
	template_id TEXT NOT NULL,
	student_id TEXT NOT NULL,
	workbook_name TEXT NOT NULL,
	normalized_name TEXT NOT NULL,
	row_index INTEGER NOT NULL,
	gender_block TEXT,
	PRIMARY KEY (template_id, student_id)
);

DROP TABLE sf2_templates;
CREATE TABLE sf2_templates (
	id TEXT PRIMARY KEY NOT NULL,
	source_path TEXT,
	source_hash TEXT NOT NULL DEFAULT '',
	school_id TEXT,
	school_name TEXT,
	school_year TEXT,
	report_month TEXT,
	grade_level TEXT,
	section TEXT,
	adviser_name TEXT,
	school_head_name TEXT,
	layout_fingerprint TEXT,
	active_class_id TEXT,
	imported_at INTEGER NOT NULL,
	last_synced_at INTEGER,
	UNIQUE (source_hash, grade_level, section)
);

CREATE TABLE sf2_date_mappings (
	template_id TEXT NOT NULL,
	sheet_name TEXT,
	date TEXT NOT NULL,
	column_letter TEXT NOT NULL,
	column_index INTEGER NOT NULL,
	PRIMARY KEY (template_id, date)
);
`;

useTestDb();

const CLASS_ID = 'class-1';

function template(overrides: Partial<Sf2TemplateRecord> = {}): Sf2TemplateRecord {
	return {
		id: 'template-1',
		sourcePath: 'C:/workbooks/SF2-GRADE-3-A.xlsx',
		sourceHash: 'hash-1',
		schoolId: 'S-1',
		schoolName: 'Test School',
		schoolYear: '2026-2027',
		reportMonth: 'SEPTEMBER',
		gradeLevel: '3',
		section: 'A',
		adviserName: 'Dela Cruz',
		schoolHeadName: 'Santos',
		layoutFingerprint: 'fingerprint-1',
		activeClassId: CLASS_ID,
		importedAt: 1_000,
		lastSyncedAt: undefined,
		...overrides
	};
}

function studentMapping(overrides: Partial<Sf2StudentMappingRecord> = {}): Sf2StudentMappingRecord {
	return {
		templateId: 'template-1',
		studentId: 'student-1',
		workbookName: 'DELA CRUZ, JUAN',
		normalizedName: 'dela cruz juan',
		rowIndex: 8,
		genderBlock: 'MALE',
		...overrides
	};
}

function dateMapping(overrides: Partial<Sf2DateMapping> = {}): Sf2DateMapping {
	return {
		templateId: 'template-1',
		sheetName: 'SEPTEMBER 2026',
		date: '2026-09-01',
		columnLetter: 'F',
		columnIndex: 6,
		...overrides
	};
}

/** A template holding one student mapping and one date mapping: the "good" state. */
async function seedTemplate(): Promise<Sf2TemplateRecord> {
	await db().execute(
		"INSERT INTO students (id, name, class_id, created_at) VALUES ('student-1', 'Juan Dela Cruz', 'class-1', 1)"
	);
	await upsertTemplateWithMappings(template(), [studentMapping()], [dateMapping()]);
	return template();
}

function thrownBy(promise: Promise<unknown>): Promise<AppError> {
	return promise.then(
		() => expect.unreachable('the call was expected to fail'),
		(thrown: AppError) => thrown
	);
}

beforeEach(async () => {
	await db().script(SF2_SCHEMA);
});

describe('findTemplate', () => {
	test('answers undefined for an identity nothing is stored under', async () => {
		expect(await findTemplate('nope', '3', 'A')).toBeUndefined();
	});

	test('finds by source hash, grade level and section', async () => {
		await upsertTemplateWithMappings(template(), [studentMapping()], [dateMapping()]);
		const found = await findTemplate('hash-1', '3', 'A');
		expect(found?.id).toBe('template-1');
	});
});

describe('upsertTemplateWithMappings', () => {
	test('replaces both mapping sets and keeps one row per identity', async () => {
		await upsertTemplateWithMappings(template(), [studentMapping()], [dateMapping()]);
		await upsertTemplateWithMappings(
			template({ importedAt: 2_000, layoutFingerprint: 'fingerprint-2' }),
			[studentMapping({ studentId: 'student-2', rowIndex: 9 })],
			[dateMapping({ date: '2026-09-02', columnLetter: 'H', columnIndex: 8 })]
		);

		expect(await studentMappingsForTemplate('template-1')).toHaveLength(1);
		expect((await studentMappingsForTemplate('template-1'))[0].studentId).toBe('student-2');
		const dates = await dateMappingsForTemplate('template-1');
		expect(dates.map((date) => date.date)).toEqual(['2026-09-02']);
		expect((await listTemplates())[0].id).toBe('template-1');
	});

	test('normalises the school year on read, so one label has one spelling', async () => {
		await upsertTemplateWithMappings(template({ schoolYear: '2026 - 2027' }), [], []);
		expect((await listTemplates())[0].schoolYear).toBe('2026-2027');
	});

	test('reads a blank metadata column as blank rather than failing', async () => {
		await db().execute(
			`INSERT INTO sf2_templates
			 (id, active_class_id, school_name, grade_level, section, imported_at)
			 VALUES ('t-blank', 'class-1', 'Only The School', '3', 'A', 1)`
		);
		const found = await findTemplate('', '3', 'A');
		expect(found?.schoolName).toBe('Only The School');
		expect(found?.schoolId).toBe('');
	});
});

describe('updateTemplateWithMappings', () => {
	test('refuses an empty date analysis and leaves the good mappings alone', async () => {
		// The Excel read only sees visible monthly sheets, so an empty result means the
		// target sheet never became visible. Committing it deletes every date mapping.
		await seedTemplate();
		const error = await thrownBy(updateTemplateWithMappings(template(), [studentMapping()], []));
		expect(errorMessage(error)).toBe(
			'invalid input: The SF2 workbook produced no calendar dates. The existing mappings were left untouched.'
		);
		expect(await dateMappingsForTemplate('template-1')).toHaveLength(1);
		expect(await studentMappingsForTemplate('template-1')).toHaveLength(1);
	});

	test('refuses an empty roster, which would unmap every learner', async () => {
		await seedTemplate();
		const error = await thrownBy(updateTemplateWithMappings(template(), [], [dateMapping()]));
		expect(errorMessage(error)).toBe(
			'invalid input: The SF2 workbook produced no learners. The existing mappings were left untouched.'
		);
		expect(await studentMappingsForTemplate('template-1')).toHaveLength(1);
		expect(await dateMappingsForTemplate('template-1')).toHaveLength(1);
	});

	test('applies a usable analysis', async () => {
		await seedTemplate();
		await updateTemplateWithMappings(
			template(),
			[studentMapping()],
			[dateMapping(), dateMapping({ date: '2026-09-02', columnLetter: 'H', columnIndex: 8 })]
		);
		expect(await dateMappingsForTemplate('template-1')).toHaveLength(2);
	});

	test('drops the previous year’s dates rather than leaving duplicates', async () => {
		// A template analysed in 2025 holds `2025-07-01`, which `sf2_date_mappings_for_report_month`
		// would normalise onto the same day as `2026-07-01` - at a *different* column,
		// because the weekday differs. Two rows for one day corrupt mark placement.
		await upsertTemplateWithMappings(
			template(),
			[studentMapping()],
			[dateMapping({ date: '2025-07-01', columnLetter: 'H', columnIndex: 8 })]
		);
		await updateTemplateWithMappings(
			template(),
			[studentMapping()],
			[dateMapping({ date: '2026-07-01', columnLetter: 'I', columnIndex: 9 })]
		);
		const dates = await dateMappingsForTemplate('template-1');
		expect(dates.map((date) => date.date)).toEqual(['2026-07-01']);
	});

	test('refuses a template id that is not on file', async () => {
		await seedTemplate();
		const error = await thrownBy(
			updateTemplateWithMappings(template({ id: 'missing' }), [studentMapping()], [dateMapping()])
		);
		expect(errorMessage(error)).toBe('invalid input: Selected SF2 workbook was not found');
	});
});

describe('latestTemplateForClass', () => {
	test('answers undefined for a class with no workbook', async () => {
		expect(await latestTemplateForClass('nobody')).toBeUndefined();
	});

	test('answers the newest import, not the first row', async () => {
		await upsertTemplateWithMappings(template(), [studentMapping()], [dateMapping()]);
		await upsertTemplateWithMappings(
			template({ id: 'template-2', sourceHash: 'hash-2', importedAt: 2_000 }),
			[],
			[]
		);
		expect((await latestTemplateForClass(CLASS_ID))?.id).toBe('template-2');
	});
});

describe('templateSummary', () => {
	test('drops the layout fingerprint and renames the class column', () => {
		expect(templateSummary(template())).toEqual({
			id: 'template-1',
			sourcePath: 'C:/workbooks/SF2-GRADE-3-A.xlsx',
			schoolId: 'S-1',
			schoolName: 'Test School',
			schoolYear: '2026-2027',
			reportMonth: 'SEPTEMBER',
			gradeLevel: '3',
			section: 'A',
			adviserName: 'Dela Cruz',
			schoolHeadName: 'Santos',
			classId: CLASS_ID,
			importedAt: 1_000
		});
	});
});

describe('setLastSyncedAt', () => {
	test('records and clears the sync timestamp', async () => {
		await seedTemplate();
		await setLastSyncedAt('template-1', 1_700_000_000);
		expect((await latestTemplateForClass(CLASS_ID))?.lastSyncedAt).toBe(1_700_000_000);
		await setLastSyncedAt('template-1', undefined);
		expect((await latestTemplateForClass(CLASS_ID))?.lastSyncedAt).toBeUndefined();
	});
});

describe('studentMappingsForTemplate', () => {
	test('answers in workbook row order', async () => {
		await upsertTemplateWithMappings(
			template(),
			[
				studentMapping({ studentId: 'b', rowIndex: 30, genderBlock: 'FEMALE' }),
				studentMapping({ studentId: 'a', rowIndex: 8 })
			],
			[dateMapping()]
		);
		const rows = await studentMappingsForTemplate('template-1');
		expect(rows.map((row) => row.studentId)).toEqual(['a', 'b']);
	});
});

describe('legacyDateMappingsInMonth', () => {
	beforeEach(async () => {
		await upsertTemplateWithMappings(
			template(),
			[studentMapping()],
			[
				dateMapping({ date: '2025-07-01', columnLetter: 'H', columnIndex: 8 }),
				dateMapping({ date: '2026-07-01', columnLetter: 'I', columnIndex: 9 }),
				dateMapping({ date: '2026-07-31', columnLetter: 'AJ', columnIndex: 36 }),
				dateMapping({ date: '2026-08-03', columnLetter: 'F', columnIndex: 6 }),
				dateMapping({ date: 'not-a-date', columnLetter: 'Z', columnIndex: 26 })
			]
		);
	});

	test('is a closed range over fixed-width ISO dates', async () => {
		const july = await legacyDateMappingsInMonth('template-1', '2026-07-01', '2026-07-31');
		expect(july.map((date) => date.date)).toEqual(['2026-07-01', '2026-07-31']);
	});

	test('cannot hand one month another month’s columns', async () => {
		// 2026-07-01 is a Wednesday (col I) and 2025-07-01 a Tuesday (col H). Reading
		// the whole table and sorting it out client-side is how a September grid ends
		// up wearing October's columns.
		const july = await legacyDateMappingsInMonth('template-1', '2026-07-01', '2026-07-31');
		expect(july.some((date) => date.columnLetter === 'H')).toBe(false);
	});

	test('excludes a malformed stored date rather than reading it as a day', async () => {
		const all = await legacyDateMappingsInMonth('template-1', '0000-01-01', '9999-12-31');
		expect(all.some((date) => date.date === 'not-a-date')).toBe(false);
	});
});
