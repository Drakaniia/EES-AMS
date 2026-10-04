import { describe, expect, it } from 'vitest';
import {
	addEvent,
	addEvents,
	deleteEvents,
	lastEventForStudent,
	listAttendanceAudit,
	listEvents,
	listEventsForDate,
	listEventsForStudent,
	updateEvent
} from '$lib/api/events';
import { CLASS_ID, db, seedRoster, useApiFixture } from './fixture';

useApiFixture();

/** Local 09:00 on a fixed day, because every date rule here is a local-calendar rule. */
const morning = (day: number) => new Date(2026, 9, day, 9, 0, 0).toISOString();

const sessionKeyFor = (day: number) => `2026-10-${String(day).padStart(2, '0')}|class-1|day`;

describe('addEvent', () => {
	it('derives the session key from the local date and class', async () => {
		await seedRoster();

		const event = await addEvent({
			studentId: 's1',
			classId: CLASS_ID,
			type: 'in',
			timestamp: morning(2)
		});

		expect(event).toMatchObject({
			studentId: 's1',
			classId: CLASS_ID,
			type: 'in',
			sessionKey: sessionKeyFor(2)
		});
		expect(event.updatedAt).toBeUndefined();
	});

	it('rejects a second record for the same student, class and day', async () => {
		await seedRoster();
		await addEvent({ studentId: 's1', classId: CLASS_ID, type: 'in', timestamp: morning(2) });

		await expect(
			addEvent({ studentId: 's1', classId: CLASS_ID, type: 'absent', timestamp: morning(2) })
		).rejects.toMatchObject({ kind: 'DuplicateAttendance' });
	});
});

describe('addEvents', () => {
	it('skips a duplicate rather than failing the whole batch', async () => {
		await seedRoster();
		await addEvent({ studentId: 's1', classId: CLASS_ID, type: 'in', timestamp: morning(2) });

		const events = await addEvents([
			{ studentId: 's1', classId: CLASS_ID, type: 'in', timestamp: morning(2) },
			{ studentId: 's2', classId: CLASS_ID, type: 'absent', timestamp: morning(2) }
		]);

		expect(events).toHaveLength(1);
		expect(events[0]?.studentId).toBe('s2');
		expect(await listEvents()).toHaveLength(2);
	});
});

describe('listEventsForDate', () => {
	it('returns only the events on that local day', async () => {
		await seedRoster();
		await addEvent({ studentId: 's1', classId: CLASS_ID, type: 'in', timestamp: morning(2) });
		await addEvent({ studentId: 's2', classId: CLASS_ID, type: 'in', timestamp: morning(3) });

		const day = await listEventsForDate('2026-10-02');

		expect(day.map((event) => event.studentId)).toEqual(['s1']);
	});

	it('rejects a date that is not a calendar day', async () => {
		await expect(listEventsForDate('2026-02-30')).rejects.toMatchObject({
			kind: 'InvalidInput'
		});
	});
});

describe('lastEventForStudent', () => {
	it('returns the newest event, and undefined for a student with none', async () => {
		await seedRoster();
		await addEvent({ studentId: 's1', classId: CLASS_ID, type: 'in', timestamp: morning(2) });
		await addEvent({ studentId: 's1', classId: CLASS_ID, type: 'in', timestamp: morning(5) });

		expect((await lastEventForStudent('s1'))?.timestamp).toBe(morning(5));
		expect(await lastEventForStudent('s2')).toBeUndefined();
		expect(await listEventsForStudent('s1')).toHaveLength(2);
	});
});

describe('updateEvent', () => {
	it('stamps the override reason, the edit time and an attendance audit entry', async () => {
		await seedRoster();
		const created = await addEvent({
			studentId: 's1',
			classId: CLASS_ID,
			type: 'in',
			timestamp: morning(2)
		});

		const updated = await updateEvent(created.id, { note: 'Late bus', reason: 'Corrected' });

		expect(updated.note).toBe('Late bus');
		expect(updated.overrideReason).toBe('Corrected');
		expect(updated.updatedAt).toBeDefined();

		const audit = await listAttendanceAudit({ eventId: created.id });
		expect(audit).toHaveLength(1);
		expect(audit[0]).toMatchObject({ action: 'update', reason: 'Corrected' });
	});
});

describe('deleteEvents', () => {
	it('removes the listed rows, ignores ids it does not know, and audits the removals', async () => {
		await seedRoster();
		const first = await addEvent({
			studentId: 's1',
			classId: CLASS_ID,
			type: 'in',
			timestamp: morning(2)
		});
		await addEvent({ studentId: 's2', classId: CLASS_ID, type: 'in', timestamp: morning(2) });

		await deleteEvents([first.id, 'not-a-real-id'], 'Cleared');

		expect(await listEvents()).toHaveLength(1);
		const audit = await listAttendanceAudit({ studentId: 's1' });
		expect(audit).toHaveLength(1);
		expect(audit[0]).toMatchObject({ action: 'delete', reason: 'Cleared' });
	});
});

describe('listAttendanceAudit', () => {
	it('treats eventId and studentId as alternatives, not a conjunction', async () => {
		await seedRoster();
		const forJuan = await addEvent({
			studentId: 's1',
			classId: CLASS_ID,
			type: 'in',
			timestamp: morning(2)
		});
		await updateEvent(forJuan.id, { reason: 'Fixed' });

		const both = await listAttendanceAudit({
			eventId: forJuan.id,
			studentId: 's2'
		});

		expect(both.map((entry) => entry.id)).toHaveLength(1);
		const rows = await db().query<{ total: number }>('SELECT COUNT(*) AS total FROM audit_events');
		expect(Number(rows[0]?.total)).toBeGreaterThan(0);
	});
});
