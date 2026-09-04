/**
 * Hand-rolled fuzzy matching for the command palette.
 *
 * `fuzzyMatch(query, text)` returns a score when every character of `query`
 * appears in `text` in order (a subsequence match), or `null` otherwise.
 * Scoring rewards:
 *   - matches that start at a word boundary,
 *   - consecutive runs (contiguous typing beats scattered letters),
 *   - a prefix match of the whole query.
 * The result also carries the matched character indices so the UI could
 * highlight them later.
 */

export interface FuzzyMatch {
	/** Higher is better. Always positive for a successful match. */
	score: number;
	/** Indices (in `text`, case-insensitive) of the matched characters. */
	indices: number[];
}

const WORD_BOUNDARY = /[\s.\-_/()]/;

/**
 * Match a single whitespace-free token as a subsequence of `text`, scanning
 * from `from`. Returns null when the token does not appear in order.
 */
function matchToken(token: string, text: string, from: number) {
	const indices: number[] = [];
	let qi = 0;
	let score = 0;
	let prev = -1;

	for (let i = from; i < text.length && qi < token.length; i++) {
		if (text[i] !== token[qi]) continue;
		indices.push(i);
		if (prev !== -1) {
			const gap = i - prev - 1;
			// Consecutive letters score highest; one skipped char is fine;
			// larger gaps still match but weigh less.
			score += gap === 0 ? 5 : Math.max(1, 3 - gap);
		}
		// Word-start bonus: beginning of the string or after a separator.
		if (i === 0 || WORD_BOUNDARY.test(text[i - 1] ?? '')) score += 3;
		prev = i;
		qi++;
	}

	if (qi < token.length) return null;
	return { score, indices, lastIndex: prev };
}

export function fuzzyMatch(query: string, text: string): FuzzyMatch | null {
	const q = query.trim().toLowerCase();
	const t = text.toLowerCase();
	if (!q) return { score: 0, indices: [] };
	if (q.length > t.length) return null;

	// Whitespace in the query separates independent tokens: "j dc" means
	// "j, then dc later" rather than requiring a literal space in the text.
	const tokens = q.split(/\s+/).filter(Boolean);
	const indices: number[] = [];
	let score = 0;
	let searchFrom = 0;

	for (const token of tokens) {
		const match = matchToken(token, t, searchFrom);
		if (!match) return null;
		score += match.score;
		indices.push(...match.indices);
		searchFrom = match.lastIndex + 1;
	}

	// Whole-query prefix match is the strongest signal.
	if (t.startsWith(q)) score += 12;
	// Prefer matches that start earlier in the string.
	score -= (indices[0] ?? 0) * 0.5;

	return { score, indices };
}

/**
 * Best score across several candidate fields (e.g. label + keywords).
 * Returns `null` when no field matches.
 */
export function bestMatchScore(query: string, ...fields: string[]): number | null {
	let best: number | null = null;
	for (const field of fields) {
		if (!field) continue;
		const match = fuzzyMatch(query, field);
		if (match && (best === null || match.score > best)) best = match.score;
	}
	return best;
}
