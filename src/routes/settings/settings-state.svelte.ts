import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import {
	SF2_SCHOOL_MONTHS,
	defaultSf2FirstSchoolDay,
	defaultSf2SchoolYear,
	isSf2SchoolDay,
	normalizedSf2FirstSchoolDay,
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
	auditEntityLabel,
	auditMetadataPreview,
	backupKindLabel,
	backupPathLabel,
	formatAuditTimestamp,
	formatBackupBytes,
	formatBackupTimestamp,
	googleDriveStatusLabel as backupGoogleDriveStatusLabel
} from '$lib/features/settings/backup';
import { classDaysLabel as getDaysLabel } from '$lib/features/settings/class-schedule';
import {
	buildGlobalSettingsPayload,
	globalSettingsEqual,
	normalizeGlobalSettings
} from '$lib/features/settings/global-settings';
import { sf2ValidationReportText } from '$lib/features/settings/sf2-validation';
import { settingsStore } from '$lib/stores/settings.svelte';
import {
	listClasses,
	saveClass,
	deleteClass,
	chooseBackupSyncFolder,
	chooseRestoreBackup,
	clearAuditEvents,
	clearBackupSyncFolder,
	connectGoogleDriveBackup,
	createBackupNow,
	disconnectGoogleDriveBackup,
	exportDatabase,
	exportJsonWithFolder,
	getBackupStatus,
	listAuditEvents,
	listBackups,
	createSf2WorkbookFromTemplate,
	getSf2WorkbookSettings,
	importSf2Workbook,
	importAll,
	openBackupFolder,
	restoreBackup,
	uploadLatestBackupToGoogleDrive,
	updateSf2WorkbookSettings,
	validateSf2WorkbookImport,
	wipeAll,
	type AuditEvent,
	type AttendanceMode,
	type BackupPreview,
	type BackupSummary,
	type BackupStatus,
	type Settings,
	type Class,
	type Session,
	type Sf2ImportSummary,
	type Sf2ImportValidation,
	type Sf2TemplateDraft,
	type Sf2WorkbookSettings
} from '$lib/features/settings/native';

class SettingsPageState {
	// ── Classes ────────────────────────────────────────────────────────────────
	classes = $state<Class[]>([]);
	classDialogOpen = $state(false);
	editingClass = $state<Class | null>(null);
	formClassName = $state('');
	formRoom = $state('');
	formDayStart = $state('');
	formDayEnd = $state('');
	formLateAfter = $state('');
	formSessions = $state<Session[]>([]);
	formDays = $state<number[]>([1, 2, 3, 4, 5]);
	sessionMode = $state<'single' | 'morning-afternoon' | 'custom'>('single');
	deleteTarget = $state<{ id: string; name: string } | null>(null);

	// ── Global settings ────────────────────────────────────────────────────────
	defaultDayStart = $state('08:30');
	defaultDayEnd = $state('15:30');
	defaultLateAfter = $state('08:45');
	defaultQuarter = $state('1st Quarter');
	attendanceMode = $state<AttendanceMode>('manual');

	q1Start = $state('');
	q1End = $state('');
	q2Start = $state('');
	q2End = $state('');
	q3Start = $state('');
	q3End = $state('');
	savedGlobalSettingsSnapshot = $state<Settings | null>(null);
	pendingGlobalSettingsReload = $state<Settings | null>(null);
	unsavedGlobalDialogOpen = $state(false);
	globalSettingsSaving = $state(false);
	globalSettingsDirty = $derived.by(
		() =>
			this.savedGlobalSettingsSnapshot !== null &&
			!globalSettingsEqual(this.currentSettingsPayload(), this.savedGlobalSettingsSnapshot)
	);
	quarterDialogOpen = $state(false);

	// ── Toast ──────────────────────────────────────────────────────────────────
	toastMessage = $state<string | null>(null);
	toastOk = $state(true);
	toastTimer: ReturnType<typeof setTimeout> | null = null;

	// ── Wipe ───────────────────────────────────────────────────────────────────
	wipeTarget = $state(false);

	// ── Export ─────────────────────────────────────────────────────────────────
	exportDialogOpen = $state(false);
	exportFormat = $state<'json' | 'database'>('json');

	// ── Backup & Restore ────────────────────────────────────────────────────────
	backupStatus = $state<BackupStatus | null>(null);
	backupSummaries = $state<BackupSummary[]>([]);
	backupBusy = $state(false);
	backupFolderOpening = $state(false);
	syncFolderBusy = $state(false);
	googleDriveBusy = $state(false);
	restoreChoosing = $state(false);
	restoreBusy = $state(false);
	restorePreview = $state<BackupPreview | null>(null);
	fileInput = $state<HTMLInputElement | null>(null);

	// ── Audit ──────────────────────────────────────────────────────────────────
	auditEvents = $state<AuditEvent[]>([]);
	auditLoading = $state(false);
	auditClearing = $state(false);
	auditClearTarget = $state(false);

	// ── SF2 ────────────────────────────────────────────────────────────────────
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
	sf2DraftSchoolId = $state('');
	sf2DraftSchoolName = $state('');
	sf2DraftSchoolYear = $state('');
	sf2DraftReportMonth = $state('');
	sf2DraftGradeLevel = $state('');
	sf2DraftSection = $state('');
	sf2DraftAdviserName = $state('');
	sf2DraftSchoolHeadName = $state('');
	sf2DraftFirstSchoolDay = $state(1);

	// ── Lifecycle ──────────────────────────────────────────────────────────────
	init() {
		this.reload();
		this.reloadBackups();
		this.reloadAuditEvents();
	}

	// ── Helper ─────────────────────────────────────────────────────────────────
	toast(msg: string, ok = true) {
		this.toastMessage = msg;
		this.toastOk = ok;
		if (this.toastTimer) clearTimeout(this.toastTimer);
		this.toastTimer = setTimeout(() => (this.toastMessage = null), 3000);
	}

	errorMessage(error: unknown, fallback: string) {
		if (error instanceof Error) return error.message;
		if (typeof error === 'string') return error;
		return fallback;
	}

	// ── Global Settings ────────────────────────────────────────────────────────
	currentSettingsPayload(): Settings {
		return buildGlobalSettingsPayload({
			dayStart: this.defaultDayStart,
			dayEnd: this.defaultDayEnd,
			lateAfter: this.defaultLateAfter,
			quarter: this.defaultQuarter,
			attendanceMode: this.attendanceMode,
			q1Start: this.q1Start,
			q1End: this.q1End,
			q2Start: this.q2Start,
			q2End: this.q2End,
			q3Start: this.q3Start,
			q3End: this.q3End
		});
	}

	applyGlobalSettings(settings: Settings) {
		const normalized = normalizeGlobalSettings(settings);
		this.defaultDayStart = normalized.dayStart;
		this.defaultDayEnd = normalized.dayEnd;
		this.defaultLateAfter = normalized.lateAfter;
		this.defaultQuarter = normalized.quarter;
		this.attendanceMode = normalized.attendanceMode;
		this.q1Start = normalized.q1Start ?? '';
		this.q1End = normalized.q1End ?? '';
		this.q2Start = normalized.q2Start ?? '';
		this.q2End = normalized.q2End ?? '';
		this.q3Start = normalized.q3Start ?? '';
		this.q3End = normalized.q3End ?? '';
		this.savedGlobalSettingsSnapshot = normalized;
		this.pendingGlobalSettingsReload = null;
	}

	handleGlobalSettingsFocusOut(event: FocusEvent) {
		if (!this.globalSettingsDirty || this.unsavedGlobalDialogOpen) return;
		const currentTarget = event.currentTarget as HTMLElement;
		const nextTarget = event.relatedTarget;
		if (nextTarget instanceof Node && currentTarget.contains(nextTarget)) return;
		this.unsavedGlobalDialogOpen = true;
	}

	async saveGlobalSettings() {
		if (this.globalSettingsSaving) return false;
		this.globalSettingsSaving = true;
		try {
			const savedSettings = await settingsStore.save(this.currentSettingsPayload());
			this.applyGlobalSettings(savedSettings);
			this.unsavedGlobalDialogOpen = false;
			this.toast('Global configuration saved');
			this.reloadAuditEvents();
			return true;
		} catch (error) {
			const msg = this.errorMessage(error, 'Failed to save settings');
			this.toast(`Save failed: ${msg}`, false);
			return false;
		} finally {
			this.globalSettingsSaving = false;
		}
	}

	async onSaveGlobal(e: SubmitEvent) {
		e.preventDefault();
		await this.saveGlobalSettings();
	}

	keepEditingGlobalSettings() {
		this.pendingGlobalSettingsReload = null;
		this.unsavedGlobalDialogOpen = false;
	}

	discardGlobalSettingsChanges() {
		const settingsToApply = this.pendingGlobalSettingsReload ?? this.savedGlobalSettingsSnapshot;
		if (settingsToApply) {
			this.applyGlobalSettings(settingsToApply);
		}
		this.unsavedGlobalDialogOpen = false;
	}

	async saveGlobalSettingsFromDialog() {
		await this.saveGlobalSettings();
	}

	// ── Reload ─────────────────────────────────────────────────────────────────
	async reload() {
		try {
			const [c] = await Promise.all([listClasses(), settingsStore.load()]);
			this.classes = c;
			if (settingsStore.settings) {
				const loadedSettings = normalizeGlobalSettings(settingsStore.settings);
				if (this.globalSettingsDirty) {
					this.pendingGlobalSettingsReload = loadedSettings;
					this.unsavedGlobalDialogOpen = true;
					return;
				}
				this.applyGlobalSettings(loadedSettings);
			}
		} catch (err: unknown) {
			const msg = this.errorMessage(err, 'Database error');
			this.toast(`Failed to load: ${msg}`, false);
		}
	}

	async reloadBackupStatus() {
		try {
			this.backupStatus = await getBackupStatus();
		} catch (err: unknown) {
			const msg = this.errorMessage(err, 'Backup status unavailable');
			this.toast(`Backup status unavailable: ${msg}`, false);
		}
	}

	async reloadBackupSummaries() {
		try {
			this.backupSummaries = await listBackups();
		} catch (err: unknown) {
			const msg = this.errorMessage(err, 'Backup list unavailable');
			this.toast(`Backup list unavailable: ${msg}`, false);
		}
	}

	async reloadBackups() {
		await Promise.all([this.reloadBackupStatus(), this.reloadBackupSummaries()]);
	}

	async reloadAuditEvents() {
		this.auditLoading = true;
		try {
			this.auditEvents = await listAuditEvents(50);
		} catch (err: unknown) {
			const msg = this.errorMessage(err, 'Audit trail unavailable');
			this.toast(`Audit trail unavailable: ${msg}`, false);
		} finally {
			this.auditLoading = false;
		}
	}

	async confirmClearAuditEvents() {
		if (this.auditClearing) return;
		this.auditClearing = true;
		try {
			const deletedCount = await clearAuditEvents();
			this.auditEvents = [];
			this.auditClearTarget = false;
			this.toast(
				deletedCount === 1 ? 'Cleared 1 audit event' : `Cleared ${deletedCount} audit events`
			);
		} catch (err: unknown) {
			const msg = this.errorMessage(err, 'Audit trail could not be cleared');
			this.toast(`Audit trail could not be cleared: ${msg}`, false);
		} finally {
			this.auditClearing = false;
		}
	}

	googleDriveStatusLabel() {
		return backupGoogleDriveStatusLabel(this.backupStatus);
	}

	// ── Class Actions ──────────────────────────────────────────────────────────
	openAddClass() {
		if (this.classes.length > 0) {
			this.toast('One class is already configured. Edit the existing class instead.', false);
			return;
		}
		this.editingClass = null;
		this.formClassName = '';
		this.formRoom = '';
		this.formDayStart = this.defaultDayStart;
		this.formDayEnd = this.defaultDayEnd;
		this.formLateAfter = this.defaultLateAfter;
		this.formSessions = [
			{
				name: 'Full Day',
				startTime: this.defaultDayStart,
				endTime: this.defaultDayEnd,
				lateAfter: this.defaultLateAfter
			}
		];
		this.formDays = [1, 2, 3, 4, 5];
		this.sessionMode = 'single';
		this.classDialogOpen = true;
	}

	openEditClass(c: Class) {
		this.editingClass = c;
		this.formClassName = c.name;
		this.formRoom = c.room ?? '';
		this.formDayStart = c.dayStart;
		this.formDayEnd = c.dayEnd;
		this.formLateAfter = c.lateAfter;
		this.formSessions =
			c.sessions && c.sessions.length > 0
				? JSON.parse(JSON.stringify(c.sessions))
				: [
						{
							name: 'Full Day',
							startTime: c.dayStart,
							endTime: c.dayEnd,
							lateAfter: c.lateAfter
						}
					];
		this.formDays = c.days && c.days.length > 0 ? [...c.days] : [1, 2, 3, 4, 5];

		if (this.formSessions.length === 1 && this.formSessions[0].name === 'Full Day') {
			this.sessionMode = 'single';
		} else if (
			this.formSessions.length === 2 &&
			this.formSessions[0].name === 'Morning' &&
			this.formSessions[1].name === 'Afternoon'
		) {
			this.sessionMode = 'morning-afternoon';
		} else {
			this.sessionMode = 'custom';
		}

		this.classDialogOpen = true;
	}

	async onSaveClass(e: SubmitEvent) {
		e.preventDefault();
		const name = this.formClassName.trim();
		if (!name) return;

		const primary = this.formSessions[0] || {
			startTime: this.formDayStart,
			endTime: this.formDayEnd,
			lateAfter: this.formLateAfter
		};

		const c: Class = {
			id: this.editingClass?.id ?? '',
			name,
			room: this.formRoom.trim(),
			dayStart: primary.startTime,
			dayEnd: primary.endTime,
			lateAfter: primary.lateAfter,
			sessions: this.formSessions,
			days: this.formDays,
			createdAt: this.editingClass?.createdAt ?? ''
		};

		try {
			await saveClass(c, !!this.editingClass);
			this.toast(this.editingClass ? 'Class updated' : 'Class added');
			this.classDialogOpen = false;
			this.reload();
			this.reloadAuditEvents();
		} catch (error) {
			this.toast(`Failed to save class: ${error}`, false);
		}
	}

	async confirmDeleteClass(target = this.deleteTarget) {
		if (!target) return;
		await deleteClass(target.id);
		this.toast('Class deleted');
		this.deleteTarget = null;
		this.reload();
		this.reloadAuditEvents();
	}

	async onDeleteClass(event: MouseEvent, id: string) {
		const classToDelete = this.classes.find((c) => c.id === id);
		if (!classToDelete) return;

		const target = { id: classToDelete.id, name: classToDelete.name };
		if (event.shiftKey) {
			await this.confirmDeleteClass(target);
			return;
		}

		this.deleteTarget = target;
	}

	// ── Export / Import ────────────────────────────────────────────────────────
	openExportDialog() {
		this.exportDialogOpen = true;
	}

	async onExport() {
		try {
			let filePath: string;

			if (this.exportFormat === 'database') {
				filePath = await exportDatabase();
				this.toast(`Database exported to: ${filePath}`);
			} else {
				filePath = await exportJsonWithFolder();
				this.toast(`JSON exported to: ${filePath}`);
			}

			this.exportDialogOpen = false;
			this.reloadAuditEvents();
		} catch (error) {
			const msg = this.errorMessage(error, 'Export failed');
			this.toast(`Export failed: ${msg}`, false);
		}
	}

	async onImport(file: File) {
		try {
			const txt = await file.text();
			const data = JSON.parse(txt);
			await importAll(data);
			await this.reload();
			await this.reloadAuditEvents();
			this.toast('Backup imported');
		} catch (err: unknown) {
			const msg = this.errorMessage(err, 'Unknown error');
			this.toast(`Import failed: ${msg}`, false);
		}
	}

	handleFileChange(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (file) this.onImport(file);
		input.value = '';
	}

	// ── Backup Actions ─────────────────────────────────────────────────────────
	async onCreateBackupNow() {
		if (this.backupBusy) return;
		this.backupBusy = true;
		try {
			this.backupStatus = await createBackupNow();
			await this.reloadBackupSummaries();
			await this.reloadAuditEvents();
			this.toast('Backup created');
		} catch (error) {
			const msg = this.errorMessage(error, 'Backup failed');
			this.toast(`Backup failed: ${msg}`, false);
		} finally {
			this.backupBusy = false;
		}
	}

	async onOpenBackupFolder() {
		if (this.backupFolderOpening) return;
		this.backupFolderOpening = true;
		try {
			await openBackupFolder();
			this.toast('Backup folder opened');
		} catch (error) {
			const msg = this.errorMessage(error, 'Failed to open backup folder');
			this.toast(`Failed to open backup folder: ${msg}`, false);
		} finally {
			this.backupFolderOpening = false;
		}
	}

	async onChooseBackupSyncFolder() {
		if (this.syncFolderBusy) return;
		this.syncFolderBusy = true;
		try {
			this.backupStatus = await chooseBackupSyncFolder();
			this.toast(this.backupStatus.syncFolderPath ? 'Local sync folder set' : 'Backup folder unchanged');
		} catch (error) {
			const msg = this.errorMessage(error, 'Sync folder selection failed');
			this.toast(`Sync folder selection failed: ${msg}`, false);
		} finally {
			this.syncFolderBusy = false;
		}
	}

	async onClearBackupSyncFolder() {
		if (this.syncFolderBusy) return;
		this.syncFolderBusy = true;
		try {
			this.backupStatus = await clearBackupSyncFolder();
			this.toast('Backup sync folder cleared');
		} catch (error) {
			const msg = this.errorMessage(error, 'Failed to clear sync folder');
			this.toast(`Failed to clear sync folder: ${msg}`, false);
		} finally {
			this.syncFolderBusy = false;
		}
	}

	async onConnectGoogleDriveBackup() {
		if (this.googleDriveBusy) return;
		this.googleDriveBusy = true;
		try {
			this.backupStatus = await connectGoogleDriveBackup();
			this.toast('Google Drive connected');
		} catch (error) {
			const msg = this.errorMessage(error, 'Google Drive connection failed');
			this.toast(`Google Drive connection failed: ${msg}`, false);
		} finally {
			this.googleDriveBusy = false;
		}
	}

	async onDisconnectGoogleDriveBackup() {
		if (this.googleDriveBusy) return;
		this.googleDriveBusy = true;
		try {
			this.backupStatus = await disconnectGoogleDriveBackup();
			this.toast('Google Drive disconnected');
		} catch (error) {
			const msg = this.errorMessage(error, 'Google Drive disconnect failed');
			this.toast(`Google Drive disconnect failed: ${msg}`, false);
		} finally {
			this.googleDriveBusy = false;
		}
	}

	async onUploadLatestBackupToGoogleDrive() {
		if (this.googleDriveBusy) return;
		this.googleDriveBusy = true;
		try {
			this.backupStatus = await uploadLatestBackupToGoogleDrive();
			this.toast('Latest backup uploaded to Google Drive');
		} catch (error) {
			const msg = this.errorMessage(error, 'Google Drive upload failed');
			this.toast(`Google Drive upload failed: ${msg}`, false);
		} finally {
			this.googleDriveBusy = false;
		}
	}

	async onChooseRestoreBackup() {
		if (this.restoreChoosing || this.restoreBusy) return;
		this.restoreChoosing = true;
		try {
			const preview = await chooseRestoreBackup();
			if (preview) this.restorePreview = preview;
		} catch (error) {
			const msg = this.errorMessage(error, 'Restore preview failed');
			this.toast(`Restore preview failed: ${msg}`, false);
		} finally {
			this.restoreChoosing = false;
		}
	}

	async onConfirmRestoreBackup() {
		if (!this.restorePreview || this.restoreBusy) return;
		this.restoreBusy = true;
		try {
			const result = await restoreBackup(this.restorePreview.sourcePath);
			this.restorePreview = null;
			await Promise.all([this.reload(), this.reloadBackups(), this.reloadAuditEvents()]);
			this.toast(`Database restored. Safety backup: ${result.preRestoreBackupPath}`);
		} catch (error) {
			const msg = this.errorMessage(error, 'Restore failed');
			this.toast(`Restore failed: ${msg}`, false);
		} finally {
			this.restoreBusy = false;
		}
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
				this.toast('Student list mismatch detected. Review the SF2 validation report.', false);
				return;
			}

			await this.runSf2Import(validation, false);
		} catch (error) {
			const msg = this.errorMessage(error, 'SF2 import failed');
			this.toast(`SF2 import failed: ${msg}`, false);
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
		await this.reload();
		await this.reloadAuditEvents();

		try {
			const settings = await getSf2WorkbookSettings(summary.classId);
			if (shouldPromptForSf2SettingsUpdate(settings)) {
				this.openImportedSf2SettingsReview(settings);
				this.toast(`Imported ${summary.learnersFound} learners. Review SF2 settings first.`, false);
				return;
			}
		} catch (error) {
			const msg = this.errorMessage(error, 'SF2 settings check failed');
			this.toast(`Imported ${summary.learnersFound} learners, but settings check failed: ${msg}`, false);
			return;
		}

		this.toast(`Imported ${summary.learnersFound} learners from SF2`);
	}

	async proceedWithSf2MismatchImport() {
		if (!this.sf2Validation || this.sf2Importing) return;
		this.sf2Importing = true;
		try {
			await this.runSf2Import(this.sf2Validation, true);
		} catch (error) {
			const msg = this.errorMessage(error, 'SF2 import failed');
			this.toast(`SF2 import failed: ${msg}`, false);
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

	async openSf2TemplateDialog() {
		const classId = this.sf2TemplateClassId || this.classes[0]?.id || '';
		if (classId) {
			try {
				const settings = await getSf2WorkbookSettings(classId);
				this.populateSf2Draft(settings);
				this.sf2TemplateDialogMode = 'edit';
				this.sf2TemplateDialogNotice =
					'An SF2 workbook already exists for this class. Update the saved workbook settings instead of creating a new SF2 copy.';
				this.sf2TemplateDialogOpen = true;
				this.toast('Existing SF2 workbook found. Update settings instead of creating a new one.', false);
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
			await this.reload();
			await this.reloadAuditEvents();
			this.toast(
				creating
					? `Created SF2 working copy for ${summary.learnersFound} learners`
					: `Updated SF2 workbook for ${summary.learnersFound} learners`
			);
		} catch (error) {
			const msg = this.errorMessage(error, creating ? 'SF2 template setup failed' : 'SF2 update failed');
			this.toast(`${creating ? 'SF2 template setup' : 'SF2 update'} failed: ${msg}`, false);
		} finally {
			this.sf2TemplateCreating = false;
			this.sf2SettingsSaving = false;
		}
	}

	startSf2Attendance() {
		if (!this.sf2ImportSummary) return;
		goto(resolve(`/attendance?classId=${this.sf2ImportSummary.classId}&manual=true`));
	}

	// ── Wipe ───────────────────────────────────────────────────────────────────
	onWipe() {
		this.wipeTarget = true;
	}

	async onWipeConfirm() {
		await wipeAll();
		await this.reload();
		await this.reloadAuditEvents();
		this.toast('All data wiped');
	}
}

export const settingsState = new SettingsPageState();
