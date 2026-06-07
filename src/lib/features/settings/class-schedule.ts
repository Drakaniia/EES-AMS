export function classDaysLabel(days?: number[]) {
	if (!days || days.length === 0) return 'None';
	if (days.length === 7) return 'Everyday';

	const weekdays = [1, 2, 3, 4, 5];
	if (days.length === 5 && weekdays.every((day) => days.includes(day))) return 'Weekdays';

	const shortDayNames = ['S', 'M', 'T', 'W', 'TH', 'F', 'S'];
	return days
		.slice()
		.sort((a, b) => a - b)
		.map((day) => shortDayNames[day])
		.join(' ');
}
