import type { StudentGender } from '$lib/db-rust';

export type EntryMode = 'single' | 'bulk';

export type GenderOption = {
	value: StudentGender;
	label: string;
};

export type EntryModeTab = {
	value: EntryMode;
	label: string;
};

export const genderOptions: GenderOption[] = [
	{ value: 'male', label: 'Male' },
	{ value: 'female', label: 'Female' }
];

export const entryModeTabs: EntryModeTab[] = [
	{ value: 'single', label: 'Individual' },
	{ value: 'bulk', label: 'Bulk paste' }
];

export function parseStudentNames(value: string) {
	return value
		.split(/\r?\n/)
		.map((name) => name.trim().toUpperCase())
		.filter(Boolean);
}

export function genderLabel(gender?: StudentGender) {
	if (gender === 'male') return 'Male';
	if (gender === 'female') return 'Female';
	return 'Not set';
}
