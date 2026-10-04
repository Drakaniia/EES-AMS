import { describe, expect, it } from 'vitest';
import {
	getSf2ExportPreview,
	getSf2ExportReadiness,
	getSf2WorkbookSettings,
	killAllExcelProcesses,
	toggleSf2PreviewAttendance
} from '$lib/api/sf2';
import { addEvent } from '$lib/api/events';
import {
	CLASS_ID,
	currentMonth,
	db,
	insertMonthTemplate,
	seedMonth,
	seedRoster,
	seedSettings,
	useApiFixture
} from './fixture';

useApiFixture();

async function seededMonth(): Promise<string[]> {
	await seedRoster();
	await seedSettings();
	const days = await seedMonth();
	// 08:30 on the first mapped day: after the class day start, so the event is a
	// real check-in and not an out-of-hours write.
	await db().execute(
		`INSERT INTO events (id, student_id, class_id, event_type, timestamp, session_key)
		 VALUES ('e1', 's1', ?, 'absent', ?, ?)`,
		[CLASS_ID, localSeconds(days[0] ?? ''), `${days[0] ?? ''}|${CLASS_ID}|day`]
	);
	return days;
}

function localSeconds(date: string): number {
	return Math.floor(new Date(`${date}T08:30:00`).getTime() / 1000);
}

describe('getSf2ExportPreview', () => {
	it('draws the month grid: one absent cell, and everything else claims nothing', async () => {
		const days = await seededMonth();

		const preview = await getSf2ExportPreview(CLASS_ID);

		expect(preview.classId).toBe(CLASS_ID);
		expect(preview.className).toBe('Grade 3 - Matapat');
		expect(preview.canExport).toBe(true);
		expect(preview.issues).toEqual([]);
		expect(preview.absenceCount).toBe(1);
		expect(preview.absentList).toEqual([
			{
				studentId: 's1',
				studentName: 'Dela Cruz, Juan',
				date: days[0],
				rowIndex: 8
			}
		]);

		const juan = preview.students.find((row) => row.studentId === 's1');
		expect(juan?.cells.find((cell) => cell.date === days[0])?.status).toBe('absent');
		// The row the teacher has not opened a day for stays `open`, not `present`.
		const untouched = juan?.cells.find((cell) => cell.date === days[4]);
		expect(untouched?.status).toBe('open');
	});
});

describe('getSf2ExportReadiness', () => {
	it('counts what is mapped and answers whether the month may be exported', async () => {
		await seededMonth();

		const readiness = await getSf2ExportReadiness(CLASS_ID);

		expect(readiness.canExport).toBe(true);
		expect(readiness.mappedStudents).toBe(3);
		expect(readiness.mappedDates).toBe(5);
		expect(readiness.template).toMatchObject({
			classId: CLASS_ID,
			reportMonth: expect.any(String)
		});
	});

	it('refuses a month that is not set up, in the words the sidebar already shows', async () => {
		await seedRoster();
		await seedSettings();
		await insertMonthTemplate({ classId: 'class-other', sourcePath: '/nope.xls' });

		const readiness = await getSf2ExportReadiness('class-other');

		expect(readiness.canExport).toBe(false);
		expect(readiness.mappedStudents).toBe(0);
		expect(readiness.issues).toEqual([
			`No SF2 workbook is stored for ${currentMonth()} yet. Create it to switch to this month.`,
			'No attendance days are mapped to this month yet.',
			"No learners are mapped to this month's SF2 workbook yet."
		]);
	});
});

describe('getSf2WorkbookSettings', () => {
	it('reads the month row, and leaves the row-0 placeholder off the learner list', async () => {
		await seededMonth();

		const settings = await getSf2WorkbookSettings(CLASS_ID);

		expect(settings).toMatchObject({
			templateId: 'month-1',
			classId: CLASS_ID,
			className: 'Grade 3 - Matapat',
			datesMapped: 5
		});
		// `Bautista, Ana` is mapped to row 0, which is the "no workbook row" slot.
		expect(settings.learnerNames).toEqual(['Dela Cruz, Juan', 'Reyes, Maria']);
	});

	it('rejects when the class has no month set up yet', async () => {
		await seedRoster();
		await seedSettings();

		await expect(getSf2WorkbookSettings(CLASS_ID)).rejects.toMatchObject({
			kind: 'InvalidInput'
		});
	});
});

describe('toggleSf2PreviewAttendance', () => {
	it('writes the absent mark and the grid then shows it', async () => {
		const days = await seededMonth();

		await toggleSf2PreviewAttendance(CLASS_ID, 's2', days[0] ?? '', false);

		const preview = await getSf2ExportPreview(CLASS_ID);
		expect(preview.absenceCount).toBe(2);
		const maria = preview.students.find((row) => row.studentId === 's2');
		expect(maria?.cells.find((cell) => cell.date === days[0])?.status).toBe('absent');
	});

	it('marks a learner present again by removing the absence', async () => {
		const days = await seededMonth();

		await toggleSf2PreviewAttendance(CLASS_ID, 's1', days[0] ?? '', true);

		expect((await getSf2ExportPreview(CLASS_ID)).absenceCount).toBe(0);
	});

	it('refuses a student who is not on the class roster', async () => {
		const days = await seededMonth();

		await expect(
			toggleSf2PreviewAttendance(CLASS_ID, 'not-a-student', days[0] ?? '', false)
		).rejects.toMatchObject({ kind: 'InvalidInput' });
	});
});

describe('killAllExcelProcesses', () => {
	it('rejects: D14 means there is no Excel process to kill', async () => {
		await expect(killAllExcelProcesses()).rejects.toMatchObject({
			kind: 'InvalidInput',
			detail: 'Excel is no longer driven by this app'
		});
	});
});

describe('addEvent against a seeded month', () => {
	it('rejects a duplicate session even when the mark came from the grid', async () => {
		const days = await seededMonth();

		await expect(
			addEvent({
				studentId: 's1',
				classId: CLASS_ID,
				type: 'absent',
				timestamp: `${days[0]}T09:00:00`
			})
		).rejects.toMatchObject({ kind: 'DuplicateAttendance' });
	});
});
