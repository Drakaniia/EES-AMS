import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { sf2ValidationReportText } from '$lib/features/settings/sf2-validation';
import {
	newSf2WorkbookDraftFields,
	sf2DraftFromWorkbookSettings,
	sf2ImportedSettingsDraftDefaults,
	sf2ImportedSettingsReviewNotice,
	sf2TemplateDraftFromFields,
	shouldPromptForSf2SettingsUpdate,
	type Sf2DraftDefaults,
	type Sf2WorkbookDraftFields
} from '$lib/features/settings/sf2-workbook';
import {
	createSf2WorkbookFromTemplate,
	getSf2WorkbookSettings,
	importSf2Workbook,
	updateSf2WorkbookSettings,
	validateSf2WorkbookImport,
	type Class,
	type Sf2ImportSummary,
	type Sf2ImportValidation,
	type Sf2TemplateDraft,
	type Sf2WorkbookSettings
} from '$lib/features/settings/native';
import type { Ctx } from './state-context';

/**
 * SF2 workbook creation, import, and template-draft state and actions.
 *
 * Singleton pattern: imported by both orchestrator and components.
 * The orchestrator calls `.init(ctx)` to wire cross-cutting services.
 */
class Sf2State {
	ctx!: Ctx;

	init(ctx: Ctx) {
		this.ctx = ctx;
	}

	// ── State ──────────────────────────────────────────────────────────────────
	sf2Importing = $state(false);
	sf2TemplateCreating = $state(false);
	sf2SettingsSaving = $state(false);
	sf2TemplateClassId = $state('');
	sf2ImportSummary = $state<Sf2ImportSummary | null>(null);
	sf2Validation = $state<Sf2ImportValidation | null>(null);
	sf2ValidationDialogOpen = $state(false);
	sf2ValidationDetailsOpen = $state(false);
	sf2TemplateDialogOpen = $state(false);
	sf2TemplateDialogMode = $state<'create' | 'edit'>('create');
	sf2TemplateDialogNotice = $state<string | null>(null);

	// ── SF2 Progress (Phase 2) ────────────────────────────────────────────────
	/** Current progress event listener (cleaned up on completion). */
	private sf2ProgressUnlisten: UnlistenFn | null = null;
	/** The active task name ('import' | 'create' | 'update' | ''). */
	sf2ProgressTask = $state('');
	/** Current progress step (1-based). */
	sf2ProgressCurrent = $state(0);
	/** Total progress steps. */
	sf2ProgressTotal = $state(0);
	/** The progress overlay is visible. */
	sf2ProgressVisible = $state(false);
	/** The current display message shown in the overlay. */
	sf2ProgressDisplayMessage = $state('');
	/** Whether the backend sent a message that should be shown (timed). */
	private sf2ProgressLastBackendMsg = $state('');
	private sf2ProgressLastBackendTime = $state(0);
	/** Timer for fallback cycling messages when backend is quiet. */
	private sf2ProgressCycleTimer: ReturnType<typeof setInterval> | null = null;

	// ── Draft fields ───────────────────────────────────────────────────────────
	sf2DraftSchoolId = $state('');
	sf2DraftSchoolName = $state('');
	sf2DraftSchoolYear = $state('');
	sf2DraftReportMonth = $state('');
	sf2DraftGradeLevel = $state('');
	sf2DraftSection = $state('');
	sf2DraftAdviserName = $state('');
	sf2DraftSchoolHeadName = $state('');
	sf2DraftFirstSchoolDay = $state(1);

	// ── SF2 Progress Helpers ────────────────────────────────────────────────────

	/** Friendly fallback messages that cycle when the backend is quiet. */
	private static SF2_PROGRESS_MESSAGES: Record<string, string[]> = {
		import: [
			'Reading the SF2 workbook…',
			'Validating student data…',
			'Creating student mappings…',
			'Still working on it…',
			'Just a moment longer…'
		],
		create: [
			'Setting up the bundled template…',
			'Writing student names…',
			'Configuring attendance columns…',
			'Almost there…'
		],
		update: [
			'Updating workbook settings…',
			'Reconfiguring calendar…',
			'Writing changes…'
		]
	};

	private sf2ProgressCycleIndex = 0;

	async sf2SetupProgressListener(task: string) {
		this.sf2CleanupProgress();
		this.sf2ProgressTask = task;
		this.sf2ProgressCurrent = 0;
		this.sf2ProgressTotal = 0;
		this.sf2ProgressVisible = true;
		this.sf2ProgressDisplayMessage = '';
		this.sf2ProgressLastBackendMsg = '';
		this.sf2ProgressLastBackendTime = 0;
		this.sf2ProgressCycleIndex = 0;

		try {
			this.sf2ProgressUnlisten = await listen<{
				task: string;
				current: number;
				total: number;
				message: string;
			}>('sf2-progress', (event) => {
				// Only process events matching the active task
				if (event.payload.task === this.sf2ProgressTask) {
					this.sf2ProgressCurrent = event.payload.current;
					this.sf2ProgressTotal = event.payload.total;
					if (event.payload.message) {
						this.sf2ProgressLastBackendMsg = event.payload.message;
						this.sf2ProgressLastBackendTime = Date.now();
					}
					this.sf2UpdateProgressMessage();
				}
			});
		} catch {
			// Listener setup failed — continue without it (no progress updates)
		}

		// Start cycling fallback messages
		this.sf2StartProgressCycle();
	}

	private sf2StartProgressCycle() {
		this.sf2StopProgressCycle();
		this.sf2UpdateProgressMessage();
		this.sf2ProgressCycleTimer = setInterval(() => {
			const now = Date.now();
			// Only advance cycle if no backend message arrived in the last 3s
			if (now - this.sf2ProgressLastBackendTime > 3000) {
				const messages =
					Sf2State.SF2_PROGRESS_MESSAGES[this.sf2ProgressTask] ??
					Sf2State.SF2_PROGRESS_MESSAGES.import;
				this.sf2ProgressCycleIndex =
					(this.sf2ProgressCycleIndex + 1) % messages.length;
			}
			this.sf2UpdateProgressMessage();
		}, 2500);
	}

	private sf2StopProgressCycle() {
		if (this.sf2ProgressCycleTimer !== null) {
			clearInterval(this.sf2ProgressCycleTimer);
			this.sf2ProgressCycleTimer = null;
		}
	}

	private sf2UpdateProgressMessage() {
		// Backend message has priority for 4 seconds
		if (this.sf2ProgressLastBackendMsg && Date.now() - this.sf2ProgressLastBackendTime < 4000) {
			this.sf2ProgressDisplayMessage = this.sf2ProgressLastBackendMsg;
			return;
		}
		// Map progress steps to messages when no backend message
		if (this.sf2ProgressCurrent > 0 && this.sf2ProgressTotal > 0) {
			const stepMessages: Record<string, Record<number, string>> = {
				import: {
					1: 'Analyzing workbook structure…',
					2: 'Finding class for imported workbook…',
					3: 'Processing student data…',
					4: 'Validating learner roster…',
					5: 'Creating date mappings…',
					6: 'Creating working copy…',
					7: 'Finalizing workbook…'
				},
				create: {
					1: 'Creating SF2 working workbook…',
					2: 'Finalizing workbook…'
				}
			};
			const taskMessages = stepMessages[this.sf2ProgressTask];
			const stepMessage = taskMessages?.[this.sf2ProgressCurrent];
			if (stepMessage) {
				this.sf2ProgressDisplayMessage = stepMessage;
				return;
			}
		}
		// Fallback to cycling messages
		const messages =
			Sf2State.SF2_PROGRESS_MESSAGES[this.sf2ProgressTask] ??
			Sf2State.SF2_PROGRESS_MESSAGES.import;
		this.sf2ProgressDisplayMessage =
			messages[this.sf2ProgressCycleIndex % messages.length];
	}

	sf2CleanupProgress() {
		this.sf2StopProgressCycle();
		if (this.sf2ProgressUnlisten) {
			this.sf2ProgressUnlisten();
			this.sf2ProgressUnlisten = null;
		}
	}

	private sf2HideProgress() {
		this.sf2CleanupProgress();
		this.sf2ProgressVisible = false;
		this.sf2ProgressTask = '';
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────
	private errorMessage(error: unknown, fallback: string): string {
		if (error instanceof Error) return error.message;
		if (typeof error === 'string') return error;
		return fallback;
	}

	// ── SF2 Actions ────────────────────────────────────────────────────────────
	async onImportSf2() {
		if (this.sf2Importing) return;
		this.sf2Importing = true;

		// Show progress overlay before the validation dialog
		await this.sf2SetupProgressListener('import');

		try {
			const validation = await validateSf2WorkbookImport();
			this.sf2Validation = validation;

			if (validation.hasDiscrepancies) {
				this.sf2HideProgress();
				this.sf2ValidationDialogOpen = true;
				this.sf2ValidationDetailsOpen = false;
				this.ctx.toast('Student list mismatch detected. Review the SF2 validation report.', false);
				return;
			}

			await this.runSf2Import(validation, false);
		} catch (error) {
			const msg = this.errorMessage(error, 'SF2 import failed');
			this.ctx.toast(`SF2 import failed: ${msg}`, false);
		} finally {
			this.sf2Importing = false;
			this.sf2HideProgress();
		}
	}

	async runSf2Import(validation: Sf2ImportValidation, proceedAnyway: boolean) {
		const summary = await importSf2Workbook(validation.sourcePath, proceedAnyway);
		await this.finishSf2Import(summary);
	}

	async finishSf2Import(summary: Sf2ImportSummary) {
		this.sf2ImportSummary = summary;
		this.sf2TemplateClassId = summary.classId;
		this.sf2Validation = null;
		this.sf2ValidationDialogOpen = false;
		this.sf2ValidationDetailsOpen = false;
		await this.ctx.reload();
		await this.ctx.reloadAuditEvents();

		try {
			const settings = await getSf2WorkbookSettings(summary.classId);
			if (shouldPromptForSf2SettingsUpdate(settings)) {
				this.openImportedSf2SettingsReview(settings);
				this.ctx.toast(
					`Imported ${summary.learnersFound} learners. Review SF2 settings first.`
				);
				return;
			}
		} catch (error) {
			const msg = this.errorMessage(error, 'SF2 settings check failed');
			this.ctx.toast(
				`Imported ${summary.learnersFound} learners, but settings check failed: ${msg}`,
				false
			);
			return;
		}

		this.ctx.toast(`Imported ${summary.learnersFound} learners from SF2`);
	}

	async proceedWithSf2MismatchImport() {
		if (!this.sf2Validation || this.sf2Importing) return;
		this.sf2Importing = true;

		// Set up progress for the actual import (validation dialog is hidden now)
		await this.sf2SetupProgressListener('import');

		try {
			await this.runSf2Import(this.sf2Validation, true);
		} catch (error) {
			const msg = this.errorMessage(error, 'SF2 import failed');
			this.ctx.toast(`SF2 import failed: ${msg}`, false);
		} finally {
			this.sf2Importing = false;
			this.sf2HideProgress();
		}
	}

	cancelSf2ValidationImport() {
		if (this.sf2Importing) return;
		this.sf2Validation = null;
		this.sf2ValidationDialogOpen = false;
		this.sf2ValidationDetailsOpen = false;
	}

	downloadSf2ValidationReport() {
		if (!this.sf2Validation) return;
		const blob = new Blob([sf2ValidationReportText(this.sf2Validation)], {
			type: 'text/plain;charset=utf-8'
		});
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = url;
		link.download = 'sf2-validation-report.txt';
		link.click();
		URL.revokeObjectURL(url);
	}

	// ── Template Dialog ────────────────────────────────────────────────────────
	async openSf2TemplateDialog(classes: Class[]) {
		const classId = this.sf2TemplateClassId || classes[0]?.id || '';
		if (classId) {
			try {
				const settings = await getSf2WorkbookSettings(classId);
				this.populateSf2Draft(settings);
				this.sf2TemplateDialogMode = 'edit';
				this.sf2TemplateDialogNotice =
					'An SF2 workbook already exists for this class. Update the saved workbook settings instead of creating a new SF2 copy.';
				this.sf2TemplateDialogOpen = true;
				this.ctx.toast(
					'Existing SF2 workbook found. Update settings instead of creating a new one.',
					false
				);
				return;
			} catch {
				// No workbook exists for this class yet.
			}
		}

		this.sf2TemplateDialogMode = 'create';
		this.sf2TemplateDialogNotice = null;
		this.applySf2Draft({ ...newSf2WorkbookDraftFields(), classId });
		this.sf2TemplateDialogOpen = true;
	}

	closeSf2TemplateDialog(force = false) {
		if (!force && (this.sf2TemplateCreating || this.sf2SettingsSaving)) return;
		this.sf2TemplateDialogOpen = false;
		this.sf2TemplateDialogNotice = null;
	}

	openImportedSf2SettingsReview(settings: Sf2WorkbookSettings) {
		const defaults = sf2ImportedSettingsDraftDefaults(settings);

		this.sf2TemplateDialogMode = 'edit';
		this.sf2TemplateDialogNotice = sf2ImportedSettingsReviewNotice(settings, defaults);
		this.populateSf2Draft(settings, defaults);
		this.sf2TemplateDialogOpen = true;
	}

	populateSf2Draft(settings: Sf2WorkbookSettings, defaults?: Partial<Sf2DraftDefaults>) {
		this.applySf2Draft(sf2DraftFromWorkbookSettings(settings, defaults));
	}

	applySf2Draft(draft: Sf2WorkbookDraftFields) {
		this.sf2TemplateClassId = draft.classId ?? '';
		this.sf2DraftSchoolId = draft.schoolId;
		this.sf2DraftSchoolName = draft.schoolName;
		this.sf2DraftSchoolYear = draft.schoolYear;
		this.sf2DraftReportMonth = draft.reportMonth;
		this.sf2DraftGradeLevel = draft.gradeLevel;
		this.sf2DraftSection = draft.section;
		this.sf2DraftAdviserName = draft.adviserName;
		this.sf2DraftSchoolHeadName = draft.schoolHeadName;
		this.sf2DraftFirstSchoolDay = draft.firstSchoolDay;
	}

	sf2DraftPayload(): Sf2TemplateDraft {
		const payload = sf2TemplateDraftFromFields(
			{
				classId: this.sf2TemplateClassId,
				schoolId: this.sf2DraftSchoolId,
				schoolName: this.sf2DraftSchoolName,
				schoolYear: this.sf2DraftSchoolYear,
				reportMonth: this.sf2DraftReportMonth,
				gradeLevel: this.sf2DraftGradeLevel,
				section: this.sf2DraftSection,
				adviserName: this.sf2DraftAdviserName,
				schoolHeadName: this.sf2DraftSchoolHeadName,
				firstSchoolDay: this.sf2DraftFirstSchoolDay
			},
			this.sf2TemplateDialogMode
		);
		this.sf2DraftFirstSchoolDay = payload.firstSchoolDay ?? this.sf2DraftFirstSchoolDay;
		return payload;
	}

	async onCreateSf2FromTemplate(event: SubmitEvent) {
		event.preventDefault();
		if (this.sf2TemplateCreating || this.sf2SettingsSaving) return;

		const creating = this.sf2TemplateDialogMode === 'create';
		if (creating) {
			this.sf2TemplateCreating = true;
		} else {
			this.sf2SettingsSaving = true;
		}

		// Show progress overlay for create (2 backend steps)
		if (creating) {
			await this.sf2SetupProgressListener('create');
		}

		try {
			const draft = this.sf2DraftPayload();
			const summary = creating
				? await createSf2WorkbookFromTemplate(draft)
				: await updateSf2WorkbookSettings(draft);
			this.sf2ImportSummary = summary;
			this.sf2TemplateClassId = summary.classId;
			this.closeSf2TemplateDialog(true);
			await this.ctx.reload();
			await this.ctx.reloadAuditEvents();
			this.ctx.toast(
				creating
					? `Created SF2 working copy for ${summary.learnersFound} learners`
					: `Updated SF2 workbook for ${summary.learnersFound} learners`
			);
		} catch (error) {
			const msg = this.errorMessage(
				error,
				creating ? 'SF2 template setup failed' : 'SF2 update failed'
			);
			this.ctx.toast(`${creating ? 'SF2 template setup' : 'SF2 update'} failed: ${msg}`, false);
		} finally {
			this.sf2TemplateCreating = false;
			this.sf2SettingsSaving = false;
			if (creating) {
				this.sf2HideProgress();
			}
		}
	}

	startSf2Attendance() {
		if (!this.sf2ImportSummary) return;
		goto(resolve(`/attendance?classId=${this.sf2ImportSummary.classId}&manual=true`));
	}
}

export const sf2State = new Sf2State();
