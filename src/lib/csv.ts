const pad = (n: number) => String(n).padStart(2, '0');

export const fmtDate = (ts: number | string) => {
	const d = new Date(typeof ts === 'string' ? ts : ts);
	return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};

export const fmtTime = (ts: number | string) => {
	const d = new Date(typeof ts === 'string' ? ts : ts);
	return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
};
