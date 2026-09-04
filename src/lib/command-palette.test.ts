import { describe, expect, it, vi } from 'vitest';
import { filterPaletteItems, type PaletteItem } from './command-palette';

function item(overrides: Partial<PaletteItem>): PaletteItem {
	return {
		id: 'item',
		label: 'Item',
		keywords: '',
		hint: '',
		group: 'Actions',
		run: vi.fn(),
		...overrides
	};
}

const items: PaletteItem[] = [
	item({ id: 'a1', label: 'SF2 Reports', group: 'Pages', keywords: 'sf2 excel' }),
	item({ id: 'a2', label: 'Take Attendance', group: 'Pages', keywords: 'card reader' }),
	item({ id: 'a3', label: 'Juan Dela Cruz', group: 'Students' }),
	item({ id: 'a4', label: 'Maria Santos', group: 'Students' }),
	item({ id: 'a5', label: 'Switch Month', group: 'Actions' })
];

describe('filterPaletteItems', () => {
	it('keeps group order Pages, Actions, Students', () => {
		const groups = filterPaletteItems(items, '');
		expect(groups.map((g) => g.group)).toEqual(['Pages', 'Actions', 'Students']);
	});

	it('matches fuzzy queries across label and keywords', () => {
		const groups = filterPaletteItems(items, 'sf2');
		const pages = groups.find((g) => g.group === 'Pages')!;
		expect(pages.items.map((i) => i.id)).toContain('a1');
	});

	it('drops items that do not match', () => {
		expect(filterPaletteItems(items, 'zzz-no-match')).toHaveLength(0);
	});

	it('finds students by partial name', () => {
		const groups = filterPaletteItems(items, 'juan');
		const studentGroup = groups.find((g) => g.group === 'Students')!;
		expect(studentGroup.items.map((i) => i.id)).toContain('a3');
	});

	it('sorts matches by score, then label', () => {
		const groups = filterPaletteItems(items, 'dela');
		const studentGroup = groups.find((g) => g.group === 'Students')!;
		expect(studentGroup.items[0].id).toBe('a3');
	});

	it('caps the student group when browsing without a query', () => {
		const manyStudents: PaletteItem[] = Array.from({ length: 10 }, (_, i) =>
			item({ id: `stu-${i}`, label: `Student ${i}`, group: 'Students' })
		);
		const groups = filterPaletteItems(manyStudents, '', 3);
		const studentGroup = groups.find((g) => g.group === 'Students')!;
		expect(studentGroup.items).toHaveLength(3);
	});

	it('caps students even with a query', () => {
		const manyStudents: PaletteItem[] = Array.from({ length: 10 }, (_, i) =>
			item({ id: `stu-${i}`, label: `Jose Number ${i}`, group: 'Students' })
		);
		const groups = filterPaletteItems(manyStudents, 'jose', 4);
		const studentGroup = groups.find((g) => g.group === 'Students')!;
		expect(studentGroup.items).toHaveLength(4);
	});
});
