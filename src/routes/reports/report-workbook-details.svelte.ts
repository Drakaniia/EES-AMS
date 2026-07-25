import type { Sf2WorkbookSettings, Sf2TemplateDraft } from '$lib/db-rust';
import { normalizedSf2FirstSchoolDay } from '$lib/features/settings/sf2-workbook';

// ── Workbook details draft state ───────────────────────────────────────────────
// Manages the reactive draft fields for the SF2 workbook header (school info,
// grade level, adviser, etc.) and provides helper functions for validation and
// payload construction.

export function createWorkbookDetailsDraft() {
	let schoolId = $state('');
	let schoolName = $state('');
	let schoolYear = $state('');
	let reportMonth = $state('');
	let gradeLevel = $state('');
	let section = $state('');
	let adviserName = $state('');
	let schoolHeadName = $state('');

	function hydrate(settings: Sf2WorkbookSettings) {
		schoolId = settings.schoolId;
		schoolName = settings.schoolName;
		schoolYear = settings.schoolYear;
		reportMonth = settings.reportMonth;
		gradeLevel = settings.gradeLevel;
		section = settings.section;
		adviserName = settings.adviserName;
		schoolHeadName = settings.schoolHeadName;
	}

	function clear() {
		schoolId = '';
		schoolName = '';
		schoolYear = '';
		reportMonth = '';
		gradeLevel = '';
		section = '';
		adviserName = '';
		schoolHeadName = '';
	}

	function onFieldChange(field: string, value: string) {
		if (field === 'draftSchoolId') schoolId = value;
		else if (field === 'draftSchoolName') schoolName = value;
		else if (field === 'draftSchoolYear') schoolYear = value;
		else if (field === 'draftReportMonth') reportMonth = value;
		else if (field === 'draftGradeLevel') gradeLevel = value;
		else if (field === 'draftSection') section = value;
		else if (field === 'draftAdviserName') adviserName = value;
		else if (field === 'draftSchoolHeadName') schoolHeadName = value;
	}

	function headerFields(): { label: string; value: string }[] {
		return [
			{ label: 'School ID', value: schoolId },
			{ label: 'Name of School', value: schoolName },
			{ label: 'School Year', value: schoolYear },
			{ label: 'Report Month', value: reportMonth },
			{ label: 'Grade Level', value: gradeLevel },
			{ label: 'Section', value: section },
			{ label: 'Adviser / LIS Name', value: adviserName },
			{ label: 'School Head Name', value: schoolHeadName }
		];
	}

	function blankFields(): string[] {
		return headerFields()
			.filter((field) => field.value.trim() === '')
			.map((field) => field.label);
	}

	function hasChanges(settings: Sf2WorkbookSettings | null): boolean {
		if (!settings) return false;
		// Report Month is excluded from modal draft changes because it is
		// changed exclusively via the Switch Month button, not the edit dialog.
		return (
			schoolId !== settings.schoolId ||
			schoolName !== settings.schoolName ||
			schoolYear !== settings.schoolYear ||
			gradeLevel !== settings.gradeLevel ||
			section !== settings.section ||
			adviserName !== settings.adviserName ||
			schoolHeadName !== settings.schoolHeadName
		);
	}

	function buildPayload(
		activeClassId: string,
		settings: Sf2WorkbookSettings | null
	): Sf2TemplateDraft | null {
		if (!settings || !activeClassId) return null;
		return {
			classId: activeClassId,
			schoolId,
			schoolName,
			schoolYear,
			reportMonth,
			gradeLevel,
			section,
			adviserName,
			schoolHeadName,
			firstSchoolDay: normalizedSf2FirstSchoolDay(
				reportMonth,
				schoolYear,
				settings.firstSchoolDay
			),
			learnerNames: []
		};
	}

	return {
		get schoolId() {
			return schoolId;
		},
		get schoolName() {
			return schoolName;
		},
		get schoolYear() {
			return schoolYear;
		},
		get reportMonth() {
			return reportMonth;
		},
		get gradeLevel() {
			return gradeLevel;
		},
		get section() {
			return section;
		},
		get adviserName() {
			return adviserName;
		},
		get schoolHeadName() {
			return schoolHeadName;
		},

		hydrate,
		clear,
		onFieldChange,
		headerFields,
		blankFields,
		hasChanges,
		buildPayload
	};
}
