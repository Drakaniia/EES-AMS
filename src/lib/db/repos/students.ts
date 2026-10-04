import { appError, getDriver } from '$lib/db';
import type { CreateStudentRequest, StudentRecord, UpdateStudentRequest } from '$lib/domain/models';
import {
	epochSecondsToIso,
	isoToEpochSeconds,
	normalizeOptionalText,
	nowEpochSeconds,
	studentGenderFromDb
} from '$lib/domain/models';
import type { Student } from '$lib/types';
import { recordAuditEvent } from './audit';

/**
 * Student repository — the port of
 * `src-tauri/src/infrastructure/database/students.rs`.
 *
 * The card serial is the only unique, user-supplied identifier on this table,
 * so every write path checks it against the table — and against the rest of its
 * own batch — before it inserts. That check is what makes `card already
 * registered` a specific message the Students page can show, instead of a UNIQUE
 * constraint blowing up halfway through a transaction.
 */

interface StudentRow {
	id: string;
	name: string;
	gender: string | null;
	card_serial: string | null;
	class_id: string | null;
	sf2_learner_id: string | null;
	created_at: number;
}

const STUDENT_COLUMNS = 'id, name, gender, card_serial, class_id, created_at, sf2_learner_id';

function toStudent(row: StudentRow): StudentRecord {
	return {
		id: row.id,
		name: row.name,
		gender: studentGenderFromDb(row.gender),
		cardSerial: row.card_serial ?? undefined,
		classId: row.class_id ?? undefined,
		sf2LearnerId: row.sf2_learner_id ?? undefined,
		createdAt: epochSecondsToIso(row.created_at)
	};
}

/**
 * The DepEd learner ID is not part of `CreateStudentRequest`: nothing the user
 * types on the Students page knows it — it arrives from the SF2 roster. A create
 * therefore starts without one, which is honest (the school has not told us
 * yet), and the INSERT below deliberately omits the column.
 */
function newStudent(
	name: string,
	gender: CreateStudentRequest['gender'],
	cardSerial: string | undefined,
	classId: string | undefined,
	createdAt: number
): StudentRecord {
	return {
		id: crypto.randomUUID(),
		name,
		gender,
		cardSerial,
		classId,
		sf2LearnerId: undefined,
		createdAt: epochSecondsToIso(createdAt)
	};
}

async function insertStudent(student: StudentRecord): Promise<void> {
	await getDriver().execute(
		`INSERT INTO students (id, name, gender, card_serial, class_id, created_at)
		 VALUES (?, ?, ?, ?, ?, ?)`,
		[
			student.id,
			student.name,
			student.gender ?? null,
			student.cardSerial ?? null,
			student.classId ?? null,
			// The record holds whole seconds, so this round-trip is exact.
			isoToEpochSeconds(student.createdAt)
		]
	);
	await recordAuditEvent({
		entityType: 'student',
		entityId: student.id,
		action: 'create',
		summary: `Created student ${student.name}`,
		afterJson: JSON.stringify(student)
	});
}

export async function findStudentByCard(serial: string): Promise<StudentRecord | undefined> {
	const row = await getDriver().queryOne<StudentRow>(
		`SELECT ${STUDENT_COLUMNS} FROM students WHERE card_serial = ?`,
		[serial]
	);
	return row === undefined ? undefined : toStudent(row);
}

/** Throws `CardAlreadyRegistered` when the serial is on file for someone else. */
async function assertCardFree(serial: string | undefined, exceptStudentId?: string): Promise<void> {
	if (serial === undefined) return;
	const existing = await findStudentByCard(serial);
	if (existing !== undefined && existing.id !== exceptStudentId) {
		throw appError('CardAlreadyRegistered', serial);
	}
}

export async function listStudents(classId?: string): Promise<StudentRecord[]> {
	const sql =
		classId === undefined
			? `SELECT ${STUDENT_COLUMNS} FROM students ORDER BY name ASC`
			: `SELECT ${STUDENT_COLUMNS} FROM students WHERE class_id = ? ORDER BY name ASC`;
	const rows = await getDriver().query<StudentRow>(sql, classId === undefined ? [] : [classId]);
	return rows.map(toStudent);
}

export async function getStudent(id: string): Promise<StudentRecord> {
	const row = await getDriver().queryOne<StudentRow>(
		`SELECT ${STUDENT_COLUMNS} FROM students WHERE id = ?`,
		[id]
	);
	if (row === undefined) throw appError('StudentNotFound', id);
	return toStudent(row);
}

export async function createStudent(req: CreateStudentRequest): Promise<StudentRecord> {
	const cardSerial = normalizeOptionalText(req.cardSerial);
	await assertCardFree(cardSerial);

	const student = newStudent(
		req.name,
		req.gender,
		cardSerial,
		normalizeOptionalText(req.classId),
		nowEpochSeconds()
	);
	await getDriver().transaction(async () => {
		await insertStudent(student);
	});
	return student;
}

/** All or nothing: one duplicate card anywhere in the batch aborts the batch. */
export async function createStudents(reqs: CreateStudentRequest[]): Promise<StudentRecord[]> {
	const normalized = reqs.map((req) => ({
		name: req.name,
		gender: req.gender,
		cardSerial: normalizeOptionalText(req.cardSerial),
		classId: normalizeOptionalText(req.classId)
	}));

	const seen = new Set<string>();
	for (const { cardSerial } of normalized) {
		if (cardSerial === undefined) continue;
		if (seen.has(cardSerial)) throw appError('CardAlreadyRegistered', cardSerial);
		seen.add(cardSerial);
		await assertCardFree(cardSerial);
	}

	// One clock read for the batch: a bulk import should read as created at one
	// moment, not a second apart per row.
	const createdAt = nowEpochSeconds();
	const students = normalized.map((entry) =>
		newStudent(entry.name, entry.gender, entry.cardSerial, entry.classId, createdAt)
	);

	return await getDriver().transaction(async () => {
		for (const student of students) await insertStudent(student);
		return students;
	});
}

export async function updateStudent(id: string, req: UpdateStudentRequest): Promise<StudentRecord> {
	// Rust's `Option<Option<String>>`: an absent key leaves the column alone, a
	// present-but-blank key CLEARS it. Collapsing both to `undefined` would turn
	// "clear the card" into "change nothing", so presence is tracked separately.
	const hasCardSerial = req.cardSerial !== undefined;
	const hasClassId = req.classId !== undefined;
	const cardSerial = normalizeOptionalText(req.cardSerial);
	const classId = normalizeOptionalText(req.classId);

	if (cardSerial !== undefined) await assertCardFree(cardSerial, id);

	const before = await getStudent(id);
	const student: StudentRecord = { ...before };
	if (req.name !== undefined) student.name = req.name;
	// Gender can only be set, never cleared — the Rust `if let Some(gender)`
	// had no clear path either.
	if (req.gender !== undefined) student.gender = req.gender;
	if (hasCardSerial) student.cardSerial = cardSerial;
	if (hasClassId) student.classId = classId;

	await getDriver().transaction(async () => {
		// `sf2_learner_id` is deliberately absent from this statement. It is not
		// part of `UpdateStudentRequest`, so naming it here would either NULL the
		// school's own record on every name change or thread it through a request
		// the Students page has no field for. The SF2 roster owns that column.
		await getDriver().execute(
			`UPDATE students
			 SET name = ?, gender = ?, card_serial = ?, class_id = ?
			 WHERE id = ?`,
			[
				student.name,
				student.gender ?? null,
				student.cardSerial ?? null,
				student.classId ?? null,
				id
			]
		);
		await recordAuditEvent({
			entityType: 'student',
			entityId: id,
			action: 'update',
			summary: `Updated student ${student.name}`,
			beforeJson: JSON.stringify(before),
			afterJson: JSON.stringify(student)
		});
	});

	return student;
}

/** The `saveStudent` upsert the UI already calls: an id means update, no id means create. */
export async function saveStudent(student: Student): Promise<StudentRecord> {
	const req: CreateStudentRequest = {
		name: student.name,
		gender: student.gender,
		cardSerial: student.cardSerial,
		classId: student.classId
	};
	return student.id ? await updateStudent(student.id, req) : await createStudent(req);
}

export async function deleteStudent(id: string): Promise<void> {
	const before = await getStudent(id);
	await getDriver().transaction(async () => {
		const deletedEventsRow = await getDriver().queryOne<{ count: number }>(
			'SELECT COUNT(*) AS count FROM events WHERE student_id = ?',
			[id]
		);
		await getDriver().execute('DELETE FROM sf2_student_mappings WHERE student_id = ?', [id]);
		// The month mappings too: the roster sync that follows this delete rebuilds the
		// roster from `students`, and a mapping left behind would put the row back.
		await getDriver().execute('DELETE FROM sf2_month_student_mappings WHERE student_id = ?', [id]);
		await getDriver().execute('DELETE FROM events WHERE student_id = ?', [id]);
		const rows = await getDriver().execute('DELETE FROM students WHERE id = ?', [id]);
		if (rows === 0) throw appError('StudentNotFound', id);
		await recordAuditEvent({
			entityType: 'student',
			entityId: before.id,
			action: 'delete',
			summary: `Deleted student ${before.name}`,
			beforeJson: JSON.stringify(before),
			metadataJson: JSON.stringify({ deletedEvents: Number(deletedEventsRow?.count ?? 0) })
		});
	});
}
