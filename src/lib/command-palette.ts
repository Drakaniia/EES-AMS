/**
 * Pure command-palette model + filtering. Kept free of SvelteKit imports so
 * it can be unit-tested without mocking `$app/*`.
 */

import { bestMatchScore } from '$lib/fuzzy';

export type PaletteGroup = 'Pages' | 'Actions' | 'Students';

export interface PaletteItem {
	id: string;
	label: string;
	keywords: string;
	hint: string;
	group: PaletteGroup;
	run: () => void;
}

export interface PaletteGroupResult {
	group: PaletteGroup;
	items: PaletteItem[];
}

export const GROUP_ORDER: PaletteGroup[] = ['Pages', 'Actions', 'Students'];

/**
 * Fuzzy-filter items and group them in fixed order. Students are capped to
 * `studentLimit` so a short query never floods the list with the whole roster.
 */
export function filterPaletteItems(
	items: PaletteItem[],
	query: string,
	studentLimit = 6
): PaletteGroupResult[] {
	const q = query.trim();
	const buckets: Record<PaletteGroup, { item: PaletteItem; score: number }[]> = {
		Pages: [],
		Actions: [],
		Students: []
	};

	for (const item of items) {
		const score = bestMatchScore(q, item.label, item.keywords);
		if (score === null) continue;
		buckets[item.group].push({ item, score });
	}

	const results: PaletteGroupResult[] = [];
	for (const group of GROUP_ORDER) {
		const scored = buckets[group].sort(
			(a, b) => b.score - a.score || a.item.label.localeCompare(b.item.label)
		);
		if (scored.length === 0) continue;
		const itemsInGroup = scored.map((s) => s.item);
		if (group === 'Students') itemsInGroup.length = Math.min(itemsInGroup.length, studentLimit);
		results.push({ group, items: itemsInGroup });
	}
	return results;
}
