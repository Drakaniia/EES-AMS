/**
 * The roster validation report.
 *
 * This is the screen that stands between a teacher's workbook and the database:
 * before anything is imported, it says exactly which learners the workbook and the
 * database disagree about, and refuses to import until the teacher has seen the
 * report and said so. Everything in this file is pure — the database is read by
 * `validation-service.ts`, which hands the rows in.
 */

import { invalidInput } from '$lib/db';
import { className } from './naming';
import { normalizeLearnerName } from './logic';
import { isLearnerName } from '$lib/features/excel/roster';
import type { Sf2WorkbookAnalysis, Sf2WorkbookLearner } from './calendar';
import type { Student } from '$lib/domain/models';

/** A database student, as the validation report shows it. */
type Sf2ValidationStudent = {
	studentId: string;
	name: string;
	normalizedName: string;
	/** `male` / `female`, or `undefined` when the school never recorded one. */
	gender?: string;
};

/** A workbook learner row, as the validation report shows it. */
type Sf2ValidationLearner = {
	rowIndex: number;
	name: string;
	normalizedName: string;
	genderBlock?: string;
};

/** One learner whose two names look like the same person typed two ways. */
type Sf2ValidationNameMismatch = {
	currentStudent: Sf2ValidationStudent;
	sf2Learner: Sf2ValidationLearner;
	reason: string;
};

/** Two rows on one side of the comparison carrying the same name. */
type Sf2ValidationDuplicate = {
	normalizedName: string;
	names: string[];
	studentIds: string[];
	rowIndexes: number[];
};

export type Sf2ImportValidation = {
	sourcePath: string;
	classId?: string;
	className: string;
	currentStudentCount: number;
	sf2LearnerCount: number;
	missingFromSf2: Sf2ValidationStudent[];
	missingFromCurrent: Sf2ValidationLearner[];
	possibleNameMismatches: Sf2ValidationNameMismatch[];
	duplicateCurrentStudents: Sf2ValidationDuplicate[];
	duplicateSf2Learners: Sf2ValidationDuplicate[];
	missingLearnerInfo: Sf2ValidationLearner[];
	hasDiscrepancies: boolean;
};

/** The class an analysis names, matched case-insensitively as Rust did. */
export function analysisClassName(analysis: Sf2WorkbookAnalysis): string {
	return className(analysis.gradeLevel, analysis.section);
}

function validationStudent(student: Student): Sf2ValidationStudent {
	return {
		studentId: student.id,
		name: student.name,
		normalizedName: normalizeLearnerName(student.name),
		gender: student.gender
	};
}

function validationLearner(learner: Sf2WorkbookLearner): Sf2ValidationLearner {
	return {
		rowIndex: learner.rowIndex,
		name: learner.name,
		normalizedName: normalizeLearnerName(learner.name),
		genderBlock: learner.genderBlock
	};
}

/**
 * Compare the class's students with the learners a workbook holds.
 *
 * `learners` is every row the workbook read, not only the learner rows: a row with
 * a name but no gender block is a real finding, and filtering it out first would
 * hide the one thing the teacher has to go and fix.
 */
export function validateStudentList(
	sourcePath: string,
	classId: string | undefined,
	className: string,
	currentStudents: readonly Student[],
	learners: readonly Sf2WorkbookLearner[]
): Sf2ImportValidation {
	const current = currentStudents.map(validationStudent);
	const validLearners = learners
		.filter((learner) => isLearnerName(learner.name))
		.map(validationLearner);

	const currentNames = new Set(current.map((student) => student.normalizedName));
	const sf2Names = new Set(validLearners.map((learner) => learner.normalizedName));

	const missingFromSf2 = current.filter((student) => !sf2Names.has(student.normalizedName));
	// A class with no students on record cannot disagree with the workbook: every
	// learner would be "missing from current", and a fresh install's first import
	// would open on a report full of invented mismatches.
	const missingFromCurrent =
		current.length === 0
			? []
			: validLearners.filter((learner) => !currentNames.has(learner.normalizedName));

	const possibleNameMismatches = findPossibleNameMismatches(missingFromSf2, missingFromCurrent);
	const duplicateCurrentStudents = findDuplicateStudents(current);
	const duplicateSf2Learners = findDuplicateLearners(validLearners);
	const missingLearnerInfo = learners
		.filter(
			(learner) =>
				learner.name.trim() === '' ||
				(isLearnerName(learner.name) && learner.genderBlock === undefined)
		)
		.map(validationLearner);

	return {
		sourcePath,
		classId,
		className,
		currentStudentCount: current.length,
		sf2LearnerCount: validLearners.length,
		missingFromSf2,
		missingFromCurrent,
		possibleNameMismatches,
		duplicateCurrentStudents,
		duplicateSf2Learners,
		missingLearnerInfo,
		hasDiscrepancies:
			missingFromSf2.length > 0 ||
			missingFromCurrent.length > 0 ||
			possibleNameMismatches.length > 0 ||
			duplicateCurrentStudents.length > 0 ||
			duplicateSf2Learners.length > 0 ||
			missingLearnerInfo.length > 0
	};
}

/**
 * Refuse an import whose roster disagrees until the teacher has said to go ahead.
 *
 * `proceedAnyway` is the teacher's explicit acknowledgement, so it is the *only*
 * thing that clears a discrepancy — a validation report nobody opened must never be
 * bypassed by a flag that defaults to true somewhere upstream.
 */
export function ensureImportValidationAllows(
	validation: Sf2ImportValidation,
	proceedAnyway: boolean
): void {
	if (validation.hasDiscrepancies && !proceedAnyway) {
		throw invalidInput(
			'Student List Mismatch Detected. Review the validation report and explicitly proceed before importing this SF2 workbook.'
		);
	}
}

/** Every pair that could be the same learner typed two ways, in both directions. */
function findPossibleNameMismatches(
	current: readonly Sf2ValidationStudent[],
	learners: readonly Sf2ValidationLearner[]
): Sf2ValidationNameMismatch[] {
	const mismatches: Sf2ValidationNameMismatch[] = [];
	for (const student of current) {
		for (const learner of learners) {
			const reason = nameMismatchReason(student.normalizedName, learner.normalizedName);
			if (reason === undefined) continue;
			mismatches.push({ currentStudent: student, sf2Learner: learner, reason });
		}
	}
	return mismatches;
}

/**
 * Why two normalized names are probably the same learner, or `undefined`.
 *
 * Two shapes only: the same tokens in another order, and a spelling close enough
 * that a human would read it as a typo. The threshold is deliberately
 * conservative — a wrong pairing here tells a teacher their two children are one
 * person.
 */
export function nameMismatchReason(current: string, learner: string): string | undefined {
	if (current === learner) return undefined;

	if (tokenSignature(current).join('|') === tokenSignature(learner).join('|')) {
		return 'Same name tokens in a different order';
	}

	const currentCompact = compactName(current);
	const learnerCompact = compactName(learner);
	const longest = Math.max(currentCompact.length, learnerCompact.length);
	if (longest >= 6 && editDistance(currentCompact, learnerCompact) <= 2) {
		return 'Very similar spelling';
	}

	return undefined;
}

function tokenSignature(value: string): string[] {
	return value
		.split(/[^a-z0-9]+/i)
		.filter((token) => token !== '')
		.sort();
}

function compactName(value: string): string {
	return [...value].filter((character) => /[a-z0-9]/i.test(character)).join('');
}

/** Levenshtein distance, one row at a time. */
export function editDistance(left: string, right: string): number {
	const rightChars = [...right];
	let previous = Array.from({ length: rightChars.length + 1 }, (_, index) => index);
	let current = new Array<number>(rightChars.length + 1).fill(0);

	[...left].forEach((leftChar, leftIndex) => {
		current[0] = leftIndex + 1;
		for (const [rightIndex, rightChar] of rightChars.entries()) {
			current[rightIndex + 1] = Math.min(
				previous[rightIndex + 1] + 1,
				current[rightIndex] + 1,
				previous[rightIndex] + (leftChar === rightChar ? 0 : 1)
			);
		}
		[previous, current] = [current, previous];
	});

	return previous[rightChars.length];
}

/**
 * Rows sharing one normalized name, in the order the first of them appeared.
 *
 * Rust collected these into a `HashMap`, so its order was arbitrary; first
 * appearance is both deterministic and the order the teacher reads the roster in.
 */
function findDuplicateStudents(
	students: readonly Sf2ValidationStudent[]
): Sf2ValidationDuplicate[] {
	const grouped = new Map<string, Sf2ValidationStudent[]>();
	for (const student of students) {
		grouped.set(student.normalizedName, [...(grouped.get(student.normalizedName) ?? []), student]);
	}

	const duplicates: Sf2ValidationDuplicate[] = [];
	for (const [normalizedName, group] of grouped) {
		if (group.length < 2) continue;
		duplicates.push({
			normalizedName,
			names: group.map((student) => student.name),
			studentIds: group.map((student) => student.studentId),
			rowIndexes: []
		});
	}
	return duplicates;
}

function findDuplicateLearners(
	learners: readonly Sf2ValidationLearner[]
): Sf2ValidationDuplicate[] {
	const grouped = new Map<string, Sf2ValidationLearner[]>();
	for (const learner of learners) {
		grouped.set(learner.normalizedName, [...(grouped.get(learner.normalizedName) ?? []), learner]);
	}

	const duplicates: Sf2ValidationDuplicate[] = [];
	for (const [normalizedName, group] of grouped) {
		if (group.length < 2) continue;
		duplicates.push({
			normalizedName,
			names: group.map((learner) => learner.name),
			studentIds: [],
			rowIndexes: group.map((learner) => learner.rowIndex)
		});
	}
	return duplicates;
}
