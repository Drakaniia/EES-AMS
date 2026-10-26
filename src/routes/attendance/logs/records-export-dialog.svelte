<script lang="ts">
	import { onDestroy } from 'svelte';

	import Dialog from '$lib/components/ui/Dialog.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import { fmtDate, fmtTime } from '$lib/csv';
	import {
		deleteEvent,
		updateEvent,
		listAttendanceAudit,
		type AttendanceEvent,
		type AttendanceAuditEntry,
		type Class
	} from '$lib/api';
	import { type StudentAttendance, primaryEvent, sessionKeyFor } from './records-state.svelte';

	let {
		classes,
		onRecordChanged
	}: {
		classes: Class[];
		onRecordChanged: () => void;
	} = $props();

	let editTarget = $state<StudentAttendance | null>(null);
	let editDate = $state('');
	let editTime = $state('');
	let editClassId = $state('');
	let editReason = $state('');
	let isSavingEdit = $state(false);

	let deleteTarget = $state<{
		studentName: string;
		date: string;
		events: AttendanceEvent[];
	} | null>(null);
	let auditTarget = $state<StudentAttendance | null>(null);
	let auditEntries = $state<AttendanceAuditEntry[]>([]);
	let auditLoading = $state(false);
	let auditError = $state<string | null>(null);

	let deleteReason = $state('');

	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = $state(null);

	onDestroy(() => {
		if (toastTimer) clearTimeout(toastTimer);
	});

	export function showToast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	export function onDeleteAttendanceRecord(_event: MouseEvent, record: StudentAttendance) {
		deleteTarget = {
			studentName: record.studentName,
			date: record.date,
			events: record.events
		};
		deleteReason = '';
	}

	async function confirmDeleteAttendanceRecords() {
		if (!deleteTarget) return;
		const reason = deleteReason.trim();
		if (reason.length < 3) {
			showToast('Delete reason is required', false);
			return;
		}
		await Promise.all(
			deleteTarget.events.map((event: AttendanceEvent) => deleteEvent(event.id, reason))
		);
		showToast('Deleted');
		deleteTarget = null;
		deleteReason = '';
		onRecordChanged();
	}

	export function onEditAttendanceRecord(record: StudentAttendance) {
		const event = primaryEvent(record);
		if (!event) return;

		editTarget = record;
		editDate = record.date;
		editTime = record.checkInTime || fmtTime(event.timestamp);
		editClassId = event.classId || record.classId || '';
		editReason = '';
	}

	async function saveAttendanceEdit() {
		if (!editTarget || isSavingEdit) return;
		const event = primaryEvent(editTarget);
		if (!event) return;

		const reason = editReason.trim();
		if (reason.length < 3) {
			showToast('Edit reason is required', false);
			return;
		}

		const timestamp = new Date(`${editDate}T${editTime}:00`);
		if (Number.isNaN(timestamp.getTime())) {
			showToast('Enter a valid date and time', false);
			return;
		}

		isSavingEdit = true;
		try {
			await updateEvent(event.id, {
				classId: editClassId || undefined,
				timestamp: timestamp.toISOString(),
				sessionKey: sessionKeyFor(editClassId, timestamp, new Map(classes.map((c) => [c.id, c]))),
				reason
			});
			showToast('Record updated');
			editTarget = null;
			onRecordChanged();
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Update failed';
			showToast(`Update failed: ${msg}`, false);
		} finally {
			isSavingEdit = false;
		}
	}

	export async function openAudit(record: StudentAttendance) {
		auditTarget = record;
		auditEntries = [];
		auditError = null;
		auditLoading = true;
		try {
			auditEntries = await listAttendanceAudit({ studentId: record.studentId });
		} catch (error) {
			auditError = error instanceof Error ? error.message : 'Audit history could not be loaded.';
		} finally {
			auditLoading = false;
		}
	}

	function cancelDelete() {
		deleteTarget = null;
		deleteReason = '';
	}
</script>

<FeedbackToast message={toastMessage} ok={toastOk} onClose={() => (toastMessage = null)} />

<Dialog
	open={!!editTarget}
	title="Edit attendance"
	description="Adjust the stored class or time and keep an audit reason."
	onClose={() => {
		if (!isSavingEdit) editTarget = null;
	}}
>
	{#if editTarget}
		<div class="grid gap-4 sm:grid-cols-2">
			<div class="space-y-2">
				<label for="edit-date" class="label-mono">Date</label>
				<input id="edit-date" type="date" bind:value={editDate} class="field" />
			</div>
			<div class="space-y-2">
				<label for="edit-time" class="label-mono">Time</label>
				<input id="edit-time" type="time" bind:value={editTime} class="field" />
			</div>
		</div>

		<div class="space-y-2">
			<label for="edit-class" class="label-mono">Class</label>
			<select id="edit-class" bind:value={editClassId} class="field">
				<option value="">Unassigned</option>
				{#each classes as classItem (classItem.id)}
					<option value={classItem.id}>{classItem.name}</option>
				{/each}
			</select>
		</div>

		<div class="space-y-2">
			<label for="edit-reason" class="label-mono">Reason</label>
			<textarea
				id="edit-reason"
				rows="4"
				bind:value={editReason}
				placeholder="Example: corrected mistaken tap time..."
				class="textarea-field min-h-28 resize-none"
			></textarea>
		</div>

		<div class="flex justify-end gap-2 pt-2">
			<button
				type="button"
				disabled={isSavingEdit}
				onclick={() => (editTarget = null)}
				class="btn btn-secondary"
			>
				Cancel
			</button>
			<button
				type="button"
				disabled={isSavingEdit || editReason.trim().length < 3}
				onclick={saveAttendanceEdit}
				class="btn btn-primary"
			>
				{isSavingEdit ? 'Saving...' : 'Save edit'}
			</button>
		</div>
	{/if}
</Dialog>

<Dialog
	open={!!auditTarget}
	title="Audit history"
	description="Override, edit, and delete reasons for this learner."
	onClose={() => (auditTarget = null)}
>
	{#if auditTarget}
		<div class="rounded-lg border border-border bg-surface/60 p-3">
			<div class="text-[13px] font-medium">{auditTarget.studentName}</div>
			<div class="mt-1 font-mono text-xs text-muted-foreground">{auditTarget.date}</div>
		</div>

		{#if auditLoading}
			<div
				class="rounded-xl border border-dashed border-border p-6 text-center text-sm text-muted-foreground"
			>
				Loading audit history...
			</div>
		{:else if auditError}
			<div
				class="rounded-xl border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive"
			>
				{auditError}
			</div>
		{:else if auditEntries.length === 0}
			<div
				class="rounded-xl border border-dashed border-border p-6 text-center text-sm text-muted-foreground"
			>
				No audited changes for this learner yet.
			</div>
		{:else}
			<ul class="max-h-80 divide-y divide-border overflow-y-auto rounded-xl border border-border">
				{#each auditEntries as entry (entry.id)}
					<li class="px-4 py-3">
						<div class="flex flex-wrap items-center justify-between gap-2">
							<span class="chip capitalize">
								{entry.action.replace('_', ' ')}
							</span>
							<span class="font-mono text-xs text-muted-foreground">
								{fmtDate(entry.createdAt)}
								{fmtTime(entry.createdAt)}
							</span>
						</div>
						<p class="mt-2 text-sm leading-6">{entry.reason}</p>
					</li>
				{/each}
			</ul>
		{/if}
	{/if}
</Dialog>

<!-- ── Delete confirmation dialog ────────────────────────────────────────── -->
<Dialog
	open={!!deleteTarget}
	title="Delete attendance records?"
	description={deleteTarget
		? `${deleteTarget.studentName} · ${deleteTarget.date} will be permanently removed.`
		: ''}
	maxWidth="sm"
	onClose={cancelDelete}
>
	{#if deleteTarget}
		<div class="space-y-1.5">
			<label for="records-delete-reason" class="text-[11.5px] font-medium text-muted-foreground">
				Reason
			</label>
			<textarea
				id="records-delete-reason"
				rows="3"
				bind:value={deleteReason}
				placeholder="Required for the audit log"
				class="textarea-field min-h-24 resize-none"
			></textarea>
		</div>

		<div class="flex justify-end gap-2 border-t border-border pt-3">
			<button onclick={cancelDelete} class="btn btn-secondary">Cancel</button>
			<button
				disabled={deleteReason.trim().length < 3}
				onclick={confirmDeleteAttendanceRecords}
				class="btn btn-danger"
			>
				Delete records
			</button>
		</div>
	{/if}
</Dialog>
