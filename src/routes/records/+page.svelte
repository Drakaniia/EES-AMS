<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import DateRangePicker from '$lib/components/ui/DateRangePicker.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import Pagination from '$lib/components/ui/Pagination.svelte';
	import StudentPicker from '$lib/components/ui/StudentPicker.svelte';
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
	import { CalendarDays, Download, FileSpreadsheet, History, Pencil, Trash2 } from 'lucide-svelte';

	// ── Types ────────────────────────────────────────────────────────────────
	type StudentAttendance = {
		studentId: string;
		studentName: string;
		classId?: string;
		className: string;
		date: string;
		checkInTime?: string;
		checkInTimestamp?: number;
		isLate?: boolean;
		events: AttendanceEvent[];
	};
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

	// Date range picker dialog state
	let dateRangePickerOpen = $state(false);

	// Toast
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	// Delete confirmation dialog
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

	// Pagination
	let currentPage = $state(1);
	let availableHeight = $state(0);
	const itemsPerPage = $derived.by(() => {
		if (availableHeight === 0) return 10;
		// availableHeight is bound to the <section> which has pb-20 (80px).
		// The table container has overflow-hidden rounded-2xl border.
		// We need a conservative buffer to ensure no row is partially covered.
		const rowHeight = 60; // Safer estimate for row height
		const headerHeight = 48; // Table header height
		const verticalBuffer = 120; // Accounts for pb-20 (80px) and table margins
		const calculated = Math.floor((availableHeight - headerHeight - verticalBuffer) / rowHeight);
		return Math.max(1, calculated);
	});

	$effect(() => {
		if (currentPage > totalPages && totalPages > 0) {
			currentPage = totalPages;
		}
	});

	// ── Derived ──────────────────────────────────────────────────────────────
	let studentMap = $derived(new Map(students.map((s) => [s.id, s])));
	let classMap = $derived(new Map(classes.map((c) => [c.id, c])));

	// Group events by student and date
	let groupedAttendance = $derived.by(() => {
		const groups = new SvelteMap<string, StudentAttendance>();

		filtered.forEach((event) => {
			const student = studentMap.get(event.studentId);
			if (!student) return;

			const date = fmtDate(event.timestamp);
			const key = `${event.studentId}-${date}`;

			if (!groups.has(key)) {
				const className = getEventClassName(event);
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

					// Check if late
					const studentClass = classes.find((c) => c.id === student.classId);
					if (studentClass) {
						const eventTime = new Date(event.timestamp);
						const timeStr = `${String(eventTime.getHours()).padStart(2, '0')}:${String(eventTime.getMinutes()).padStart(2, '0')}`;

						let lateAfter = studentClass.lateAfter;
						if (studentClass.sessions && studentClass.sessions.length > 0) {
							for (const session of studentClass.sessions) {
								if (timeStr >= session.startTime && timeStr <= session.endTime) {
									lateAfter = session.lateAfter;
									break;
								}
							}
						}

						if (lateAfter) {
							const [h, m] = lateAfter.split(':').map(Number);
							const lateTime = new Date(
								eventTime.getFullYear(),
								eventTime.getMonth(),
								eventTime.getDate(),
								h,
								m,
								0,
								0
							);
							group.isLate = eventTime > lateTime;
						}
					}
				}
			}
		});

		return Array.from(groups.values()).sort((a: StudentAttendance, b: StudentAttendance) => {
			// Sort by date descending, then by student name
			if (a.date !== b.date) {
				return new Date(b.date).getTime() - new Date(a.date).getTime();
			}
			return a.studentName.localeCompare(b.studentName);
		});
	});

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

	// Pagination for records
	const totalPages = $derived(Math.ceil(groupedAttendance.length / itemsPerPage));
	const paginatedFiltered = $derived.by(() => {
		const start = (currentPage - 1) * itemsPerPage;
		const end = start + itemsPerPage;
		return groupedAttendance.slice(start, end);
	});

	function handlePageChange(page: number) {
		currentPage = page;
	}

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	function eventTime(event: AttendanceEvent) {
		return new Date(event.timestamp).getTime();
	}

	function primaryEvent(record: StudentAttendance) {
		return [...record.events].sort((a, b) => eventTime(a) - eventTime(b))[0];
	}

	function sessionSegment(classObj: Class | undefined, timestamp: Date) {
		if (!classObj?.sessions || classObj.sessions.length <= 1) return 'day';

		const timeStr = `${String(timestamp.getHours()).padStart(2, '0')}:${String(
			timestamp.getMinutes()
		).padStart(2, '0')}`;
		const session = classObj.sessions.find(
			(item) => timeStr >= item.startTime && timeStr <= item.endTime
		);

		return (session?.name || 'off-schedule')
			.trim()
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-|-$/g, '');
	}

	function sessionKeyFor(classId: string, timestamp: Date) {
		const classObj = classMap.get(classId);
		const classKey = classId || 'unassigned';
		const segment = sessionSegment(classObj, timestamp) || 'day';
		return `${fmtDate(timestamp.getTime())}|${classKey}|${segment}`;
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

	async function onDeleteAttendanceRecord(_event: MouseEvent, record: StudentAttendance) {
		const target = {
			studentName: record.studentName,
			date: record.date,
			events: record.events
		};

		deleteTarget = target;
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
				sessionKey: sessionKeyFor(editClassId, timestamp),
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

	function getEventClassName(e: AttendanceEvent) {
		const id = e.classId || studentMap.get(e.studentId)?.classId;
		if (!id) return '—';
		return classMap.get(id)?.name ?? 'Unknown';
	}

	// ── Lifecycle ────────────────────────────────────────────────────────────
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

	<!-- ── Filters ──────────────────────────────────────────────────────────── -->
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
		<section class="grid gap-4 px-4 py-5 sm:grid-cols-2 md:px-8 lg:grid-cols-4 lg:px-10">
			<!-- Date Range -->
			<div class="space-y-2">
				<div class="label-mono">Date Range</div>
				<button
					onclick={() => (dateRangePickerOpen = true)}
					class="flex h-10 w-full items-center justify-between rounded-md border border-border bg-background px-3 text-left text-sm transition-colors hover:bg-surface focus:ring-2 focus:ring-primary focus:outline-none"
				>
					<span class={from || to ? '' : 'text-muted-foreground'}>
						{from && to
							? `${new Date(from).toLocaleDateString()} - ${new Date(to).toLocaleDateString()}`
							: from
								? `From ${new Date(from).toLocaleDateString()}`
								: to
									? `To ${new Date(to).toLocaleDateString()}`
									: 'Select date range'}
					</span>
					<svg
						class="size-4 text-muted-foreground"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
						<line x1="16" y1="2" x2="16" y2="6"></line>
						<line x1="8" y1="2" x2="8" y2="6"></line>
						<line x1="3" y1="10" x2="21" y2="10"></line>
					</svg>
				</button>
			</div>

			<!-- Class -->
			<div class="space-y-2">
				<div class="label-mono">Class</div>
				{#if classes.length <= 1}
					<div
						class="flex h-10 items-center rounded-md border border-border bg-surface px-3 text-sm font-medium"
					>
						{classes[0]?.name ?? 'No class configured'}
					</div>
				{:else}
					<div class="relative">
						<select
							bind:value={classId}
							onchange={() => (studentId = '')}
							class="h-10 w-full appearance-none rounded-md border border-border bg-background px-3 pr-10 text-sm transition-colors hover:bg-surface focus:ring-2 focus:ring-primary focus:outline-none"
						>
							<option value="">All classes</option>
							{#each classes as c (c.id)}
								<option value={c.id}>{c.name}</option>
							{/each}
						</select>
						<div
							class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-muted-foreground"
						>
							<svg
								class="size-4"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								<path d="m6 9 6 6 6-6" />
							</svg>
						</div>
					</div>
				{/if}
			</div>

			<!-- Student -->
			<div class="space-y-2">
				<div class="label-mono">Student</div>
				<StudentPicker
					{students}
					selectedId={studentId}
					{classId}
					placeholder="All students"
					onSelect={({ id }) => (studentId = id)}
				/>
			</div>

			<!-- Total -->
			<div class="space-y-2">
				<div class="label-mono">Total attendance records</div>
				<div class="flex h-10 items-center font-mono text-sm">{groupedAttendance.length}</div>
			</div>
		</section>

		<!-- ── Table ────────────────────────────────────────────────────────────── -->
		<section class="min-h-0 flex-1 px-4 pb-20 md:px-8 lg:px-10" bind:clientHeight={availableHeight}>
			<div class="table-wrap">
				<table class="min-w-[720px] text-sm">
					<thead class="bg-surface text-left">
						<tr>
							<th class="label-mono px-4 py-3">Date</th>
							<th class="label-mono px-4 py-3">Student</th>
							<th class="label-mono px-4 py-3">Class</th>
							<th class="label-mono px-4 py-3">Check In</th>
							<th class="label-mono w-36 px-4 py-3 text-right">Actions</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-border">
						{#if groupedAttendance.length === 0}
							{@render emptyState()}
						{:else}
							{#each paginatedFiltered as record (record.studentId + record.date)}
								<tr class="transition-colors hover:bg-surface/40">
									<td class="px-4 py-3 align-top font-mono">{record.date}</td>
									<td class="px-4 py-3 align-top">
										<div class="text-balance-safe font-medium">{record.studentName}</div>
									</td>
									<td class="px-4 py-3 align-top">
										<span
											class="rounded-pill border border-border bg-surface px-2 py-0.5 text-[10px]"
										>
											{record.className}
										</span>
									</td>
									<td class="px-4 py-3 align-top">
										{#if record.checkInTime}
											<div class="flex flex-col items-start gap-1">
												{@render checkInPill(record.checkInTime, record.isLate)}
												{#if primaryEvent(record)?.overrideReason}
													<span class="max-w-56 text-xs leading-5 text-muted-foreground">
														{primaryEvent(record)?.overrideReason}
													</span>
												{/if}
											</div>
										{:else}
											<span class="font-mono text-xs text-muted-foreground">—</span>
										{/if}
									</td>
									<td class="px-4 py-3 text-right align-top">
										{#if record.events.length > 0}
											<div class="inline-flex items-center gap-1">
												<button
													type="button"
													onclick={() => onEditAttendanceRecord(record)}
													aria-label="Edit attendance record"
													class="inline-flex size-8 items-center justify-center rounded-md border border-border text-primary transition-colors hover:bg-primary/10"
												>
													<Pencil class="size-3.5" aria-hidden="true" />
												</button>
												<button
													type="button"
													onclick={() => openAudit(record)}
													aria-label="View audit history"
													class="inline-flex size-8 items-center justify-center rounded-md border border-border text-muted-foreground transition-colors hover:bg-surface"
												>
													<History class="size-3.5" aria-hidden="true" />
												</button>
												<button
													type="button"
													onclick={(event) => onDeleteAttendanceRecord(event, record)}
													aria-label="Delete attendance record"
													class="inline-flex size-8 items-center justify-center rounded-md border border-border text-destructive transition-colors hover:bg-destructive/10"
												>
													<Trash2 class="size-3.5" aria-hidden="true" />
												</button>
											</div>
										{/if}
									</td>
								</tr>
							{/each}
						{/if}
					</tbody>
				</table>
			</div>
		</section>

		<div class="fixed bottom-6 left-1/2 z-30 -translate-x-1/2">
			<Pagination {currentPage} {totalPages} onPageChange={handlePageChange} />
		</div>
	{/if}
</div>

<FeedbackToast message={toastMessage} ok={toastOk} onClose={() => (toastMessage = null)} />

<DateRangePicker
	open={dateRangePickerOpen}
	fromValue={from}
	toValue={to}
	onClose={() => (dateRangePickerOpen = false)}
	onSelect={(range) => {
		from = range.from;
		to = range.to;
	}}
/>

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

{#snippet emptyState()}
	<tr>
		<td colspan={5} class="px-4 py-12 text-center text-muted-foreground">
			No attendance records match the filters.
		</td>
	</tr>
{/snippet}

{#snippet checkInPill(time: string, isLate?: boolean)}
	<span
		class="rounded-pill px-2 py-1 font-mono text-xs
					{isLate ? 'bg-destructive text-destructive-foreground' : 'bg-primary text-primary-foreground'}"
	>
		{time}
		{#if isLate}
			(LATE){/if}
	</span>
{/snippet}

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
			class="w-full max-w-sm space-y-5 rounded-2xl border border-border bg-background p-6 shadow-xl"
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
