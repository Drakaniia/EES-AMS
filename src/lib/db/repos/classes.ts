import { appError, getDriver } from '$lib/db';
import type { Class, CreateClassRequest, Session, UpdateClassRequest } from '$lib/domain/models';
import { epochSecondsToIso, normalizeOptionalText, nowEpochSeconds } from '$lib/domain/models';
import { recordAuditEvent } from './audit';

/**
 * Class repository — the port of
 * `src-tauri/src/infrastructure/database/classes.rs`.
 *
 * `sessions` and `days` are stored as JSON text. A class written before those
 * columns existed has no text in them, and a school day list is never empty in
 * practice, so an unreadable `days` falls back to Monday..Friday and an
 * unreadable `sessions` falls back to none — the same defaults the Rust row
 * reader used, so an old row reads back identically after the port.
 */

interface ClassRow {
	id: string;
	name: string;
	room: string | null;
	day_start: string;
	day_end: string;
	late_after: string;
	created_at: number;
	sessions: string | null;
	days: string | null;
}

const CLASS_COLUMNS = 'id, name, room, day_start, day_end, late_after, created_at, sessions, days';

const WEEKDAYS = [1, 2, 3, 4, 5];

function parseJson<T>(raw: string | null, fallback: T): T {
	if (raw === null) return fallback;
	try {
		return JSON.parse(raw) as T;
	} catch {
		return fallback;
	}
}

function toClass(row: ClassRow): Class {
	return {
		id: row.id,
		name: row.name,
		room: row.room && row.room.length > 0 ? row.room : undefined,
		dayStart: row.day_start,
		dayEnd: row.day_end,
		lateAfter: row.late_after,
		sessions: parseJson<Session[]>(row.sessions, []),
		days: parseJson<number[]>(row.days, WEEKDAYS),
		createdAt: epochSecondsToIso(row.created_at)
	};
}

async function countRows(sql: string, param: string): Promise<number> {
	const row = await getDriver().queryOne<{ count: number }>(sql, [param]);
	return Number(row?.count ?? 0);
}

export async function listClasses(): Promise<Class[]> {
	const rows = await getDriver().query<ClassRow>(
		`SELECT ${CLASS_COLUMNS} FROM classes ORDER BY name ASC`
	);
	return rows.map(toClass);
}

export async function getClass(id: string): Promise<Class | undefined> {
	const row = await getDriver().queryOne<ClassRow>(
		`SELECT ${CLASS_COLUMNS} FROM classes WHERE id = ?`,
		[id]
	);
	return row === undefined ? undefined : toClass(row);
}

async function requireClass(id: string): Promise<Class> {
	const existing = await getClass(id);
	if (existing === undefined) throw appError('ClassNotFound', id);
	return existing;
}

export async function createClass(req: CreateClassRequest): Promise<Class> {
	const createdAt = nowEpochSeconds();
	const cls: Class = {
		id: crypto.randomUUID(),
		name: req.name,
		room: normalizeOptionalText(req.room),
		dayStart: req.dayStart,
		dayEnd: req.dayEnd,
		lateAfter: req.lateAfter,
		// `#[serde(default)]` on the Rust request: an absent list is an empty
		// list, never a missing field the UI has to guard for.
		sessions: req.sessions ?? [],
		days: req.days ?? [],
		createdAt: epochSecondsToIso(createdAt)
	};

	await getDriver().transaction(async () => {
		await getDriver().execute(
			`INSERT INTO classes (id, name, room, day_start, day_end, late_after, sessions, days, created_at)
			 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			[
				cls.id,
				cls.name,
				cls.room ?? null,
				cls.dayStart,
				cls.dayEnd,
				cls.lateAfter,
				JSON.stringify(cls.sessions),
				JSON.stringify(cls.days),
				createdAt
			]
		);
		await recordAuditEvent({
			entityType: 'class',
			entityId: cls.id,
			action: 'create',
			summary: `Created class ${cls.name}`,
			afterJson: JSON.stringify(cls)
		});
	});

	return cls;
}

export async function updateClass(id: string, req: UpdateClassRequest): Promise<Class> {
	// `room` keeps Rust's three-state rule: absent leaves it, blank clears it.
	const hasRoom = req.room !== undefined;
	const before = await requireClass(id);
	const cls: Class = { ...before };
	if (req.name !== undefined) cls.name = req.name;
	if (hasRoom) cls.room = normalizeOptionalText(req.room);
	if (req.dayStart !== undefined) cls.dayStart = req.dayStart;
	if (req.dayEnd !== undefined) cls.dayEnd = req.dayEnd;
	if (req.lateAfter !== undefined) cls.lateAfter = req.lateAfter;
	if (req.sessions !== undefined) cls.sessions = req.sessions;
	if (req.days !== undefined) cls.days = req.days;

	await getDriver().transaction(async () => {
		await getDriver().execute(
			`UPDATE classes
			 SET name = ?, room = ?, day_start = ?, day_end = ?, late_after = ?, sessions = ?, days = ?
			 WHERE id = ?`,
			[
				cls.name,
				cls.room ?? null,
				cls.dayStart,
				cls.dayEnd,
				cls.lateAfter,
				JSON.stringify(cls.sessions),
				JSON.stringify(cls.days),
				id
			]
		);
		await recordAuditEvent({
			entityType: 'class',
			entityId: cls.id,
			action: 'update',
			summary: `Updated class ${cls.name}`,
			beforeJson: JSON.stringify(before),
			afterJson: JSON.stringify(cls)
		});
	});

	return cls;
}

export async function deleteClass(id: string): Promise<void> {
	const before = await requireClass(id);
	await getDriver().transaction(async () => {
		const affectedStudents = await countRows(
			'SELECT COUNT(*) AS count FROM students WHERE class_id = ?',
			id
		);
		const affectedEvents = await countRows(
			'SELECT COUNT(*) AS count FROM events WHERE class_id = ?',
			id
		);
		const deletedSf2Templates = await countRows(
			'SELECT COUNT(*) AS count FROM sf2_templates WHERE active_class_id = ?',
			id
		);

		await getDriver().execute('DELETE FROM sf2_templates WHERE active_class_id = ?', [id]);
		await getDriver().execute('DELETE FROM attendance_day_status WHERE class_id = ?', [id]);
		// Events and students survive their class: the marks are the teacher's
		// record and outlive the roster grouping, so the class is detached
		// rather than cascading.
		await getDriver().execute('UPDATE events SET class_id = NULL WHERE class_id = ?', [id]);
		await getDriver().execute('UPDATE students SET class_id = NULL WHERE class_id = ?', [id]);

		const rows = await getDriver().execute('DELETE FROM classes WHERE id = ?', [id]);
		if (rows === 0) throw appError('ClassNotFound', id);

		await recordAuditEvent({
			entityType: 'class',
			entityId: before.id,
			action: 'delete',
			summary: `Deleted class ${before.name}`,
			beforeJson: JSON.stringify(before),
			metadataJson: JSON.stringify({ affectedStudents, affectedEvents, deletedSf2Templates })
		});
	});
}
