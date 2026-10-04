import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invalidInput } from '$lib/db';

const native = vi.hoisted(() => ({
	pickImportWorkbookFile: vi.fn(),
	stageImportWorkbook: vi.fn(),
	validateSf2WorkbookImportFile: vi.fn(),
	importSf2Workbook: vi.fn(),
	runSf2WorkbookSplit: vi.fn(),
	listClasses: vi.fn(),
	createSf2WorkbookFromTemplate: vi.fn()
}));

vi.mock('$lib/features/settings/native', () => native);

/** An `AppError` is a plain object, not an `Error` — the toast must still name the reason. */
describe('sf2ImportState.onPickImportFile', () => {
	beforeEach(() => vi.clearAllMocks());

	it('shows the AppError detail when staging refuses an old .xls workbook', async () => {
		const toast = vi.fn();
		const { sf2ImportState } = await import('./sf2-import-state.svelte');
		sf2ImportState.init({ toast } as never, async () => {});

		native.pickImportWorkbookFile.mockResolvedValue('/picked/SF2_2026.xls');
		native.stageImportWorkbook.mockRejectedValue(
			invalidInput(
				'This is an old .xls workbook. Open it once in Excel, Save As .xlsx, then import from that file.'
			)
		);

		await sf2ImportState.onPickImportFile();

		expect(toast).toHaveBeenCalledWith(expect.stringContaining('Save As .xlsx'), false);
	});
});

/**
 * The school year and report month are derivable from the clock, so clearing them
 * is not a mistake: the build must still carry today's ones rather than fail with
 * "School Year is required".
 */
describe('sf2ImportState.onCreateFromTemplate', () => {
	beforeEach(() => vi.clearAllMocks());

	it('falls back to the current school year and month when they are cleared', async () => {
		const toast = vi.fn();
		const { sf2ImportState } = await import('./sf2-import-state.svelte');
		const { defaultSf2ReportMonth, defaultSf2SchoolYear } =
			await import('$lib/features/settings/sf2-workbook');
		sf2ImportState.init({ toast } as never, async () => {});

		native.createSf2WorkbookFromTemplate.mockResolvedValue({ className: '3 - MATAPAT' });
		native.runSf2WorkbookSplit.mockResolvedValue({});

		sf2ImportState.createDraft.schoolYear = '';
		sf2ImportState.createDraft.reportMonth = '';
		await sf2ImportState.onCreateFromTemplate();

		const draft = native.createSf2WorkbookFromTemplate.mock.calls[0]?.[0];
		expect(draft.schoolYear).toBe(defaultSf2SchoolYear());
		expect(draft.reportMonth).toBe(defaultSf2ReportMonth());
		expect(draft.firstSchoolDay).toBeGreaterThan(0);
	});
});
