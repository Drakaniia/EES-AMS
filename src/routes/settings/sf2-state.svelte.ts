import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
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

		try {
			const validation = await validateSf2WorkbookImport();
			this.sf2Validation = validation;

			if (validation.hasDiscrepancies) {
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
					`Imported ${summary.learnersFound} learners. Review SF2 settings first.`,
					false
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
		try {
			await this.runSf2Import(this.sf2Validation, true);
		} catch (error) {
			const msg = this.errorMessage(error, 'SF2 import failed');
			this.ctx.toast(`SF2 import failed: ${msg}`, false);
		} finally {
			this.sf2Importing = false;
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
		}
	}

	startSf2Attendance() {
		if (!this.sf2ImportSummary) return;
		goto(resolve(`/attendance?classId=${this.sf2ImportSummary.classId}&manual=true`));
	}
}

export const sf2State = new Sf2State();
