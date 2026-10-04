import type { BackupKind } from '$lib/types';

export function formatBackupTimestamp(value?: number) {
	if (!value) return 'Never';
	return new Date(value * 1000).toLocaleString(undefined, {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
		hour: 'numeric',
		minute: '2-digit'
	});
}

export function formatBackupBytes(value: number) {
	if (value < 1024) return `${value} B`;
	if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
	return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

export function backupKindLabel(kind: BackupKind) {
	if (kind === 'auto') return 'Auto';
	if (kind === 'manual') return 'Manual';
	if (kind === 'pre_restore') return 'Pre-restore';
	if (kind === 'pre_wipe') return 'Pre-wipe';
	if (kind === 'pre_install') return 'Pre-update';
	if (kind === 'manual_workbooks') return 'Workbooks';
	return 'Unknown';
}
