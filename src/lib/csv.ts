import type { AttendanceEvent, Student, Class } from './types';

const pad = (n: number) => String(n).padStart(2, '0');

export const fmtDate = (ts: number | string) => {
	const d = new Date(typeof ts === 'string' ? ts : ts);
	return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};

export const fmtTime = (ts: number | string) => {
	const d = new Date(typeof ts === 'string' ? ts : ts);
	return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
};

export const fmtDateTime = (ts: number | string) => `${fmtDate(ts)} ${fmtTime(ts)}`;

const escape = (v: string | number | undefined | null) => {
	if (v === undefined || v === null) return '';
	const s = String(v);
	return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
};

export function eventsToCSV(
	events: AttendanceEvent[],
	students: Student[],
	classes: Class[],
	globalLateAfter: string
): string {
	const byStudent = new Map(students.map((s) => [s.id, s]));
	const byClass = new Map(classes.map((c) => [c.id, c]));
	const groups = new Map<
		string,
		{ student: Student; date: string; ins: number[]; classId?: string }
	>();

	for (const e of events) {
		const student = byStudent.get(e.studentId);
		if (!student) continue;
		const date = fmtDate(e.timestamp);
		const key = `${student.id}|${date}`;
		let g = groups.get(key);
		if (!g) {
			g = { student, date, ins: [], classId: e.classId || student.classId };
			groups.set(key, g);
		}
		const timestamp =
			typeof e.timestamp === 'string' ? new Date(e.timestamp).getTime() : e.timestamp;
		g.ins.push(timestamp);
	}

	const rows = [...groups.values()]
		.sort((a, b) =>
			a.date === b.date
				? a.student.name.localeCompare(b.student.name)
				: a.date.localeCompare(b.date)
		)
		.map((g) => {
			const checkIn = g.ins.length ? Math.min(...g.ins) : null;

			const cls = g.classId ? byClass.get(g.classId) : null;
			const lateThreshold = cls?.lateAfter || globalLateAfter;

			const late =
				checkIn && lateThreshold ? (fmtTime(checkIn) > lateThreshold ? 'Yes' : 'No') : '';
			return [
				g.date,
				cls?.name || 'Unknown',
				g.student.name,
				checkIn ? fmtTime(checkIn) : '',
				late
			];
		});

	const header = ['Date', 'Class', 'Name', 'IN', 'Late'];
	return [header, ...rows].map((r) => r.map(escape).join(',')).join('\n');
}

export function downloadCSV(filename: string, content: string) {
	const blob = new Blob([content], { type: 'text/csv;charset=utf-8;' });
	const url = URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.href = url;
	a.download = filename;
	a.click();
	URL.revokeObjectURL(url);
}
