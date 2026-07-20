import {
	createBackupNow,
	openBackupFolder,
	chooseBackupSyncFolder,
	clearBackupSyncFolder,
	connectGoogleDriveBackup,
	disconnectGoogleDriveBackup,
	uploadLatestBackupToGoogleDrive,
	chooseRestoreBackup,
	restoreBackup,
	exportDatabase,
	exportJsonWithFolder,
	importAll,
	wipeAll,
	getBackupStatus,
	listBackups,
	type BackupPreview,
	type BackupSummary,
	type BackupStatus
} from '$lib/features/settings/native';
import { googleDriveStatusLabel as backupGoogleDriveStatusLabel } from '$lib/features/settings/backup';
import type { Ctx } from './state-context';

/**
 * Backup/restore, export/import, and wipe state and actions.
 *
 * Singleton pattern: imported by both orchestrator and components.
 * The orchestrator calls `.init(ctx)` to wire cross-cutting services.
 */
class BackupState {
	ctx!: Ctx;

	init(ctx: Ctx) {
		this.ctx = ctx;
	}

	// ── Backup & Restore ───────────────────────────────────────────────────────
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

	// ── Export ─────────────────────────────────────────────────────────────────
	exportDialogOpen = $state(false);
	exportFormat = $state<'json' | 'database'>('json');

	// ── Wipe ───────────────────────────────────────────────────────────────────
	wipeTarget = $state(false);

	// ── Helpers ─────────────────────────────────────────────────────────────────
	private errorMessage(error: unknown, fallback: string): string {
		if (error instanceof Error) return error.message;
		if (typeof error === 'string') return error;
		return fallback;
	}

	googleDriveStatusLabel(): string {
		return backupGoogleDriveStatusLabel(this.backupStatus);
	}

	// ── Reload backup data ─────────────────────────────────────────────────────
	async reloadBackupStatus() {
		try {
			this.backupStatus = await getBackupStatus();
		} catch (err: unknown) {
			const msg = this.errorMessage(err, 'Backup status unavailable');
			this.ctx.toast(`Backup status unavailable: ${msg}`, false);
		}
	}

	async reloadBackupSummaries() {
		try {
			this.backupSummaries = await listBackups();
		} catch (err: unknown) {
			const msg = this.errorMessage(err, 'Backup list unavailable');
			this.ctx.toast(`Backup list unavailable: ${msg}`, false);
		}
	}

	async reloadBackups() {
		await Promise.all([this.reloadBackupStatus(), this.reloadBackupSummaries()]);
	}

	// ── Backup Actions ─────────────────────────────────────────────────────────
	async onCreateBackupNow() {
		if (this.backupBusy) return;
		this.backupBusy = true;
		try {
			this.backupStatus = await createBackupNow();
			await this.reloadBackupSummaries();
			this.ctx.toast('Backup created');
		} catch (error) {
			const msg = this.errorMessage(error, 'Backup failed');
			this.ctx.toast(`Backup failed: ${msg}`, false);
		} finally {
			this.backupBusy = false;
		}
	}

	async onOpenBackupFolder() {
		if (this.backupFolderOpening) return;
		this.backupFolderOpening = true;
		try {
			await openBackupFolder();
			this.ctx.toast('Backup folder opened');
		} catch (error) {
			const msg = this.errorMessage(error, 'Failed to open backup folder');
			this.ctx.toast(`Failed to open backup folder: ${msg}`, false);
		} finally {
			this.backupFolderOpening = false;
		}
	}

	async onChooseBackupSyncFolder() {
		if (this.syncFolderBusy) return;
		this.syncFolderBusy = true;
		try {
			this.backupStatus = await chooseBackupSyncFolder();
			this.ctx.toast(
				this.backupStatus.syncFolderPath ? 'Local sync folder set' : 'Backup folder unchanged'
			);
		} catch (error) {
			const msg = this.errorMessage(error, 'Sync folder selection failed');
			this.ctx.toast(`Sync folder selection failed: ${msg}`, false);
		} finally {
			this.syncFolderBusy = false;
		}
	}

	async onClearBackupSyncFolder() {
		if (this.syncFolderBusy) return;
		this.syncFolderBusy = true;
		try {
			this.backupStatus = await clearBackupSyncFolder();
			this.ctx.toast('Backup sync folder cleared');
		} catch (error) {
			const msg = this.errorMessage(error, 'Failed to clear sync folder');
			this.ctx.toast(`Failed to clear sync folder: ${msg}`, false);
		} finally {
			this.syncFolderBusy = false;
		}
	}

	async onConnectGoogleDriveBackup() {
		if (this.googleDriveBusy) return;
		this.googleDriveBusy = true;
		try {
			this.backupStatus = await connectGoogleDriveBackup();
			this.ctx.toast('Google Drive connected');
		} catch (error) {
			const msg = this.errorMessage(error, 'Google Drive connection failed');
			this.ctx.toast(`Google Drive connection failed: ${msg}`, false);
		} finally {
			this.googleDriveBusy = false;
		}
	}

	async onDisconnectGoogleDriveBackup() {
		if (this.googleDriveBusy) return;
		this.googleDriveBusy = true;
		try {
			this.backupStatus = await disconnectGoogleDriveBackup();
			this.ctx.toast('Google Drive disconnected');
		} catch (error) {
			const msg = this.errorMessage(error, 'Google Drive disconnect failed');
			this.ctx.toast(`Google Drive disconnect failed: ${msg}`, false);
		} finally {
			this.googleDriveBusy = false;
		}
	}

	async onUploadLatestBackupToGoogleDrive() {
		if (this.googleDriveBusy) return;
		this.googleDriveBusy = true;
		try {
			this.backupStatus = await uploadLatestBackupToGoogleDrive();
			this.ctx.toast('Latest backup uploaded to Google Drive');
		} catch (error) {
			const msg = this.errorMessage(error, 'Google Drive upload failed');
			this.ctx.toast(`Google Drive upload failed: ${msg}`, false);
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
			this.ctx.toast(`Restore preview failed: ${msg}`, false);
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
			await Promise.all([this.ctx.reload(), this.reloadBackups()]);
			this.ctx.toast(`Database restored. Safety backup: ${result.preRestoreBackupPath}`);
		} catch (error) {
			const msg = this.errorMessage(error, 'Restore failed');
			this.ctx.toast(`Restore failed: ${msg}`, false);
		} finally {
			this.restoreBusy = false;
		}
	}

	// ── Export / Import ─────────────────────────────────────────────────────────
	openExportDialog() {
		this.exportDialogOpen = true;
	}

	async onExport() {
		try {
			let filePath: string;

			if (this.exportFormat === 'database') {
				filePath = await exportDatabase();
				this.ctx.toast(`Database exported to: ${filePath}`);
			} else {
				filePath = await exportJsonWithFolder();
				this.ctx.toast(`JSON exported to: ${filePath}`);
			}

			this.exportDialogOpen = false;
		} catch (error) {
			const msg = this.errorMessage(error, 'Export failed');
			this.ctx.toast(`Export failed: ${msg}`, false);
		}
	}

	async onImport(file: File) {
		try {
			const txt = await file.text();
			const data = JSON.parse(txt);
			await importAll(data);
			await this.ctx.reload();
			this.ctx.toast('Backup imported');
		} catch (err: unknown) {
			const msg = this.errorMessage(err, 'Unknown error');
			this.ctx.toast(`Import failed: ${msg}`, false);
		}
	}

	handleFileChange(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (file) this.onImport(file);
		input.value = '';
	}

	// ── Wipe ───────────────────────────────────────────────────────────────────
	onWipe() {
		this.wipeTarget = true;
	}

	async onWipeConfirm() {
		await wipeAll();
		await this.ctx.reload();
		this.ctx.toast('All data wiped');
	}
}

export const backupState = new BackupState();
