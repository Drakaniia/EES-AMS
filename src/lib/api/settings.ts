/**
 * `$lib/api/settings.ts` → `$lib/db/repos/settings` + `$lib/db/repos/audit`.
 */

import { getSettings as readSettings, saveSettings as writeSettings } from '$lib/db/repos/settings';
import type { Settings } from '$lib/types';

export { clearAuditEvents, listAuditEvents } from '$lib/db/repos/audit';

export type { Settings, AuditEvent, AttendanceMode } from '$lib/types';

export async function getSettings(): Promise<Settings> {
	return await readSettings();
}

export async function saveSettings(settings: Settings): Promise<Settings> {
	return await writeSettings(settings);
}
