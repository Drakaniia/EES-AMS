import type {
	Sf2ImportValidation,
	Sf2ValidationDuplicate,
	Sf2ValidationLearner,
	Sf2ValidationStudent
} from '$lib/types';

export function sf2ValidationStudentLabel(student: Sf2ValidationStudent) {
	return student.gender ? `${student.name} (${student.gender})` : student.name;
}

export function sf2ValidationLearnerLabel(learner: Sf2ValidationLearner) {
	const name = learner.name.trim() || 'Blank learner name';
	const gender = learner.genderBlock ? `, ${learner.genderBlock}` : '';
	return `Row ${learner.rowIndex}: ${name}${gender}`;
}

export function sf2ValidationDuplicateLabel(duplicate: Sf2ValidationDuplicate) {
	const locations =
		duplicate.rowIndexes.length > 0
			? `Rows ${duplicate.rowIndexes.join(', ')}`
			: `${duplicate.studentIds.length} current records`;
	return `${duplicate.names.join(', ')} (${locations})`;
}

export function sf2ValidationReportText(validation: Sf2ImportValidation) {
	const lines = [
		'Warning: Student List Mismatch Detected',
		'',
		`Source path: ${validation.sourcePath}`,
		`Class: ${validation.className}`,
		`Current records: ${validation.currentStudentCount}`,
		`SF2 learners: ${validation.sf2LearnerCount}`,
		'',
		`Current records missing from SF2: ${validation.missingFromSf2.length}`,
		...validation.missingFromSf2.map((student) => `- ${sf2ValidationStudentLabel(student)}`),
		'',
		`SF2 learners missing from current records: ${validation.missingFromCurrent.length}`,
		...validation.missingFromCurrent.map((learner) => `- ${sf2ValidationLearnerLabel(learner)}`),
		'',
		`Potential name mismatches: ${validation.possibleNameMismatches.length}`,
		...validation.possibleNameMismatches.map(
			(mismatch) =>
				`- ${mismatch.currentStudent.name} <-> ${mismatch.sf2Learner.name}: ${mismatch.reason}`
		),
		'',
		`Duplicate current records: ${validation.duplicateCurrentStudents.length}`,
		...validation.duplicateCurrentStudents.map(
			(duplicate) => `- ${sf2ValidationDuplicateLabel(duplicate)}`
		),
		'',
		`Duplicate SF2 learners: ${validation.duplicateSf2Learners.length}`,
		...validation.duplicateSf2Learners.map(
			(duplicate) => `- ${sf2ValidationDuplicateLabel(duplicate)}`
		),
		'',
		`Missing learner information: ${validation.missingLearnerInfo.length}`,
		...validation.missingLearnerInfo.map((learner) => `- ${sf2ValidationLearnerLabel(learner)}`)
	];
	return lines.join('\n');
}
