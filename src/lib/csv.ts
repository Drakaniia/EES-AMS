import type { AttendanceEvent, Student } from './db';

const pad = (n: number) => String(n).padStart(2, '0');

export const fmtDate = (ts: number) => {
	const d = new Date(ts);
	return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};

export const fmtTime = (ts: number) => {
	const d = new Date(ts);
	return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
};

export const fmtDateTime = (ts: number) => `${fmtDate(ts)} ${fmtTime(ts)}`;

const escape = (v: string | number | undefined | null) => {
	if (v === undefined || v === null) return '';
	const s = String(v);
	return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
};

export function eventsToCSV(
	events: AttendanceEvent[],
	students: Student[],
	lateAfter: string
): string {
	const byStudent = new Map(students.map((s) => [s.id, s]));
	const groups = new Map<
		string,
		{ student: Student; date: string; ins: number[]; outs: number[] }
	>();

	for (const e of events) {
		const student = byStudent.get(e.studentId);
		if (!student) continue;
		const date = fmtDate(e.timestamp);
		const key = `${student.id}|${date}`;
		let g = groups.get(key);
		if (!g) {
			g = { student, date, ins: [], outs: [] };
			groups.set(key, g);
		}
		if (e.type === 'in') g.ins.push(e.timestamp);
		else g.outs.push(e.timestamp);
	}

	const rows = [...groups.values()]
		.sort((a, b) =>
			a.date === b.date
				? a.student.name.localeCompare(b.student.name)
				: a.date.localeCompare(b.date)
		)
		.map((g) => {
			const checkIn = g.ins.length ? Math.min(...g.ins) : null;
			const checkOut = g.outs.length ? Math.max(...g.outs) : null;
			const duration =
				checkIn && checkOut && checkOut > checkIn
					? ((checkOut - checkIn) / 3600000).toFixed(2)
					: '';
			const late =
				checkIn && lateAfter ? (fmtTime(checkIn) > lateAfter ? 'Yes' : 'No') : '';
			return [
				g.date,
				g.student.studentNumber,
				g.student.name,
				checkIn ? fmtTime(checkIn) : '',
				checkOut ? fmtTime(checkOut) : '',
				duration,
				late
			];
		});

	const header = ['Date', 'Student #', 'Name', 'Check-in', 'Check-out', 'Hours', 'Late'];
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
