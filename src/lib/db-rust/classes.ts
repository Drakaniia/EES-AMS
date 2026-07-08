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

export async function saveClass(classData: Class, isUpdate: boolean = false): Promise<Class> {
	let backendClass: Class;

	if (isUpdate) {
		backendClass = await invoke('update_class', {
			id: classData.id,
			req: {
				name: classData.name,
				room: classData.room,
				dayStart: classData.dayStart,
				dayEnd: classData.dayEnd,
				lateAfter: classData.lateAfter,
				sessions: classData.sessions,
				days: classData.days
			}
		});
	} else {
		backendClass = await invoke('create_class', {
			req: {
				name: classData.name,
				room: classData.room,
				dayStart: classData.dayStart,
				dayEnd: classData.dayEnd,
				lateAfter: classData.lateAfter,
				sessions: classData.sessions,
				days: classData.days
			}
		});
	}

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

export async function deleteClass(id: string): Promise<void> {
	return await invoke('delete_class', { id });
}
