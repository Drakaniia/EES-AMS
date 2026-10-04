import { describeError } from '$lib/db';
import {
	createSf2WorkbookFromTemplate,
	importSf2Workbook,
	listClasses,
	pickImportWorkbookFile,
	runSf2WorkbookSplit,
	stageImportWorkbook,
	validateSf2WorkbookImportFile
} from '$lib/features/settings/native';
import type { Class, Sf2ImportValidation, Sf2TemplateDraft } from '$lib/features/settings/native';
import { newSf2WorkbookDraftFields } from '$lib/features/settings/sf2-workbook';
import type { Ctx } from './state-context';

/**
 * Settings → SF2 Workbook entry points: start from template, import a school
 * workbook.
 *
 * Split from `sf2-state.svelte.ts`, which owns the month list and the split:
 * this owns the two restored entry flows and nothing else. `refresh` reloads
 * the month list after a flow changes what is on disk. The roster sync that used
 * to be the third entry point is gone: the Students page maps every save onto
 * the month worksheets itself, so there is nothing here for a button to do.
 */
class Sf2ImportState {
	ctx!: Ctx;
	refresh: () => Promise<void> = async () => {};

	init(ctx: Ctx, refresh: () => Promise<void>) {
		this.ctx = ctx;
		this.refresh = refresh;
	}

	// ── State ─────────────────────────────────────────────────────────────────
	/** Classes the create-form and sync-roster selectors offer. */
	availableClasses = $state<Class[]>([]);
	/** Empty = derive (or create) the class from the draft's grade + section. */
	selectedClassId = $state('');
	/**
	 * The create-from-template form, as typed.
	 *
	 * Prefilled with the current school year and report month (and the first
	 * attendance day they imply): those three are derivable from the clock, so a
	 * teacher is shown today's answer rather than a blank box, and clearing one
	 * falls back to it again at submit time instead of failing the build.
	 */
	createDraft = $state(newSf2WorkbookDraftFields());
	/** Learner names for the draft, one per line. */
	learnerNamesText = $state('');
	showCreateForm = $state(false);
	creating = $state(false);
	staging = $state(false);
	importing = $state(false);
	/** The staged file and its validation report, awaiting an explicit proceed. */
	importReview = $state<{ stagedPath: string; validation: Sf2ImportValidation } | null>(null);
	importDialogOpen = $state(false);

	// ── Loading ───────────────────────────────────────────────────────────────
	async loadClasses(): Promise<void> {
		try {
			this.availableClasses = await listClasses();
			if (this.availableClasses.length === 1) {
				this.selectedClassId = this.availableClasses[0]?.id ?? '';
			}
		} catch (error) {
			this.availableClasses = [];
			this.ctx.toast(`Could not read classes: ${this.errorMessage(error, 'unknown error')}`, false);
		}
	}

	// ── Start from template ───────────────────────────────────────────────────
	/**
	 * Write a fresh class workbook from the bundled DepEd template.
	 *
	 * An empty class selector is not a mistake: the backend derives (or
	 * creates) the class from the draft's grade level and section. A blank school
	 * year or report month is not a mistake either — both fall back to the
	 * current ones rather than failing the build.
	 */
	async onCreateFromTemplate(): Promise<void> {
		if (this.creating) return;
		this.creating = true;
		try {
			const current = newSf2WorkbookDraftFields();
			const draft: Sf2TemplateDraft = {
				...this.createDraft,
				schoolYear: this.createDraft.schoolYear.trim() || current.schoolYear,
				reportMonth: this.createDraft.reportMonth.trim() || current.reportMonth,
				firstSchoolDay: this.createDraft.firstSchoolDay ?? current.firstSchoolDay,
				classId: this.selectedClassId === '' ? undefined : this.selectedClassId,
				learnerNames: this.learnerNamesText.split('\n')
			};
			const summary = await createSf2WorkbookFromTemplate(draft);
			this.ctx.toast(
				`Workbook created for ${summary.className}: ${summary.learnersFound} learners, ${summary.datesMapped} days mapped.`
			);
			this.learnerNamesText = '';
			this.showCreateForm = false;
			await this.ensureSplit();
			await this.refresh();
		} catch (error) {
			this.ctx.toast(
				`Could not create the workbook: ${this.errorMessage(error, 'unknown error')}`,
				false
			);
		} finally {
			this.creating = false;
		}
	}

	// ── Import a school workbook ──────────────────────────────────────────────
	/**
	 * Pick a workbook, stage it in-scope, and open the validation report.
	 *
	 * A dismissed picker is a non-event, not an error. Nothing is written here:
	 * the import lands only after the teacher explicitly proceeds in the dialog.
	 */
	async onPickImportFile(): Promise<void> {
		if (this.staging) return;
		this.staging = true;
		try {
			const picked = await pickImportWorkbookFile();
			if (picked === null) return;
			const stagedPath = await stageImportWorkbook(picked);
			const validation = await validateSf2WorkbookImportFile(stagedPath);
			this.importReview = { stagedPath, validation };
			this.importDialogOpen = true;
		} catch (error) {
			this.ctx.toast(
				`Could not read that workbook: ${this.errorMessage(error, 'unknown error')}`,
				false
			);
		} finally {
			this.staging = false;
		}
	}

	/**
	 * Adopt the staged workbook after the validation report.
	 *
	 * The dialog stays open on failure so the teacher can cancel instead: the
	 * staged file is harmless where it is, and the next stage sweeps it.
	 */
	async onConfirmImport(proceed: boolean): Promise<void> {
		if (this.importing || this.importReview === null) return;
		this.importing = true;
		try {
			const summary = await importSf2Workbook(this.importReview.stagedPath, proceed);
			this.importReview = null;
			this.importDialogOpen = false;
			this.ctx.toast(
				`Workbook imported for ${summary.className}: ${summary.learnersFound} learners, ${summary.datesMapped} days mapped.`
			);
			await this.ensureSplit();
			await this.refresh();
		} catch (error) {
			this.ctx.toast(
				`Could not import the workbook: ${this.errorMessage(error, 'unknown error')}`,
				false
			);
		} finally {
			this.importing = false;
		}
	}

	onCancelImport(): void {
		this.importReview = null;
		this.importDialogOpen = false;
	}

	// ── Helpers ───────────────────────────────────────────────────────────────
	/**
	 * Silent auto-split after a workbook lands. Idempotent, so a no-op when
	 * there is nothing to split. Failure stays silent here: the month list
	 * refresh below already shows what is missing, and the startup auto-run
	 * retries on next launch.
	 */
	private async ensureSplit(): Promise<void> {
		try {
			await runSf2WorkbookSplit();
		} catch {
			// Leave it for the month list + next startup run to surface.
		}
	}

	private errorMessage(error: unknown, fallback: string): string {
		return describeError(error, fallback);
	}
}

export const sf2ImportState = new Sf2ImportState();
