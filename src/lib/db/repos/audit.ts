import { getDriver } from '$lib/db';
import type { AuditEvent } from '$lib/domain/models';
import { epochSecondsToIso, nowEpochSeconds, type AuditEventInput } from '$lib/domain/models';

/**
 * The general audit trail — the port of
 * `src-tauri/src/infrastructure/database/audit.rs`.
 *
 * Every repo in this directory writes through `recordAuditEvent()` inside its
 * own transaction, so a rolled-back write leaves no orphan audit row. The app
 * has no auth, so every row is written as `admin`; that is what the Rust
 * `AuditEventDraft::new` hard-coded and it is what the Records page shows.
 */

interface AuditRow {
	id: string;
	entity_type: string;
	entity_id: string | null;
	action: string;
	summary: string;
	before_json: string | null;
	after_json: string | null;
	metadata_json: string | null;
	created_at: number;
	actor: string;
}

const AUDIT_COLUMNS =
	'id, entity_type, entity_id, action, summary, before_json, after_json, metadata_json, created_at, actor';

function toAuditEvent(row: AuditRow): AuditEvent {
	return {
		id: row.id,
		entityType: row.entity_type,
		entityId: row.entity_id ?? undefined,
		action: row.action,
		summary: row.summary,
		beforeJson: row.before_json ?? undefined,
		afterJson: row.after_json ?? undefined,
		metadataJson: row.metadata_json ?? undefined,
		createdAt: epochSecondsToIso(row.created_at),
		actor: row.actor
	};
}

export async function recordAuditEvent(input: AuditEventInput): Promise<AuditEvent> {
	// One clock read: the returned row must be the row that was written.
	const createdAt = nowEpochSeconds();
	const event: AuditEvent = {
		id: crypto.randomUUID(),
		entityType: input.entityType,
		entityId: input.entityId,
		action: input.action,
		summary: input.summary,
		beforeJson: input.beforeJson,
		afterJson: input.afterJson,
		metadataJson: input.metadataJson,
		createdAt: epochSecondsToIso(createdAt),
		actor: 'admin'
	};

	await getDriver().execute(
		`INSERT INTO audit_events (id, entity_type, entity_id, action, summary, before_json, after_json, metadata_json, created_at, actor)
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		[
			event.id,
			event.entityType,
			event.entityId ?? null,
			event.action,
			event.summary,
			event.beforeJson ?? null,
			event.afterJson ?? null,
			event.metadataJson ?? null,
			createdAt,
			event.actor
		]
	);

	return event;
}

/** Newest first. `limit` is clamped to 1..1000, defaulting to 200. */
export async function listAuditEvents(limit?: number): Promise<AuditEvent[]> {
	const bounded = Math.min(Math.max(limit ?? 200, 1), 1000);
	const rows = await getDriver().query<AuditRow>(
		`SELECT ${AUDIT_COLUMNS} FROM audit_events ORDER BY created_at DESC, id DESC LIMIT ?`,
		[bounded]
	);
	return rows.map(toAuditEvent);
}

export async function listAllAuditEvents(): Promise<AuditEvent[]> {
	const rows = await getDriver().query<AuditRow>(
		`SELECT ${AUDIT_COLUMNS} FROM audit_events ORDER BY created_at ASC, id ASC`
	);
	return rows.map(toAuditEvent);
}

export async function clearAuditEvents(): Promise<number> {
	return await getDriver().execute('DELETE FROM audit_events');
}
