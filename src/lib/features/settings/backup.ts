import type { AuditEvent, BackupKind } from '$lib/types';

const AUDIT_METADATA_KEYS = [
	'students',
	'classes',
	'events',
	'presentCount',
	'absentCount',
	'format'
];

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

export function formatAuditTimestamp(value: string) {
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) return 'Unknown time';
	return date.toLocaleString(undefined, {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
		hour: 'numeric',
		minute: '2-digit'
	});
}

export function auditEntityLabel(event: AuditEvent) {
	const entityType = event.entityType.replaceAll('_', ' ');
	if (!event.entityId) return entityType;
	const id =
		event.entityId.length > 12
			? `${event.entityId.slice(0, 8)}...${event.entityId.slice(-4)}`
			: event.entityId;
	return `${entityType} ${id}`;
}

export function auditMetadataPreview(event: AuditEvent) {
	if (!event.metadataJson) return '';
	try {
		const metadata = JSON.parse(event.metadataJson) as Record<string, unknown>;
		return AUDIT_METADATA_KEYS.filter(
			(key) => metadata[key] !== undefined && metadata[key] !== null
		)
			.map((key) => `${key}: ${metadata[key]}`)
			.join(' | ');
	} catch {
		return '';
	}
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
