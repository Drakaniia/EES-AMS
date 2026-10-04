import { describe, expect, it } from 'vitest';
import {
	addEvent,
	addEvents,
	deleteEvent,
	deleteEvents,
	getEvent,
	lastEventForStudent,
	listAttendanceAudit,
	listEvents,
	listEventsForClassAndDateRange,
	listEventsForDate,
	listEventsForStudent,
	updateEvent
} from '$lib/db/repos/events';
import { db, useTestDb } from './schema';

useTestDb();

const STUDENT = '11111111-1111-4111-8111-111111111111';
const CLASS_ID = 'c1';

/** Local 09:00, because every date rule in this repo is a local-calendar rule. */
const localMorning = (year: number, month: number, day: number) =>
	new Date(year, month - 1, day, 9, 0, 0).toISOString();

const sessionKeyFor = (date: string, classId: string) => `${date}|${classId}|day`;

describe('addEvent', () => {
	it('derives the session key from the local date and class', async () => {
		const event = await addEvent({
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'in',
			timestamp: localMorning(2026, 10, 2)
		});

		expect(event.sessionKey).toBe(sessionKeyFor('2026-10-02', CLASS_ID));
		expect(event.updatedAt).toBeUndefined();
		expect(event.overrideReason).toBeUndefined();
		expect(await getEvent(event.id)).toEqual(event);
	});

	it('keeps a caller-supplied session key and unassigned classes apart', async () => {
		const event = await addEvent({
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'absent',
			note: '  sick  ',
			sessionKey: 'custom|key'
		});

		expect(event.sessionKey).toBe('custom|key');
		expect(event.note).toBe('sick');
	});

	it('falls back to "unassigned" when there is no class', async () => {
		const event = await addEvent({
			studentId: STUDENT,
			type: 'in',
			timestamp: localMorning(2026, 10, 2)
		});

		expect(event.sessionKey).toBe(sessionKeyFor('2026-10-02', 'unassigned'));
	});

	it('rejects a second mark for the same session', async () => {
		const req = {
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'in' as const,
			timestamp: localMorning(2026, 10, 2)
		};
		await addEvent(req);

		await expect(addEvent({ ...req, type: 'absent' })).rejects.toMatchObject({
			kind: 'DuplicateAttendance',
			detail: 'Student already recorded for this session'
		});
		expect(await listEvents()).toHaveLength(1);
	});

	it('allows an override to supersede the mark and audits it', async () => {
		const req = {
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'in' as const,
			timestamp: localMorning(2026, 10, 2)
		};
		await addEvent(req);
		const override = await addEvent({ ...req, type: 'absent', overrideReason: 'corrected' });

		expect(override.overrideReason).toBe('corrected');
		const audit = await listAttendanceAudit({ eventId: override.id });
		expect(audit).toHaveLength(1);
		expect(audit[0]).toMatchObject({
			eventId: override.id,
			studentId: STUDENT,
			action: 'create_override',
			reason: 'corrected',
			actor: 'admin'
		});
		expect(JSON.parse(audit[0].afterJson ?? '{}')).toMatchObject({ type: 'absent' });
	});
});

describe('addEvents', () => {
	it('skips rows that are already recorded instead of failing the batch', async () => {
		const req = {
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'in' as const,
			timestamp: localMorning(2026, 10, 2)
		};
		await addEvent(req);

		const events = await addEvents([
			req,
			{ ...req, studentId: '22222222-2222-4222-8222-222222222222' },
			{ ...req, type: 'absent', timestamp: localMorning(2026, 10, 3) }
		]);

		expect(events).toHaveLength(2);
		expect(await listEvents()).toHaveLength(3);
	});
});

describe('listEvents', () => {
	const seed = async () => {
		await addEvent({
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'in',
			timestamp: localMorning(2026, 10, 1)
		});
		await addEvent({
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'absent',
			timestamp: localMorning(2026, 10, 3)
		});
		await addEvent({
			studentId: STUDENT,
			classId: 'c2',
			type: 'in',
			timestamp: localMorning(2026, 10, 2)
		});
	};

	it('returns everything newest first', async () => {
		await seed();

		expect((await listEvents()).map((e) => e.timestamp)).toEqual([
			localMorning(2026, 10, 3),
			localMorning(2026, 10, 2),
			localMorning(2026, 10, 1)
		]);
	});

	it('lists one local calendar day', async () => {
		await seed();

		expect(await listEventsForDate('2026-10-02')).toHaveLength(1);
		expect(await listEventsForDate('2026-10-04')).toHaveLength(0);
	});

	it('rejects a date that is not YYYY-MM-DD', async () => {
		await expect(listEventsForDate('02-10-2026')).rejects.toMatchObject({
			kind: 'InvalidInput',
			detail: 'invalid attendance date: expected YYYY-MM-DD'
		});
		await expect(listEventsForDate('2026-02-30')).rejects.toMatchObject({ kind: 'InvalidInput' });
	});

	it('covers both ends of a class date range', async () => {
		await seed();

		const range = await listEventsForClassAndDateRange(CLASS_ID, '2026-10-01', '2026-10-03');

		expect(range.map((e) => e.type)).toEqual(['absent', 'in']);
	});

	it('filters by student and finds their latest mark', async () => {
		await seed();

		expect(await listEventsForStudent('nobody')).toHaveLength(0);
		expect(await lastEventForStudent('nobody')).toBeUndefined();
		expect(await listEventsForStudent(STUDENT)).toHaveLength(3);
		expect((await lastEventForStudent(STUDENT))?.type).toBe('absent');
	});
});

describe('getEvent', () => {
	it('throws EventNotFound for an unknown id', async () => {
		await expect(getEvent('missing')).rejects.toMatchObject({
			kind: 'EventNotFound',
			detail: 'missing'
		});
	});
});

describe('updateEvent', () => {
	const seed = async () =>
		await addEvent({
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'absent',
			note: 'sick',
			timestamp: localMorning(2026, 10, 2)
		});

	it('requires a reason', async () => {
		const event = await seed();

		await expect(updateEvent(event.id, { reason: '   ' })).rejects.toMatchObject({
			kind: 'InvalidInput',
			detail: 'audit reason is required'
		});
	});

	it('keeps the session key when only the note and reason changed', async () => {
		const event = await seed();

		const updated = await updateEvent(event.id, { note: 'returned', reason: 'typo' });

		expect(updated.sessionKey).toBe(event.sessionKey);
		expect(updated.note).toBe('returned');
		expect(updated.type).toBe('absent');
		expect(updated.updatedAt).toBeDefined();

		const audit = await listAttendanceAudit({ eventId: event.id });
		expect(audit[0]).toMatchObject({ action: 'update', reason: 'typo' });
		expect(JSON.parse(audit[0].beforeJson ?? '{}')).toMatchObject({ note: 'sick' });
		expect(JSON.parse(audit[0].afterJson ?? '{}')).toMatchObject({ note: 'returned' });
	});

	it('re-derives the session key when the timestamp moves', async () => {
		const event = await seed();

		const updated = await updateEvent(event.id, {
			timestamp: localMorning(2026, 10, 3),
			reason: 'wrong day'
		});

		expect(updated.sessionKey).toBe(sessionKeyFor('2026-10-03', CLASS_ID));
	});

	it('re-derives the session key when the class moves', async () => {
		const event = await seed();

		const updated = await updateEvent(event.id, { classId: 'c2', reason: 'reassigned' });

		expect(updated.sessionKey).toBe(sessionKeyFor('2026-10-02', 'c2'));
	});

	it('throws EventNotFound for an unknown id', async () => {
		await expect(updateEvent('missing', { reason: 'x' })).rejects.toMatchObject({
			kind: 'EventNotFound'
		});
	});
});

describe('deleteEvent', () => {
	it('removes the row and records both audit trails when given a reason', async () => {
		const event = await addEvent({
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'in',
			timestamp: localMorning(2026, 10, 2)
		});

		await deleteEvent(event.id, 'card read twice');

		expect(await listEvents()).toHaveLength(0);
		const audit = await listAttendanceAudit({ eventId: event.id });
		expect(audit).toHaveLength(1);
		expect(audit[0]).toMatchObject({
			action: 'delete',
			reason: 'card read twice',
			beforeJson: expect.stringContaining(event.id),
			afterJson: undefined
		});
		const trail = await db().query<{ summary: string; metadata_json: string }>(
			"SELECT summary, metadata_json FROM audit_events WHERE entity_type = 'attendance_event' AND action = 'delete'"
		);
		expect(trail).toHaveLength(1);
		expect(JSON.parse(trail[0].metadata_json)).toEqual({ reason: 'card read twice' });
	});

	it('still records the general trail when no reason is given', async () => {
		const event = await addEvent({
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'in',
			timestamp: localMorning(2026, 10, 2)
		});

		await deleteEvent(event.id);

		expect(await listAttendanceAudit({ eventId: event.id })).toHaveLength(0);
		const trail = await db().query<{ metadata_json: string }>(
			"SELECT metadata_json FROM audit_events WHERE action = 'delete'"
		);
		expect(JSON.parse(trail[0].metadata_json)).toEqual({ reason: null });
	});

	it('throws EventNotFound for an unknown id', async () => {
		await expect(deleteEvent('missing')).rejects.toMatchObject({ kind: 'EventNotFound' });
	});
});

describe('deleteEvents', () => {
	it('deletes what is there, skips what is not, and audits each deletion', async () => {
		const keep = await addEvent({
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'in',
			timestamp: localMorning(2026, 10, 1)
		});
		const drop = await addEvent({
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'absent',
			timestamp: localMorning(2026, 10, 2)
		});

		await deleteEvents([drop.id, 'missing'], 'clearing the month');

		expect((await listEvents()).map((e) => e.id)).toEqual([keep.id]);
		expect(await listAttendanceAudit({ eventId: drop.id })).toHaveLength(1);
		const trail = await db().query<{ entity_id: string }>(
			"SELECT entity_id FROM audit_events WHERE entity_type = 'attendance_event' AND action = 'delete'"
		);
		expect(trail.map((row) => row.entity_id)).toEqual([drop.id]);
	});
});

describe('listAttendanceAudit', () => {
	const seed = async () => {
		const first = await addEvent({
			studentId: STUDENT,
			classId: CLASS_ID,
			type: 'in',
			timestamp: localMorning(2026, 10, 2)
		});
		const other = await addEvent({
			studentId: '22222222-2222-4222-8222-222222222222',
			classId: CLASS_ID,
			type: 'in',
			timestamp: localMorning(2026, 10, 2)
		});
		await updateEvent(first.id, { note: 'late', reason: 'typo' });
		await updateEvent(other.id, { note: 'late', reason: 'typo' });
		return { first, other };
	};

	it('filters by event, by student, or by either when both are given', async () => {
		const { first, other } = await seed();

		expect(await listAttendanceAudit({ eventId: first.id })).toHaveLength(1);
		expect(await listAttendanceAudit({ studentId: STUDENT })).toHaveLength(1);
		expect(
			await listAttendanceAudit({ eventId: first.id, studentId: other.studentId })
		).toHaveLength(2);
	});

	it('returns the whole trail when given no filters', async () => {
		await seed();

		const all = await listAttendanceAudit();
		expect(all).toHaveLength(2);
		expect(all[0]).toMatchObject({
			action: 'update',
			actor: 'admin',
			studentId: expect.any(String)
		});
	});
});
