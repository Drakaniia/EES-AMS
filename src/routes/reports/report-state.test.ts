import { describe, it, expect } from 'vitest';
import type { Sf2PreviewDate } from '$lib/types';
import {
	buildMatrixWeekGroups,
	weekdayIndexForDate,
	matrixDateLabel
} from './report-state.svelte';

describe('buildMatrixWeekGroups', () => {
	it('creates dateKey slots for ALL weekdays even with empty dates array', () => {
		const groups = buildMatrixWeekGroups([], 'JANUARY');

		// January has 31 days. Weekdays (Mon-Fri) should all have dateKey set.
		// Weekends should NOT have dateKey.
		const allSlots = groups.flatMap((g) => g.slots);

		// Every slot with a dateKey should be a weekday
		for (const slot of allSlots) {
			if (slot.dateKey !== null) {
				expect(weekdayIndexForDate(slot.dateKey)).toBeGreaterThanOrEqual(0);
				expect(weekdayIndexForDate(slot.dateKey)).toBeLessThanOrEqual(4);
			}
		}

		// Count slots with dateKey = total weekdays in January 2026
		const slotsWithDateKey = allSlots.filter((s) => s.dateKey !== null);
		expect(slotsWithDateKey.length).toBeGreaterThan(20); // ~22 weekdays in January

		// Verify each slot with dateKey has the correct structure
		for (const slot of slotsWithDateKey) {
			expect(slot.dateKey).toMatch(/^\d{4}-\d{2}-\d{2}$/);
			expect(slot.key).toBe(slot.dateKey);
			// date will be null when no SF2 mapping exists
		}
	});

	it('associates SF2 dates with matching slots when provided', () => {
		const sf2Dates = [
			{ date: '2026-01-05', sheetName: 'JANUARY', columnLetter: 'F', columnIndex: 1 },
			{ date: '2026-01-06', sheetName: 'JANUARY', columnLetter: 'G', columnIndex: 2 }
		];
		const groups = buildMatrixWeekGroups(sf2Dates, 'JANUARY');
		const allSlots = groups.flatMap((g) => g.slots);

		// These dates should have date set (not null)
		const monSlot = allSlots.find((s) => s.dateKey === '2026-01-05');
		expect(monSlot).toBeDefined();
		expect(monSlot!.date).not.toBeNull();
		expect(monSlot!.date!.columnLetter).toBe('F');

		const tueSlot = allSlots.find((s) => s.dateKey === '2026-01-06');
		expect(tueSlot).toBeDefined();
		expect(tueSlot!.date).not.toBeNull();
		expect(tueSlot!.date!.columnLetter).toBe('G');

		// Other weekdays should have dateKey but date=null (no SF2 mapping for this test)
		const otherSlots = allSlots.filter(
			(s) => s.dateKey !== null && s.dateKey !== '2026-01-05' && s.dateKey !== '2026-01-06'
		);
		expect(otherSlots.length).toBeGreaterThan(0);
		for (const slot of otherSlots) {
			expect(slot.date).toBeNull();
			expect(slot.dateKey).not.toBeNull();
		}
	});

	it('handles unknown month by falling back to dates array', () => {
		const sf2Dates = [
			{ date: '2026-01-05', sheetName: 'SHEET1', columnLetter: 'F', columnIndex: 1 }
		];
		const groups = buildMatrixWeekGroups(sf2Dates, 'UNKNOWN');
		const allSlots = groups.flatMap((g) => g.slots);

		// Only the SF2 dates should produce slots when month is unknown
		const slotsWithDate = allSlots.filter((s) => s.date !== null);
		expect(slotsWithDate.length).toBe(1);
		expect(slotsWithDate[0].dateKey).toBe('2026-01-05');
	});

	it('completes under 50ms for worst-case dataset (all school months + full dates)', () => {
		// Generate a full year of date mappings (worst case: every schoolday mapped)
		const dateMappings: Sf2PreviewDate[] = [];
		const months = [
			'AUGUST', 'SEPTEMBER', 'OCTOBER', 'NOVEMBER', 'DECEMBER',
			'JANUARY', 'FEBRUARY', 'MARCH', 'APRIL', 'MAY', 'JUNE'
		];
		for (const monthName of months) {
			// Add ~22 weekdays per month (realistic full mapping)
			for (let day = 1; day <= 22; day++) {
				dateMappings.push({
					date: `2026-${String(months.indexOf(monthName) + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`,
					sheetName: monthName,
					columnLetter: String.fromCharCode(65 + day),
					columnIndex: day
				});
			}
		}

		const start = performance.now();
		const groups = buildMatrixWeekGroups(dateMappings, 'JANUARY');
		const elapsed = performance.now() - start;

		expect(elapsed).toBeLessThan(50);
		expect(groups.length).toBeGreaterThan(0);
	});
});

describe('weekdayIndexForDate', () => {
	it('returns 0-4 for weekdays', () => {
		expect(weekdayIndexForDate('2026-01-05')).toBe(0); // Monday
		expect(weekdayIndexForDate('2026-01-06')).toBe(1); // Tuesday
		expect(weekdayIndexForDate('2026-01-07')).toBe(2); // Wednesday
		expect(weekdayIndexForDate('2026-01-08')).toBe(3); // Thursday
		expect(weekdayIndexForDate('2026-01-09')).toBe(4); // Friday
	});

	it('returns -1 for weekends', () => {
		expect(weekdayIndexForDate('2026-01-10')).toBe(-1); // Saturday
		expect(weekdayIndexForDate('2026-01-11')).toBe(-1); // Sunday
	});
});

describe('matrixDateLabel', () => {
	it('formats date as weekday + day', () => {
		const label = matrixDateLabel('2026-01-05');
		expect(label).toContain('5');
	});
});
