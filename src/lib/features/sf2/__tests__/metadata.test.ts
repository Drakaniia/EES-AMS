/**
 * Metadata, the date mappings that go with it, and the layout fingerprint.
 *
 * The fingerprint assertions are the interesting ones: the value is stored in
 * `sf2_templates.layout_fingerprint` by the Rust side, so the hash has to be the same
 * FNV-1a over the same field order or every existing row reads as a different
 * workbook.
 */
import { describe, expect, it } from 'vitest';
import {
	dateMappingsAreCurrentForReportMonth,
	dateMappingsFromAnalysis,
	firstSchoolDayFromMappings,
	metadataFromAnalysis,
	metadataFromDraft,
	metadataFromImportAnalysis,
	sf2DateMappingsForReportMonth,
	sf2MetadataWarnings,
	templateMetadata
} from '../metadata';
import type { Sf2DateMapping } from '../metadata';
import { hashBytes, layoutFingerprint } from '../workbook-files';
import type { Sf2WorkbookAnalysis } from '../calendar';

/** `AppError` is a plain object, not an `Error`, so `toThrow` cannot read it. */
function appErrorOf(run: () => unknown): { kind: string; detail: string } {
	try {
		run();
	} catch (thrown) {
		return thrown as { kind: string; detail: string };
	}
	throw new Error('expected an AppError, nothing was thrown');
}

const JUNE = { reportMonth: 'JUNE', schoolYear: '2025-2026' };

function mapping(overrides: Partial<Sf2DateMapping> = {}): Sf2DateMapping {
	return {
		templateId: 't1',
		sheetName: 'JUNE 2025',
		date: '2025-06-02',
		columnLetter: 'F',
		columnIndex: 6,
		...overrides
	};
}

function analysis(overrides: Partial<Sf2WorkbookAnalysis> = {}): Sf2WorkbookAnalysis {
	return {
		schoolId: ' 401234 ',
		schoolName: ' TAPATIN ELEMENTARY SCHOOL ',
		schoolYear: ' 2024-2025 ',
		reportMonth: ' JUNE ',
		gradeLevel: ' 3 ',
		section: ' MATAPAT ',
		adviserName: ' ANA ',
		schoolHeadName: ' BEN ',
		learners: [],
		dates: [mapping()],
		sheets: [{ name: 'JUNE 2025', usedRange: 'A1:AT71' }],
		...overrides
	};
}

describe('metadataFromAnalysis', () => {
	it('trims every header field and describes the calendar without touching it', () => {
		expect(metadataFromAnalysis(analysis())).toEqual({
			schoolId: '401234',
			schoolName: 'TAPATIN ELEMENTARY SCHOOL',
			schoolYear: '2024-2025',
			reportMonth: 'JUNE',
			gradeLevel: '3',
			section: 'MATAPAT',
			adviserName: 'ANA',
			schoolHeadName: 'BEN',
			configureCalendar: false
		});
	});
});

describe('metadataFromImportAnalysis', () => {
	it('configures the calendar from the day the workbook already started on', () => {
		const metadata = metadataFromImportAnalysis(
			analysis({
				dates: [mapping({ date: '2024-06-03' }), mapping({ date: '2024-06-04', columnLetter: 'H' })]
			})
		);
		expect(metadata.configureCalendar).toBe(true);
		// June 2024 opens on a Saturday, so the first day the grid can hold is the 3rd.
		expect(metadata.firstSchoolDay).toBe(3);
	});

	it('leaves the calendar alone when the report month names no month', () => {
		const metadata = metadataFromImportAnalysis(analysis({ reportMonth: 'smiling' }));
		expect(metadata.configureCalendar).toBe(false);
		expect(metadata.firstSchoolDay).toBeUndefined();
	});
});

describe('metadataFromDraft', () => {
	const draft = {
		schoolId: '401234',
		schoolName: 'TAPATIN',
		schoolYear: '2025-2026',
		reportMonth: 'JULY',
		gradeLevel: '3',
		section: 'MATAPAT',
		adviserName: 'ANA',
		schoolHeadName: 'BEN',
		firstSchoolDay: 1,
		learnerNames: []
	};

	it('requires every field a workbook written from it will need', () => {
		expect(appErrorOf(() => metadataFromDraft({ ...draft, schoolYear: '  ' })).detail).toBe(
			'School Year is required'
		);
		expect(appErrorOf(() => metadataFromDraft({ ...draft, reportMonth: '' })).detail).toBe(
			'Report Month is required'
		);
		expect(appErrorOf(() => metadataFromDraft({ ...draft, section: '' })).detail).toBe(
			'Section is required'
		);
	});

	it('requires a first attendance day and validates it', () => {
		expect(
			appErrorOf(() => metadataFromDraft({ ...draft, firstSchoolDay: undefined })).detail
		).toBe('First attendance day is required for SF2 templates');
		expect(appErrorOf(() => metadataFromDraft({ ...draft, firstSchoolDay: 5 })).detail).toBe(
			'First attendance day must be a Monday-Friday school day'
		);
		expect(metadataFromDraft(draft).firstSchoolDay).toBe(1);
	});
});

describe('templateMetadata', () => {
	it('describes a stored template without claiming a calendar it will not write', () => {
		expect(
			templateMetadata({
				schoolId: '1',
				schoolName: 'TAPATIN',
				schoolYear: '2025-2026',
				reportMonth: 'JULY',
				gradeLevel: '3',
				section: 'MATAPAT',
				adviserName: '',
				schoolHeadName: '',
				configureCalendar: true,
				firstSchoolDay: 1
			})
		).toMatchObject({ configureCalendar: false, firstSchoolDay: undefined });
	});
});

describe('dateMappingsAreCurrentForReportMonth', () => {
	it('accepts mappings that describe the template month', () => {
		expect(dateMappingsAreCurrentForReportMonth(JUNE, [mapping()])).toBe(true);
	});

	it('rejects an empty mapping set', () => {
		expect(dateMappingsAreCurrentForReportMonth(JUNE, [])).toBe(false);
	});

	it('rejects a mapping from another month, another year or a weekend', () => {
		expect(dateMappingsAreCurrentForReportMonth(JUNE, [mapping({ date: '2025-07-01' })])).toBe(
			false
		);
		expect(dateMappingsAreCurrentForReportMonth(JUNE, [mapping({ date: '2024-06-03' })])).toBe(
			false
		);
		expect(dateMappingsAreCurrentForReportMonth(JUNE, [mapping({ date: '2025-06-07' })])).toBe(
			false
		);
	});

	it('rejects a mapping whose sheet is not the report month', () => {
		expect(dateMappingsAreCurrentForReportMonth(JUNE, [mapping({ sheetName: 'JULY 2025' })])).toBe(
			false
		);
		expect(dateMappingsAreCurrentForReportMonth(JUNE, [mapping({ sheetName: 'JUNE' })])).toBe(
			false
		);
	});

	it('rejects a report month that names no month', () => {
		expect(
			dateMappingsAreCurrentForReportMonth({ reportMonth: 'smiling', schoolYear: '2024-2025' }, [
				mapping()
			])
		).toBe(false);
	});
});

describe('sf2DateMappingsForReportMonth', () => {
	it('keeps the report month and re-dates it into the template year', () => {
		const mappings = sf2DateMappingsForReportMonth(JUNE, [
			mapping({ date: '2024-06-03' }),
			mapping({ date: '2025-07-01', columnLetter: 'H' })
		]);
		expect(mappings).toEqual([{ ...mapping({ date: '2024-06-03' }), date: '2025-06-03' }]);
	});

	it('returns nothing for a report month that names no month', () => {
		expect(
			sf2DateMappingsForReportMonth({ reportMonth: 'smiling', schoolYear: '2024-2025' }, [
				mapping()
			])
		).toEqual([]);
	});
});

describe('firstSchoolDayFromMappings', () => {
	it('is the earliest day the mappings recorded', () => {
		expect(
			firstSchoolDayFromMappings([mapping({ date: '2025-06-10' }), mapping({ date: '2025-06-09' })])
		).toBe(9);
	});

	it('falls back to the first of the month when nothing is readable', () => {
		expect(firstSchoolDayFromMappings([])).toBe(1);
		expect(firstSchoolDayFromMappings([mapping({ date: 'not a date' })])).toBe(1);
	});
});

describe('sf2MetadataWarnings', () => {
	it('names every blank field the way the form names it', () => {
		expect(
			sf2MetadataWarnings(metadataFromAnalysis(analysis({ adviserName: ' ', schoolHeadName: '' })))
		).toEqual([
			'Signature of Adviser over Printed Name / Generated thru LIS adviser name is blank in this SF2 workbook.',
			'Signature of School Head over Printed Name is blank in this SF2 workbook.'
		]);
	});

	it('says nothing about a complete header', () => {
		expect(sf2MetadataWarnings(metadataFromAnalysis(analysis()))).toEqual([]);
	});
});

describe('dateMappingsFromAnalysis', () => {
	it('copies the analysis dates onto the template', () => {
		expect(dateMappingsFromAnalysis('t9', analysis())).toEqual([mapping({ templateId: 't9' })]);
	});
});

describe('hashBytes', () => {
	it('is FNV-1a, so a fingerprint written by Rust still matches', () => {
		// The FNV-1a 64-bit offset basis, which is also what an empty input hashes
		// to: the Rust `Hasher` treated a zero state as "nothing hashed yet".
		expect(hashBytes(new Uint8Array())).toBe('cbf29ce484222325');
		expect(hashBytes(new TextEncoder().encode('a'))).toBe('af63dc4c8601ec8c');
	});

	it('pads to the 16 characters a u64 prints as', () => {
		expect(hashBytes(new Uint8Array())).toHaveLength(16);
	});
});

describe('layoutFingerprint', () => {
	it('changes when a learner row moves or is renamed', () => {
		const base = analysis({ learners: [{ rowIndex: 8, name: 'Dela Cruz, Juan' }] });
		const moved = analysis({ learners: [{ rowIndex: 9, name: 'Dela Cruz, Juan' }] });
		const renamed = analysis({ learners: [{ rowIndex: 8, name: 'Dela Cruz, Juana' }] });

		expect(layoutFingerprint(base)).toHaveLength(16);
		expect(layoutFingerprint(moved)).not.toBe(layoutFingerprint(base));
		expect(layoutFingerprint(renamed)).not.toBe(layoutFingerprint(base));
	});

	it('changes when a day column or a sheet moves', () => {
		const base = analysis();
		expect(layoutFingerprint(analysis({ dates: [mapping({ columnLetter: 'H' })] }))).not.toBe(
			layoutFingerprint(base)
		);
		expect(
			layoutFingerprint(analysis({ sheets: [{ name: 'JUNE 2025', usedRange: 'A1:AT72' }] }))
		).not.toBe(layoutFingerprint(base));
	});

	it('is stable for the same workbook', () => {
		expect(layoutFingerprint(analysis())).toBe(layoutFingerprint(analysis()));
	});
});
