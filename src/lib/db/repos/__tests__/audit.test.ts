import { describe, expect, it } from 'vitest';
import {
	clearAuditEvents,
	listAllAuditEvents,
	listAuditEvents,
	recordAuditEvent
} from '$lib/db/repos/audit';
import { db, useTestDb } from './schema';

useTestDb();

const payload = JSON.stringify({ name: 'Ana' });

describe('recordAuditEvent', () => {
	it('writes the row as admin and returns it', async () => {
		const event = await recordAuditEvent({
			entityType: 'student',
			entityId: 's1',
			action: 'create',
			summary: 'Created student Ana',
			afterJson: payload
		});

		expect(event.actor).toBe('admin');
		expect(new Date(event.createdAt).toISOString()).toBe(event.createdAt);

		const stored = await db().query<Record<string, unknown>>('SELECT * FROM audit_events');
		expect(stored).toHaveLength(1);
		expect(stored[0]).toMatchObject({
			id: event.id,
			entity_type: 'student',
			entity_id: 's1',
			before_json: null,
			after_json: payload
		});
	});
});

describe('listAuditEvents', () => {
	/**
	 * Written straight to the table with distinct seconds: three rows written in
	 * the same second tie on `created_at` and fall back to a random UUID, which
	 * is the Rust behaviour and not what an ordering test can assert on.
	 */
	const seed = async () => {
		for (const [id, summary, createdAt] of [
			['a1', 'first', 1000],
			['a2', 'second', 2000],
			['a3', 'third', 3000]
		]) {
			await db().execute(
				`INSERT INTO audit_events (id, entity_type, action, summary, created_at, actor)
				 VALUES (?, ?, ?, ?, ?, 'admin')`,
				[
					id,
					id === 'a1' ? 'student' : 'class',
					id === 'a3' ? 'update' : 'create',
					summary,
					createdAt
				]
			);
		}
	};

	it('returns newest first and clamps the limit', async () => {
		await seed();

		expect((await listAuditEvents()).map((e) => e.summary)).toEqual(['third', 'second', 'first']);
		expect((await listAuditEvents(2)).map((e) => e.summary)).toEqual(['third', 'second']);
		expect(await listAuditEvents(0)).toHaveLength(1);
		expect(await listAuditEvents(9999)).toHaveLength(3);
	});

	it('reads the epoch column back as the ISO string the UI expects', async () => {
		await seed();

		expect((await listAuditEvents())[0].createdAt).toBe(new Date(3000 * 1000).toISOString());
	});

	it('returns oldest first with no limit', async () => {
		await seed();

		expect((await listAllAuditEvents()).map((e) => e.summary)).toEqual([
			'first',
			'second',
			'third'
		]);
	});
});

describe('clearAuditEvents', () => {
	it('reports how many rows it removed', async () => {
		await recordAuditEvent({ entityType: 'student', action: 'create', summary: 'a' });
		await recordAuditEvent({ entityType: 'student', action: 'create', summary: 'b' });

		expect(await clearAuditEvents()).toBe(2);
		expect(await listAuditEvents()).toHaveLength(0);
	});
});
