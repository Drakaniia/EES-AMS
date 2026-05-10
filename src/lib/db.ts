import { openDB, type DBSchema, type IDBPDatabase } from 'idb';

export type Student = {
	id: string;
	name: string;
	studentNumber: string;
	cardSerial?: string;
	createdAt: number;
};

export type AttendanceEvent = {
	id: string;
	studentId: string;
	type: 'in' | 'out';
	timestamp: number;
	note?: string;
};

export type Settings = {
	id: 'app';
	className: string;
	dayStart: string; // "08:30"
	dayEnd: string; // "15:30"
	lateAfter: string;
};

interface AttendanceDB extends DBSchema {
	students: {
		key: string;
		value: Student;
		indexes: { 'by-card': string; 'by-name': string };
	};
	events: {
		key: string;
		value: AttendanceEvent;
		indexes: { 'by-student': string; 'by-timestamp': number };
	};
	settings: {
		key: string;
		value: Settings;
	};
}

let dbPromise: Promise<IDBPDatabase<AttendanceDB>> | null = null;

export function getDB() {
	if (typeof window === 'undefined') {
		return Promise.reject(new Error('DB only available in browser'));
	}
	if (!dbPromise) {
		dbPromise = openDB<AttendanceDB>('horizon-attendance', 1, {
			upgrade(db) {
				const s = db.createObjectStore('students', { keyPath: 'id' });
				s.createIndex('by-card', 'cardSerial');
				s.createIndex('by-name', 'name');
				const e = db.createObjectStore('events', { keyPath: 'id' });
				e.createIndex('by-student', 'studentId');
				e.createIndex('by-timestamp', 'timestamp');
				db.createObjectStore('settings', { keyPath: 'id' });
			}
		});
	}
	return dbPromise;
}

export const uid = () =>
	crypto?.randomUUID?.() ?? Math.random().toString(36).slice(2) + Date.now().toString(36);

export async function listStudents(): Promise<Student[]> {
	const db = await getDB();
	const all = await db.getAll('students');
	return all.sort((a, b) => a.name.localeCompare(b.name));
}

export async function saveStudent(s: Student) {
	const db = await getDB();
	await db.put('students', s);
}

export async function deleteStudent(id: string) {
	const db = await getDB();
	await db.delete('students', id);
	const tx = db.transaction('events', 'readwrite');
	const idx = tx.store.index('by-student');
	for await (const cursor of idx.iterate(id)) {
		await cursor.delete();
	}
	await tx.done;
}

export async function findStudentByCard(serial: string): Promise<Student | undefined> {
	const db = await getDB();
	return db.getFromIndex('students', 'by-card', serial);
}

export async function listEvents(): Promise<AttendanceEvent[]> {
	const db = await getDB();
	const all = await db.getAll('events');
	return all.sort((a, b) => b.timestamp - a.timestamp);
}

export async function listEventsForStudent(studentId: string) {
	const db = await getDB();
	return db.getAllFromIndex('events', 'by-student', studentId);
}

export async function lastEventForStudent(studentId: string) {
	const list = await listEventsForStudent(studentId);
	return list.sort((a, b) => b.timestamp - a.timestamp)[0];
}

export async function addEvent(e: AttendanceEvent) {
	const db = await getDB();
	await db.put('events', e);
}

export async function deleteEvent(id: string) {
	const db = await getDB();
	await db.delete('events', id);
}

const DEFAULT_SETTINGS: Settings = {
	id: 'app',
	className: 'My Class',
	dayStart: '08:30',
	dayEnd: '15:30',
	lateAfter: '08:45'
};

export async function getSettings(): Promise<Settings> {
	const db = await getDB();
	const s = await db.get('settings', 'app');
	return s ?? DEFAULT_SETTINGS;
}

export async function saveSettings(s: Settings) {
	const db = await getDB();
	await db.put('settings', s);
}

export async function exportAll() {
	const db = await getDB();
	const [students, events, settings] = await Promise.all([
		db.getAll('students'),
		db.getAll('events'),
		db.getAll('settings')
	]);
	return { students, events, settings, exportedAt: Date.now() };
}

export async function importAll(payload: {
	students?: Student[];
	events?: AttendanceEvent[];
	settings?: Settings[];
}) {
	const db = await getDB();
	const tx = db.transaction(['students', 'events', 'settings'], 'readwrite');
	for (const s of payload.students ?? []) await tx.objectStore('students').put(s);
	for (const e of payload.events ?? []) await tx.objectStore('events').put(e);
	for (const s of payload.settings ?? []) await tx.objectStore('settings').put(s);
	await tx.done;
}

export async function wipeAll() {
	const db = await getDB();
	const tx = db.transaction(['students', 'events', 'settings'], 'readwrite');
	await tx.objectStore('students').clear();
	await tx.objectStore('events').clear();
	await tx.objectStore('settings').clear();
	await tx.done;
}
