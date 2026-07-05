<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import RecordsFilters from './records-filters.svelte';
	import RecordsTable from './records-table.svelte';
	import {
		listStudents,
		listEvents,
		listClasses,
		getSettings,
		deleteEvent,
		updateEvent,
		listAttendanceAudit,
		exportCsvWithFolder,
		type Student,
		type AttendanceEvent,
		type AttendanceAuditEntry,
		type Class
	} from '$lib/db-rust';
	import { resolve } from '$app/paths';
	import { page } from '$app/stores';
	import { fmtDate, fmtTime } from '$lib/csv';
	import { CalendarDays, Download, FileSpreadsheet } from 'lucide-svelte';
	import {
		type StudentAttendance,
		eventTime,
		primaryEvent,
		sessionKeyFor,
		getEventClassName,
		checkIsLate,
	} from './records-state.svelte';

	let students = $state<Student[]>([]);
	let events = $state<AttendanceEvent[]>([]);
	let classes = $state<Class[]>([]);
	let from = $state('');
	let to = $state('');
	let studentId = $state($page.url.searchParams.get('studentId') || '');
	let classId = $state($page.url.searchParams.get('classId') || '');
	let lateAfter = $state('08:45');
	let exportingLogs = $state(false);
	let loading = $state(true);
	let loadError = $state<string | null>(null);

	let dateRangePickerOpen = $state(false);

	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	let deleteTarget = $state<{
		studentName: string;
		date: string;
		events: AttendanceEvent[];
	} | null>(null);
	let deleteReason = $state('');

	let editTarget = $state<StudentAttendance | null>(null);
	let editDate = $state('');
	let editTime = $state('');
	let editClassId = $state('');
	let editReason = $state('');
	let isSavingEdit = $state(false);

	let auditTarget = $state<StudentAttendance | null>(null);
	let auditEntries = $state<AttendanceAuditEntry[]>([]);
	let auditLoading = $state(false);
	let auditError = $state<string | null>(null);

	let currentPage = $state(1);

	$effect(() => {
		if (currentPage > totalPages && totalPages > 0) {
			currentPage = totalPages;
		}
	});

	let studentMap = $derived(new Map(students.map((s) => [s.id, s])));
	let classMap = $derived(new Map(classes.map((c) => [c.id, c])));

	let filtered = $derived(
		events.filter((e) => {
			const d = fmtDate(e.timestamp);
			if (from && d < from) return false;
			if (to && d > to) return false;
			if (studentId && e.studentId !== studentId) return false;

			if (classId) {
				const s = studentMap.get(e.studentId);
				const eventClassId = e.classId || s?.classId;
				if (eventClassId !== classId) return false;
			}

			return true;
		})
	);

	let groupedAttendance = $derived.by(() => {
		const groups = new SvelteMap<string, StudentAttendance>();

		filtered.forEach((event) => {
			const student = studentMap.get(event.studentId);
			if (!student) return;

			const date = fmtDate(event.timestamp);
			const key = `${event.studentId}-${date}`;

			if (!groups.has(key)) {
				const className = getEventClassName(event, classMap, studentMap);
				const eventClassId = event.classId || student.classId;
				groups.set(key, {
					studentId: event.studentId,
					studentName: student.name,
					classId: eventClassId,
					className,
					date,
					events: []
				});
			}

			const group = groups.get(key)!;
			group.events.push(event);

			if (event.type === 'in') {
				if (!group.checkInTime || event.timestamp < group.checkInTime) {
					group.checkInTime = fmtTime(event.timestamp);
					group.checkInTimestamp = new Date(event.timestamp).getTime();
					group.isLate = checkIsLate(event, student, classes);
				}
			}
		});

		return Array.from(groups.values()).sort((a, b) => {
			if (a.date !== b.date) {
				return new Date(b.date).getTime() - new Date(a.date).getTime();
			}
			return a.studentName.localeCompare(b.studentName);
		});
	});

	const totalPages = $derived(Math.ceil(groupedAttendance.length / 10));
	const paginatedFiltered = $derived.by(() => {
		const start = (currentPage - 1) * 10;
		return groupedAttendance.slice(start, start + 10);
	});

	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	async function reload() {
		loading = true;
		loadError = null;
		try {
			const [s, e, c, st] = await Promise.all([
				listStudents(),
				listEvents(),
				listClasses(),
				getSettings()
			]);
			students = s;
			events = e;
			classes = c;
			lateAfter = st.lateAfter;
			currentPage = 1;
		} catch (error) {
			loadError =
				error instanceof Error ? error.message : 'Attendance records could not be loaded.';
		} finally {
			loading = false;
		}
	}

	async function confirmDeleteAttendanceRecords(target = deleteTarget) {
		if (!target) return;
		const reason = deleteReason.trim();
		if (reason.length < 3) {
			toast('Delete reason is required', false);
			return;
		}
		await Promise.all(target.events.map((event: AttendanceEvent) => deleteEvent(event.id, reason)));
		toast('Deleted');
		deleteTarget = null;
		deleteReason = '';
		reload();
	}

	function onDeleteAttendanceRecord(_event: MouseEvent, record: StudentAttendance) {
		deleteTarget = {
			studentName: record.studentName,
			date: record.date,
			events: record.events
		};
	}

	function onEditAttendanceRecord(record: StudentAttendance) {
		const event = primaryEvent(record);
		if (!event) return;

		editTarget = record;
		editDate = record.date;
		editTime = record.checkInTime || fmtTime(event.timestamp);
		editClassId =
			event.classId || record.classId || studentMap.get(record.studentId)?.classId || '';
		editReason = '';
	}

	async function saveAttendanceEdit() {
		if (!editTarget || isSavingEdit) return;
		const event = primaryEvent(editTarget);
		if (!event) return;

		const reason = editReason.trim();
		if (reason.length < 3) {
			toast('Edit reason is required', false);
			return;
		}

		const timestamp = new Date(`${editDate}T${editTime}:00`);
		if (Number.isNaN(timestamp.getTime())) {
			toast('Enter a valid date and time', false);
			return;
		}

		isSavingEdit = true;
		try {
			await updateEvent(event.id, {
				classId: editClassId || undefined,
				timestamp: timestamp.toISOString(),
				sessionKey: sessionKeyFor(editClassId, timestamp, classMap),
				reason
			});
			toast('Record updated');
			editTarget = null;
			await reload();
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Update failed';
			toast(`Update failed: ${msg}`, false);
		} finally {
			isSavingEdit = false;
		}
	}

	async function openAudit(record: StudentAttendance) {
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

	async function onExport() {
		if (exportingLogs) return;
		exportingLogs = true;
		try {
			const filePath = await exportCsvWithFolder(filtered, students, classes, lateAfter);
			toast(`CSV exported to: ${filePath}`);
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Export failed';
			toast(`Export failed: ${msg}`, false);
		} finally {
			exportingLogs = false;
		}
	}

	function handlePageChange(page: number) {
		currentPage = page;
	}

	function handleDateRangeChange(range: { from: string; to: string }) {
		from = range.from;
		to = range.to;
		dateRangePickerOpen = false;
	}

	onMount(() => {
		reload();
	});
</script>

<svelte:head>
	<title>Records — Attendance System</title>
	<meta name="description" content="Filter, review, and export attendance records as CSV." />
</svelte:head>

<div class="flex h-full flex-col overflow-hidden">
	<PageHeader
		category="Archives"
		title="Attendance Logs"
		description="Review and filter historical attendance data for your classes."
	>
		{#snippet actions()}
			<div class="flex flex-wrap items-center gap-2">
				<a
					href={resolve('/reports')}
					class="inline-flex h-10 items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
				>
					<FileSpreadsheet class="size-4" />
					SF2 Workbook
				</a>
				<a
					href={resolve('/records/day-overview')}
					class="inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
				>
					<CalendarDays class="size-4" />
					Daily Overview
				</a>
				<button
					onclick={onExport}
					disabled={exportingLogs}
					class="inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				>
					<Download class="size-4" />
					{exportingLogs ? 'Exporting...' : 'Export Logs CSV'}
				</button>
			</div>
		{/snippet}
	</PageHeader>

	{#if loading}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<LoadingBlock rows={4} label="Loading attendance records" />
		</div>
	{:else if loadError}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<EmptyState tone="warning" title="Records are unavailable" description={loadError}>
				{#snippet actions()}
					<button
						type="button"
						onclick={reload}
						class="control-ring rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface"
					>
						Retry
					</button>
				{/snippet}
			</EmptyState>
		</div>
	{:else}
		<RecordsFilters
			{from}
			{to}
			{classId}
			{studentId}
			{classes}
			{students}
			recordCount={groupedAttendance.length}
			{dateRangePickerOpen}
			onDateRangeChange={handleDateRangeChange}
			onClassChange={(value) => {
				classId = value;
				studentId = '';
			}}
			onStudentChange={(id) => (studentId = id)}
			onDateRangePickerOpen={() => (dateRangePickerOpen = true)}
			onDateRangePickerClose={() => (dateRangePickerOpen = false)}
		/>

		<RecordsTable
			paginatedRecords={paginatedFiltered}
			{groupedAttendance}
			{currentPage}
			{totalPages}
			onEdit={onEditAttendanceRecord}
			onAudit={openAudit}
			onDelete={onDeleteAttendanceRecord}
			onPageChange={handlePageChange}
		/>
	{/if}
</div>

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
				<input
					id="edit-date"
					type="date"
					bind:value={editDate}
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
			<div class="space-y-2">
				<label for="edit-time" class="label-mono">Time</label>
				<input
					id="edit-time"
					type="time"
					bind:value={editTime}
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
		</div>

		<div class="space-y-2">
			<label for="edit-class" class="label-mono">Class</label>
			<select
				id="edit-class"
				bind:value={editClassId}
				class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			>
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
				class="min-h-28 w-full resize-none rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			></textarea>
		</div>

		<div class="flex justify-end gap-2 pt-2">
			<button
				type="button"
				disabled={isSavingEdit}
				onclick={() => (editTarget = null)}
				class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				Cancel
			</button>
			<button
				type="button"
				disabled={isSavingEdit || editReason.trim().length < 3}
				onclick={saveAttendanceEdit}
				class="rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
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
		<div class="rounded-xl border border-border bg-surface/60 p-3">
			<div class="font-medium">{auditTarget.studentName}</div>
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
							<span
								class="label-mono rounded-pill border border-border bg-surface px-2 py-1 text-[10px]"
							>
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
{#if deleteTarget}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => {
			deleteTarget = null;
			deleteReason = '';
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape') {
				deleteTarget = null;
				deleteReason = '';
			}
		}}
	></div>

	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="delete-dialog-title"
	>
		<div
			class="w-full max-w-sm space-y-5 rounded-2xl border border-border bg-background p-6"
		>
			<div class="flex flex-col items-center gap-3 text-center">
				<div class="flex size-12 items-center justify-center rounded-full bg-destructive/10">
					<svg
						class="size-6 text-destructive"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="3 6 5 6 21 6" />
						<path
							d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"
						/>
					</svg>
				</div>
				<div class="w-full text-left">
					<h2 id="delete-dialog-title" class="text-lg font-semibold">Delete attendance records?</h2>
					<p class="mt-1 text-sm text-muted-foreground">
						<span class="font-medium text-foreground">{deleteTarget.studentName}</span> attendance
						on <span class="font-medium text-foreground">{deleteTarget.date}</span> will be permanently
						removed.
					</p>
				</div>
			</div>

			<div class="space-y-2">
				<label for="delete-reason" class="label-mono">Reason</label>
				<textarea
					id="delete-reason"
					rows="3"
					bind:value={deleteReason}
					placeholder="Example: duplicate mistaken tap..."
					class="min-h-24 w-full resize-none rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				></textarea>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => {
						deleteTarget = null;
						deleteReason = '';
					}}
					class="flex-1 rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
				>
					Cancel
				</button>
				<button
					disabled={deleteReason.trim().length < 3}
					onclick={() => confirmDeleteAttendanceRecords()}
					class="flex-1 rounded-pill bg-destructive px-4 py-2 text-sm font-medium text-white hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
				>
					Delete
				</button>
			</div>
		</div>
	</div>
{/if}
