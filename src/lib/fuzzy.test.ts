import { describe, expect, it } from 'vitest';
import { bestMatchScore, fuzzyMatch } from './fuzzy';

describe('fuzzyMatch', () => {
	it('returns a zero-score match for an empty query', () => {
		expect(fuzzyMatch('', 'Anything')).toEqual({ score: 0, indices: [] });
		expect(fuzzyMatch('   ', 'Anything')).toEqual({ score: 0, indices: [] });
	});

	it('returns null when the query does not appear in order', () => {
		expect(fuzzyMatch('zxc', 'Juan')).toBeNull();
		expect(fuzzyMatch('acb', 'abc')).toBeNull();
	});

	it('returns null when the query is longer than the text', () => {
		expect(fuzzyMatch('longer query', 'abc')).toBeNull();
	});

	it('matches case-insensitively', () => {
		const match = fuzzyMatch('JUAN', 'Juan Dela Cruz');
		expect(match).not.toBeNull();
		expect(match!.indices).toEqual([0, 1, 2, 3]);
	});

	it('matches a subsequence across words', () => {
		const match = fuzzyMatch('j dc', 'Juan Dela Cruz');
		expect(match).not.toBeNull();
		// j(0) space(4) d(5) space(9) c(10)
		expect(match!.indices).toEqual([0, 5, 10]);
	});

	it('matches a surname-first subsequence', () => {
		expect(fuzzyMatch('dela cruz', 'Juan Dela Cruz')).not.toBeNull();
		expect(fuzzyMatch('cruz', 'Juan Dela Cruz')).not.toBeNull();
	});

	it('scores consecutive runs above scattered matches', () => {
		const consecutive = fuzzyMatch('juan', 'Juan Dela Cruz');
		const scattered = fuzzyMatch('jdz', 'Juan Dela Cruz');
		expect(consecutive).not.toBeNull();
		expect(scattered).not.toBeNull();
		expect(consecutive!.score).toBeGreaterThan(scattered!.score);
	});

	it('rewards a prefix match above a later one', () => {
		const prefix = fuzzyMatch('juan', 'Juan Dela Cruz');
		const later = fuzzyMatch('juan', 'Mary Juanita Cruz');
		expect(prefix).not.toBeNull();
		expect(later).not.toBeNull();
		expect(prefix!.score).toBeGreaterThan(later!.score);
	});

	it('rewards word-boundary starts', () => {
		const boundary = fuzzyMatch('del', 'Juan Dela Cruz');
		expect(boundary).not.toBeNull();
		// Same-length match, but mid-word: "del" inside "Bantadelaro".
		const midWord = fuzzyMatch('del', 'Bantadelaro');
		expect(midWord).not.toBeNull();
		expect(boundary!.score).toBeGreaterThan(midWord!.score);
	});

	it('records matched indices', () => {
		const match = fuzzyMatch('dc', 'Dela Cruz');
		expect(match!.indices).toEqual([0, 5]);
	});
});

describe('bestMatchScore', () => {
	it('returns null when nothing matches', () => {
		expect(bestMatchScore('zzz', 'Reports', 'sf2 excel')).toBeNull();
	});

	it('returns the best score across fields', () => {
		const labelOnly = bestMatchScore('sf2', 'SF2 Reports')!;
		const keywordOnly = bestMatchScore('sf2', 'SF2 Reports', 'excel deped')!;
		expect(labelOnly).toBeGreaterThan(0);
		expect(keywordOnly).toBe(labelOnly);
	});

	it('returns 0 for an empty query', () => {
		expect(bestMatchScore('', 'anything')).toBe(0);
	});
});
