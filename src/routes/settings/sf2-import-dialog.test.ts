import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Sf2ImportDialog from './sf2-import-dialog.svelte';
import type { Sf2ImportValidation } from '$lib/features/sf2/validation';

function sampleValidation(): Sf2ImportValidation {
	return {
		sourcePath: '/workbooks/staged.xlsx',
		classId: 'class-1',
		className: 'Grade 3 - MATAPAT',
		currentStudentCount: 3,
		sf2LearnerCount: 5,
		missingFromSf2: [
			{ studentId: 's1', name: 'ALVARADO, ZYRON JAY E.', normalizedName: 'alvarado zyron jay e' },
			{ studentId: 's2', name: 'BAPTISMA, JONATHAN', normalizedName: 'baptisma jonathan' }
		],
		missingFromCurrent: [],
		possibleNameMismatches: [],
		duplicateCurrentStudents: [],
		duplicateSf2Learners: [],
		missingLearnerInfo: [],
		hasDiscrepancies: true
	};
}

describe('Sf2ImportDialog', () => {
	it('shows mismatch counts and offers an explicit proceed', () => {
		render(Sf2ImportDialog, {
			props: {
				open: true,
				validation: sampleValidation(),
				busy: false,
				onProceed: () => {},
				onCancel: () => {}
			}
		});
		expect(screen.getByText(/2 missing from workbook/)).toBeTruthy();
		expect(screen.getByRole('button', { name: /import anyway/i })).toBeTruthy();
		expect(screen.getByRole('button', { name: /cancel/i })).toBeTruthy();
	});

	it('fires proceed and cancel', async () => {
		const onProceed = vi.fn();
		const onCancel = vi.fn();
		render(Sf2ImportDialog, {
			props: { open: true, validation: sampleValidation(), busy: false, onProceed, onCancel }
		});
		await fireEvent.click(screen.getByRole('button', { name: /import anyway/i }));
		expect(onProceed).toHaveBeenCalledOnce();
		await fireEvent.click(screen.getByRole('button', { name: /cancel/i }));
		expect(onCancel).toHaveBeenCalledOnce();
	});

	it('says plainly when there is nothing to disagree about', () => {
		render(Sf2ImportDialog, {
			props: {
				open: true,
				validation: { ...sampleValidation(), missingFromSf2: [], hasDiscrepancies: false },
				busy: false,
				onProceed: () => {},
				onCancel: () => {}
			}
		});
		expect(screen.getByText(/no disagreements/i)).toBeTruthy();
	});
});
