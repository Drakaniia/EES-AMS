/**
 * The attendance service: the four commands `$lib/api/sf2.ts` calls, and the
 * month resolution they share.
 *
 * No Excel and no COM: the workbook half uses `MemoryFileSystem` plus the real
 * converted DepEd template, the database half the Node driver.
 */
import { afterEach, describe, expect, it } from 'vitest';
import {
	db,
	DAY_ONE,
	insertMonthTemplate,
	MONTH,
	SCHOOL_YEAR,
	seedClass,
	seedMonth,
	SEEDED_DAYS,
	SHEET,
	useAttendanceTestDb
} from './attendance-fixture';
import { MemoryFileSystem, useFileSystem } from '$lib/platform/fs';
import { listEventsForClassAndDateRange } from '$lib/db/repos/events';
import { loadTemplate } from '$lib/features/excel/__tests__/template-fixture';
import {
	importAbsentMarksFromWorkbook,
	NO_MONTH_SELECTED_MESSAGE,
	presentAllPreviewAttendance,
	setPreviewAttendanceLightweight,
	syncAndOpenSf2Workbook,
	WORKBOOK_MISSING_MESSAGE
} from '../attendance/attendance-service';
import {
	hasAbsentEventForDay,
	hasPresentEventForDay,
	setAttendanceEventForDay,
	SF2_PREVIEW_CORRECTION
} from '../attendance/attendance-events';
import { resolveMonthWriteContext, UNKNOWN_MONTH_MESSAGE } from '../attendance/write-context';
import { writeAttendanceToWorkbook } from '../attendance/attendance-write';
import { NO_MAPPED_DATES } from '../guard';

useAttendanceTestDb();

afterEach(() => {
	// The service resolves workbook paths through the app's file-system seam.
	// `loadTemplate()` rebinds its own, so unbinding here only matters to the tests
	// that never open a workbook.
	useFileSystem(null);
});

/** A memory file system bound for a test that never opens the template. */
function useMemoryFs(): void {
	useFileSystem(new MemoryFileSystem());
}

describe('resolveMonthWriteContext', () => {
	it('refuses a month name it cannot resolve, rather than writing another month', async () => {
		useMemoryFs();
		const { classId } = await seedClass();
		await insertMonthTemplate({ classId, sourcePath: 'C:/workbooks/x.xlsx' });
		for (const unresolvable of ['Halloweeen', '', '   ', '2026-09']) {
			await expect(resolveMonthWriteContext(classId, unresolvable)).rejects.toMatchObject({
				kind: 'InvalidInput',
				detail: UNKNOWN_MONTH_MESSAGE
			});
		}
	});

	it('falls back to the latest month row when the asked month has none', async () => {
		useMemoryFs();
		const { classId } = await seedClass();
		await insertMonthTemplate({ classId, sourcePath: 'C:/workbooks/x.xlsx' });
		// The default row is SEPTEMBER 2026 with no mapped days, so the fallback
		// resolves to it and refuses on the empty grid rather than the month.
		await expect(resolveMonthWriteContext(classId, 'MARCH', SCHOOL_YEAR)).rejects.toMatchObject({
			detail: expect.stringContaining('No attendance days are mapped to SEPTEMBER')
		});
	});

	it('refuses a month with no row at all', async () => {
		useMemoryFs();
		const { classId } = await seedClass();
		await expect(resolveMonthWriteContext(classId, MONTH, SCHOOL_YEAR)).rejects.toMatchObject({
			kind: 'InvalidInput',
			detail: 'No SF2 template imported for this class'
		});
	});

	it('refuses a month with a roster but no mapped days', async () => {
		useMemoryFs();
		const { classId } = await seedClass();
		await insertMonthTemplate({
			classId,
			schoolYear: SCHOOL_YEAR,
			reportMonth: MONTH,
			reportYear: 2025,
			sourcePath: 'C:/workbooks/x.xlsx'
		});
		await expect(resolveMonthWriteContext(classId, MONTH, SCHOOL_YEAR)).rejects.toMatchObject({
			kind: 'InvalidInput',
			detail: expect.stringContaining('No attendance days are mapped')
		});
	});

	it('resolves the month the caller named, with its stored year and worksheet', async () => {
		const { classId } = await seedClass();
		const fixture = await loadTemplate();
		await seedMonth({ classId, path: fixture.path });

		// Asked for in lower case; it canonicalises to the stored uppercase name.
		const context = await resolveMonthWriteContext(classId, 'june', SCHOOL_YEAR);
		expect(context.reportMonth).toBe(MONTH);
		expect(context.reportYear).toBe(2025);
		expect(context.sheetName).toBe('JUNE 2025');
		expect(context.roster.map((entry) => entry.studentId)).toEqual(['s1', 's2']);
		expect(context.dates.map((entry) => entry.date)).toEqual(SEEDED_DAYS.map((d) => d.date));
		// Every day is stamped with the month’s worksheet, so a write cannot address a
		// day on some other sheet.
		expect(new Set(context.dates.map((entry) => entry.sheetName))).toEqual(new Set([SHEET]));
	});
});

describe('setPreviewAttendanceLightweight', () => {
	it('writes the mark and clears every month of the class as unsynced', async () => {
		useMemoryFs();
		const { classId, firstId } = await seedClass();
		const fixture = await loadTemplate();
		await seedMonth({ classId, path: fixture.path });
		await db().execute(
			'UPDATE sf2_month_templates SET last_synced_at = 1700000000 WHERE active_class_id = ?',
			[classId]
		);

		await setPreviewAttendanceLightweight({
			classId,
			studentId: firstId,
			date: '2026-09-01',
			present: false
		});

		const events = await listEventsForClassAndDateRange(classId, '2026-09-01', '2026-09-01');
		expect(events).toHaveLength(1);
		expect(events[0].type).toBe('absent');
		expect(events[0].overrideReason).toBe(SF2_PREVIEW_CORRECTION);
		const months = await db().query<{ last_synced_at: number | null }>(
			'SELECT last_synced_at FROM sf2_month_templates WHERE active_class_id = ?',
			[classId]
		);
		expect(months.every((row) => row.last_synced_at === null)).toBe(true);
	});

	it('needs no month row at all, so a migrated install can still correct a cell', async () => {
		useMemoryFs();
		const { classId, firstId } = await seedClass();
		await setPreviewAttendanceLightweight({
			classId,
			studentId: firstId,
			date: '2026-09-01',
			present: true
		});
		expect(await hasPresentEventForDay(firstId, classId, '2026-09-01')).toBe(true);
	});

	it('records a mark on an unmapped day, which the export filters out later', async () => {
		useMemoryFs();
		const { classId, firstId } = await seedClass();
		await setPreviewAttendanceLightweight({
			classId,
			studentId: firstId,
			date: '2026-09-30',
			present: false
		});
		expect(await hasAbsentEventForDay(firstId, classId, '2026-09-30')).toBe(true);
	});

	it('names the class and the learner when either is missing', async () => {
		useMemoryFs();
		const { classId, firstId } = await seedClass();
		await expect(
			setPreviewAttendanceLightweight({
				classId: 'no-such-class',
				studentId: firstId,
				date: '2026-09-01',
				present: false
			})
		).rejects.toMatchObject({ detail: 'Selected class was not found' });
		await expect(
			setPreviewAttendanceLightweight({
				classId,
				studentId: 'no-such-student',
				date: '2026-09-01',
				present: false
			})
		).rejects.toMatchObject({ detail: 'Selected student was not found' });
	});
});

describe('syncAndOpenSf2Workbook', () => {
	it('refuses an open with no month selected', async () => {
		useMemoryFs();
		const { classId } = await seedClass();
		await expect(syncAndOpenSf2Workbook({ classId, reportMonth: '  ' })).rejects.toMatchObject({
			detail: NO_MONTH_SELECTED_MESSAGE
		});
	});

	it('refuses when the workbook the month row names is gone', async () => {
		useMemoryFs();
		const { classId } = await seedClass();
		await seedMonth({ classId, path: '/workbooks/missing.xlsx' });
		await expect(syncAndOpenSf2Workbook({ classId, reportMonth: MONTH })).rejects.toMatchObject({
			detail: WORKBOOK_MISSING_MESSAGE
		});
	});

	it('writes the month’s absences, stamps the sync and reports the ten steps', async () => {
		const { classId, firstId } = await seedClass();
		const fixture = await loadTemplate();
		await seedMonth({ classId, path: fixture.path });
		await setAttendanceEventForDay({
			studentId: firstId,
			classId,
			date: DAY_ONE,
			dayStart: '07:30',
			eventType: 'absent',
			reason: 'test'
		});

		const steps: number[] = [];
		const path = await syncAndOpenSf2Workbook({
			classId,
			reportMonth: MONTH,
			progress: (update) => steps.push(update.current)
		});

		expect(path).toBe(fixture.path);
		// The outer steps are 1..10; the write phase reports on the 100-point scale in
		// between, which is why the bar crawls rather than pausing at 60%.
		expect(steps).toContain(1);
		expect(steps).toContain(10);
		expect(Math.max(...steps.filter((step) => step <= 10))).toBe(10);
		const stored = await db().queryOne<{ last_synced_at: number | null }>(
			'SELECT last_synced_at FROM sf2_month_templates WHERE active_class_id = ?',
			[classId]
		);
		expect(stored?.last_synced_at).toBeGreaterThan(0);
	});

	it('reports the path rather than opening a process', async () => {
		// D14: the app writes the file; opening it is a button and the opener plugin.
		const { classId } = await seedClass();
		const fixture = await loadTemplate();
		await seedMonth({ classId, path: fixture.path });
		await expect(syncAndOpenSf2Workbook({ classId, reportMonth: MONTH })).resolves.toBe(
			fixture.path
		);
	});

	it('names the missing file, not the unmapped month, when a month has no days', async () => {
		// Rust reached the missing-workbook check from inside the read-only branch,
		// so an empty mapping set could not answer first.
		useMemoryFs();
		const { classId } = await seedClass();
		await insertMonthTemplate({ classId, sourcePath: '/workbooks/missing.xlsx' });
		await expect(syncAndOpenSf2Workbook({ classId, reportMonth: MONTH })).rejects.toMatchObject({
			detail: WORKBOOK_MISSING_MESSAGE
		});
	});

	it('opens a month with no mapped days read-only, with zero writes', async () => {
		const { classId } = await seedClass();
		const fixture = await loadTemplate();
		await insertMonthTemplate({ classId, sourcePath: fixture.path });
		const before = await fixture.fileSystem.readFile(fixture.path);

		const messages: string[] = [];
		await expect(
			syncAndOpenSf2Workbook({
				classId,
				reportMonth: MONTH,
				progress: (update) => messages.push(update.message)
			})
		).resolves.toBe(fixture.path);

		expect(messages).toContain(`Opening read-only: ${NO_MAPPED_DATES}`);
		expect(await fixture.fileSystem.readFile(fixture.path)).toEqual(before);
		const stored = await db().queryOne<{ last_synced_at: number | null }>(
			'SELECT last_synced_at FROM sf2_month_templates WHERE active_class_id = ?',
			[classId]
		);
		expect(stored?.last_synced_at ?? null).toBeNull();
	});
});

describe('presentAllPreviewAttendance', () => {
	it('deletes the month’s absences and reports how many it cleared', async () => {
		const { classId, firstId } = await seedClass();
		const fixture = await loadTemplate();
		await seedMonth({ classId, path: fixture.path });
		for (const day of SEEDED_DAYS.map((entry) => entry.date)) {
			await setAttendanceEventForDay({
				studentId: firstId,
				classId,
				date: day,
				dayStart: '07:30',
				eventType: 'absent',
				reason: 'test'
			});
		}

		expect(await presentAllPreviewAttendance({ classId, reportMonth: MONTH })).toBe(2);
		expect(await hasAbsentEventForDay(firstId, classId, DAY_ONE)).toBe(false);
	});

	it('leaves an absence on a day the month does not map alone', async () => {
		const { classId, firstId } = await seedClass();
		const fixture = await loadTemplate();
		await seedMonth({ classId, path: fixture.path });
		await setAttendanceEventForDay({
			studentId: firstId,
			classId,
			date: '2026-09-30',
			dayStart: '07:30',
			eventType: 'absent',
			reason: 'test'
		});

		expect(await presentAllPreviewAttendance({ classId, reportMonth: MONTH })).toBe(0);
		expect(await hasAbsentEventForDay(firstId, classId, '2026-09-30')).toBe(true);
	});

	it('leaves a `present` record alone: it is the default, not an absence to clear', async () => {
		const { classId, firstId } = await seedClass();
		const fixture = await loadTemplate();
		await seedMonth({ classId, path: fixture.path });
		await setAttendanceEventForDay({
			studentId: firstId,
			classId,
			date: DAY_ONE,
			dayStart: '07:30',
			eventType: 'in',
			reason: 'test'
		});

		expect(await presentAllPreviewAttendance({ classId, reportMonth: MONTH })).toBe(0);
		expect(await hasPresentEventForDay(firstId, classId, DAY_ONE)).toBe(true);
	});
});

describe('importAbsentMarksFromWorkbook', () => {
	it('records the workbook X the database is missing, and reports the counts', async () => {
		const { classId } = await seedClass();
		const fixture = await loadTemplate();
		await writeAttendanceToWorkbook({
			target: {
				sourcePath: fixture.path,
				sheetName: SHEET,
				roster: [rosterOf()],
				dates: SEEDED_DAYS
			},
			absentIdsFor: () => new Set(['s1'])
		});
		await seedMonth({ classId, path: fixture.path });

		const outcome = await importAbsentMarksFromWorkbook({ classId, reportMonth: MONTH });
		expect(outcome.imported).toBeGreaterThanOrEqual(1);
		expect(outcome.scannedCells).toBeGreaterThan(0);
		expect(outcome.datesWithMarks).toBeGreaterThanOrEqual(1);
	});

	it('is a no-op the second time', async () => {
		const { classId } = await seedClass();
		const fixture = await loadTemplate();
		await writeAttendanceToWorkbook({
			target: {
				sourcePath: fixture.path,
				sheetName: SHEET,
				roster: [rosterOf()],
				dates: SEEDED_DAYS
			},
			absentIdsFor: () => new Set(['s1'])
		});
		await seedMonth({ classId, path: fixture.path });

		await importAbsentMarksFromWorkbook({ classId, reportMonth: MONTH });
		const second = await importAbsentMarksFromWorkbook({ classId, reportMonth: MONTH });
		expect(second.imported).toBe(0);
		expect(second.alreadyRecorded).toBeGreaterThanOrEqual(1);
	});
});

/** The one mapped learner the import tests write into the workbook. */
function rosterOf() {
	return {
		templateId: 'month-1',
		studentId: 's1',
		workbookName: 'LEARNER 8',
		normalizedName: 'LEARNER 8',
		rowIndex: 8,
		genderBlock: 'MALE'
	};
}
