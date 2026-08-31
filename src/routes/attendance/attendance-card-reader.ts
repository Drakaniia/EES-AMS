import { findStudentByCard } from '$lib/db-rust';
import type { Student, AttendanceType } from '$lib/db-rust';
import type { AttendanceLogHandle } from './attendance-page-state.svelte';

/**
 * Minimal page-state shape consumed by card reader helpers.
 * Kept intentionally narrow so the module does not depend on the full class.
 */
export type CardReaderState = {
	cardInput: string;
	cardInputElement: HTMLInputElement | null;
	isProcessing: boolean;
	dateLoading: boolean;
	lastScan: { serial: string; timestamp: number } | null;
	attendanceLog: AttendanceLogHandle | undefined;
	logForStudent(
		student: Student,
		forcedType?: AttendanceType | null,
		options?: Record<string, unknown>
	): Promise<void>;
};

/**
 * Handle a card-reader tap by looking up the student and logging attendance.
 *
 * Extracted from `AttendancePageState.handleCardSubmit` to reduce class size.
 */
export async function handleCardSubmit(state: CardReaderState, serial: string): Promise<void> {
	const trimmed = serial.trim();
	if (!trimmed) return;

	if (state.isProcessing || state.dateLoading) {
		state.attendanceLog?.showToast('Please wait - processing previous tap', false);
		return;
	}

	const now = Date.now();
	if (
		state.lastScan &&
		state.lastScan.serial === trimmed &&
		now - state.lastScan.timestamp < 2500
	) {
		state.cardInput = '';
		state.attendanceLog?.showToast(
			'Duplicate card tap ignored - wait a moment before scanning again',
			false
		);
		state.cardInputElement?.focus();
		return;
	}

	state.lastScan = { serial: trimmed, timestamp: now };
	state.cardInput = '';
	state.isProcessing = true;

	try {
		const student = await findStudentByCard(trimmed);

		if (!student) {
			state.attendanceLog?.showToast('Unknown card - not paired to any student', false);
			return;
		}
		await state.logForStudent(student);
	} catch (err: unknown) {
		const message = err instanceof Error ? err.message : String(err);
		state.attendanceLog?.showToast(`Error: ${message}`, false);
	} finally {
		state.isProcessing = false;
		state.cardInputElement?.focus();
	}
}
