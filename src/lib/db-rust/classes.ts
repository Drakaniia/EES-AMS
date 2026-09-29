import { invoke } from '@tauri-apps/api/core';
import type { Class } from '../types';
export type { Class } from '../types';

export async function listClasses(): Promise<Class[]> {
	const backendClasses = (await invoke('list_classes')) as Array<Class>;
	return backendClasses.map((cls) => ({
		id: cls.id,
		name: cls.name,
		room: cls.room,
		dayStart: cls.dayStart,
		dayEnd: cls.dayEnd,
		lateAfter: cls.lateAfter,
		sessions: cls.sessions,
		days: cls.days,
		createdAt: cls.createdAt
	}));
}

export async function getClass(id: string): Promise<Class | undefined> {
	const backendClass = (await invoke('get_class', { id })) as Class | undefined;
	if (!backendClass) return undefined;
	return {
		id: backendClass.id,
		name: backendClass.name,
		room: backendClass.room,
		dayStart: backendClass.dayStart,
		dayEnd: backendClass.dayEnd,
		lateAfter: backendClass.lateAfter,
		sessions: backendClass.sessions,
		days: backendClass.days,
		createdAt: backendClass.createdAt
	};
}

/**
 * Every class on record. Read-only from the UI (spec §12.1, D15, D18).
 *
 * `create_class` / `update_class` / `delete_class` are still registered in Rust
 * and still reachable by anyone who invokes them; Settings simply no longer
 * offers a way to, because the per-month model is one class by design (D2) and a
 * CRUD screen over a single row is a control that can only misfire.
 */
