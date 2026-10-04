/**
 * Workbook and class naming rules — the port of `src-tauri/src/sf2/naming.rs`.
 *
 * These two functions decide what a class is called in the database and what the
 * workbook on disk is called, so they are the one place a spelling change has to
 * be made: an SF2 workbook is matched back to its class by name.
 */

/**
 * The class name an SF2 analysis implies: `3 - MATAPAT`.
 *
 * Either half may be blank, and a blank half is dropped rather than leaving a
 * dangling separator — a workbook whose grade-level cell was never filled still
 * names the class it is obviously about.
 */
export function className(gradeLevel: string, section: string): string {
	const grade = gradeLevel.trim();
	const trimmedSection = section.trim();
	if (grade && trimmedSection) return `${grade} - ${trimmedSection}`;
	if (grade) return grade;
	if (trimmedSection) return trimmedSection;
	return 'SF2 Class';
}

function isAsciiAlphanumeric(character: string): boolean {
	return /[0-9A-Za-z]/.test(character);
}

/**
 * One file-name-safe path segment: `Grade 3` becomes `GRADE-3`.
 *
 * Everything that is not an ASCII alphanumeric becomes a separator and runs of
 * separators collapse, so the result can never hold a character Windows will not
 * accept in a path. Uppercased because these names are shown to the teacher in
 * the Settings list, not read by a program.
 */
export function sanitizeFilePart(value: string): string {
	return [...value]
		.map((character) => (isAsciiAlphanumeric(character) ? character.toUpperCase() : '-'))
		.join('')
		.split('-')
		.filter((part) => part !== '')
		.join('-');
}
