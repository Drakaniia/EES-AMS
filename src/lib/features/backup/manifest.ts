/**
 * `manifest.json` — the file that makes a backup trustworthy.
 *
 * Everything else in the archive is bytes the app can only trust because the
 * app wrote them; the manifest is what lets a restore say *what* it is about to
 * put back: which app version wrote it, which schema the rows came from, when,
 * and how many of each thing there were. Without it a teacher cannot tell a
 * snapshot of last week from one taken a second ago, and a mismatch between the
 * workbook X counts and the database's absence count is invisible until after
 * the restore has already replaced good data.
 *
 * The old Rust manifest lived beside a backup *folder* and carried a
 * `kind` string that recorded how the backup was taken. This is a zip, and
 * `kind` is a `BackupKind` directly, so that mapping is gone.
 */

import { asAppError, invalidInput } from '$lib/db';
import type { BackupKind } from '$lib/types';

/**
 * Bumped whenever the archive layout changes. D12: no backward compatibility.
 *
 * Format 1 is the SQL-dump era (`database.sql`); format 2 carries the real
 * `db.sqlite` image, so an archive from an older build is refused up front —
 * with the message that it belongs to the app that wrote it — rather than
 * half-applied.
 */
export const MANIFEST_FORMAT_VERSION = 2;

export const MANIFEST_FILE_NAME = 'manifest.json';

/** The database file image inside the archive: one `sqlite3_serialize()` output. */
export const DATABASE_FILE_NAME = 'db.sqlite';

/** Every workbook entry's path starts with this. */
export const WORKBOOK_PREFIX = 'workbooks/';

export type ManifestCounts = {
	students: number;
	classes: number;
	events: number;
	absent: number;
	settings: number;
	sf2Templates: number;
	/** 0 for a database older than the `sf2_month_templates` migration. */
	sf2MonthTemplates: number;
};

export type ManifestWorkbook = {
	/** Slash-separated path inside the archive, e.g. `workbooks/_legacy/SF2-….xlsx`. */
	path: string;
	bytes: number;
	/** `"X"` cells at snapshot time. 0 means "unknown", never "no absences". */
	xCount: number;
};

export type BackupManifest = {
	formatVersion: number;
	appVersion: string;
	/** `CURRENT_SCHEMA_VERSION` when the backup was taken. */
	schemaVersion: number;
	/** RFC 3339, so the file is readable without the app. */
	createdAt: string;
	kind: BackupKind;
	/** False for a workbooks-only archive, which carries no database. */
	includesDatabase: boolean;
	counts: ManifestCounts;
	workbooks: ManifestWorkbook[];
};

const KINDS: readonly BackupKind[] = [
	'auto',
	'manual',
	'pre_restore',
	'pre_wipe',
	'pre_install',
	'manual_workbooks',
	'unknown'
];

export function toManifestJson(manifest: BackupManifest): string {
	return JSON.stringify(manifest, null, 2);
}

/**
 * Parse a manifest, refusing anything this build cannot honestly restore.
 *
 * A restore overwrites the teacher's live database and workbooks. An archive
 * whose manifest does not parse, is written by an unknown format version, or
 * carries no timestamp is not something to guess about — the Rust code reached
 * for `anyhow::Context` and the teacher saw the parse failure instead of a
 * half-restored app, and that behaviour is kept.
 */
export function parseManifest(text: string): BackupManifest {
	let parsed: unknown;
	try {
		parsed = JSON.parse(text);
	} catch (thrown) {
		throw invalidInput(
			`the backup's ${MANIFEST_FILE_NAME} is not valid JSON: ${asAppError(thrown).detail}`
		);
	}
	if (typeof parsed !== 'object' || parsed === null) {
		throw invalidInput(`the backup's ${MANIFEST_FILE_NAME} is not an object`);
	}
	const raw = parsed as Record<string, unknown>;

	const formatVersion = requireInteger(raw.formatVersion, 'formatVersion');
	if (formatVersion !== MANIFEST_FORMAT_VERSION) {
		throw invalidInput(
			`this backup was written in archive format ${formatVersion}; this app reads format ${MANIFEST_FORMAT_VERSION}. Restore it with the version of the app that took it.`
		);
	}

	const createdAt = requireString(raw.createdAt, 'createdAt');
	if (Number.isNaN(Date.parse(createdAt))) {
		throw invalidInput(
			`the backup's ${MANIFEST_FILE_NAME} has an unreadable createdAt: ${createdAt}`
		);
	}

	const kind = raw.kind;
	if (typeof kind !== 'string' || !KINDS.includes(kind as BackupKind)) {
		throw invalidInput(`the backup's ${MANIFEST_FILE_NAME} has an unknown kind: ${String(kind)}`);
	}

	return {
		formatVersion,
		appVersion: typeof raw.appVersion === 'string' ? raw.appVersion : 'unknown',
		schemaVersion: requireInteger(raw.schemaVersion, 'schemaVersion'),
		createdAt,
		kind: kind as BackupKind,
		includesDatabase: raw.includesDatabase === true,
		counts: parseCounts(raw.counts),
		workbooks: parseWorkbooks(raw.workbooks)
	};
}

function parseCounts(value: unknown): ManifestCounts {
	const raw = typeof value === 'object' && value !== null ? (value as Record<string, unknown>) : {};
	return {
		students: countField(raw.students),
		classes: countField(raw.classes),
		events: countField(raw.events),
		absent: countField(raw.absent),
		settings: countField(raw.settings),
		sf2Templates: countField(raw.sf2Templates),
		sf2MonthTemplates: countField(raw.sf2MonthTemplates)
	};
}

/** A missing count is 0, not a failure: the field only has to be a number. */
function countField(value: unknown): number {
	return typeof value === 'number' && Number.isFinite(value) ? value : 0;
}

function parseWorkbooks(value: unknown): ManifestWorkbook[] {
	if (!Array.isArray(value)) return [];
	const entries: ManifestWorkbook[] = [];
	for (const item of value) {
		if (typeof item !== 'object' || item === null) continue;
		const raw = item as Record<string, unknown>;
		if (typeof raw.path !== 'string' || !raw.path.startsWith(WORKBOOK_PREFIX)) continue;
		entries.push({ path: raw.path, bytes: countField(raw.bytes), xCount: countField(raw.xCount) });
	}
	return entries;
}

function requireInteger(value: unknown, field: string): number {
	if (typeof value !== 'number' || !Number.isFinite(value)) {
		throw invalidInput(`the backup's ${MANIFEST_FILE_NAME} is missing a numeric ${field}`);
	}
	return value;
}

function requireString(value: unknown, field: string): string {
	if (typeof value !== 'string' || value === '') {
		throw invalidInput(`the backup's ${MANIFEST_FILE_NAME} is missing ${field}`);
	}
	return value;
}
