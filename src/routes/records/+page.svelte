<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import DateRangePicker from '$lib/components/ui/DateRangePicker.svelte';
	import Pagination from '$lib/components/ui/Pagination.svelte';
	import StudentPicker from '$lib/components/ui/StudentPicker.svelte';
	import {
		listStudents,
		listEvents,
		listClasses,
		getSettings,
		deleteEvent,
		type Student,
		type AttendanceEvent,
		type Class
	} from '$lib/db-rust';
	import { page } from '$app/stores';
	import { fmtDate, fmtTime } from '$lib/csv';
	import { exportCsvWithFolder } from '$lib/db-rust';
	import DTRPreviewModal from '$lib/components/ui/DTRPreviewModal.svelte';

	// ── Types ────────────────────────────────────────────────────────────────
	type StudentAttendance = {
		studentId: string;
		studentName: string;
		studentNumber: string;
		className: string;
		date: string;
		checkInTime?: string;
		checkOutTime?: string;
		checkInTimestamp?: number;
		checkOutTimestamp?: number;
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

	// Export Modal
	let exportModalOpen = $state(false);
	let exportData = $state<{
		student: Student;
		classData?: Class;
		events: AttendanceEvent[];
		month: number;
		year: number;
	} | null>(null);

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
	let groupedAttendance = $derived(() => {
		const groups = new SvelteMap<string, StudentAttendance>();

		filtered.forEach((event) => {
			const student = studentMap.get(event.studentId);
			if (!student) return;

			const date = fmtDate(event.timestamp);
			const key = `${event.studentId}-${date}`;

			if (!groups.has(key)) {
				const className = getEventClassName(event);
				groups.set(key, {
					studentId: event.studentId,
					studentName: student.name,
					studentNumber: student.studentNumber,
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
			} else if (event.type === 'out') {
				if (!group.checkOutTime || event.timestamp > group.checkOutTime) {
					group.checkOutTime = fmtTime(event.timestamp);
					group.checkOutTimestamp = new Date(event.timestamp).getTime();
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
	const totalPages = $derived(Math.ceil(groupedAttendance().length / itemsPerPage));
	const paginatedFiltered = $derived(() => {
		const start = (currentPage - 1) * itemsPerPage;
		const end = start + itemsPerPage;
		return groupedAttendance().slice(start, end);
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

	async function reload() {
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
	}

	async function onExport() {
		try {
			const filePath = await exportCsvWithFolder(filtered, students, classes, lateAfter);
			toast(`CSV exported to: ${filePath}`);
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Export failed';
			toast(`Export failed: ${msg}`, false);
		}
	}

	function onExportExcel() {
		if (!studentId) {
			toast('Please select a student first', false);
			return;
		}

		const student = studentMap.get(studentId);
		if (!student) return;

		const classData = student.classId ? classMap.get(student.classId) : undefined;

		// Determine month/year from filters or current date
		const d = from ? new Date(from) : new Date();
		const month = d.getMonth() + 1;
		const year = d.getFullYear();

		// Filter events for this student and month
		const studentEvents = events.filter((e) => {
			if (e.studentId !== studentId) return false;
			const ed = new Date(e.timestamp);
			return ed.getMonth() + 1 === month && ed.getFullYear() === year;
		});

		exportData = {
			student,
			classData,
			events: studentEvents,
			month,
			year
		};
		exportModalOpen = true;
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

<AppShell>
	<div class="flex h-full flex-col overflow-hidden">
		<PageHeader
			category="Archives"
			title="Attendance Logs"
			description="Review and filter historical attendance data for your classes."
		>
			{#snippet actions()}
				<div class="flex items-center gap-2">
					<button
						onclick={onExport}
						class="inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
					>
						<svg
							class="size-4"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
							<polyline points="7 10 12 15 17 10" />
							<line x1="12" y1="15" x2="12" y2="3" />
						</svg>
						Export CSV
					</button>
					<button
						onclick={onExportExcel}
						class="inline-flex h-10 items-center gap-2 rounded-md border border-border bg-primary/10 px-4 py-2 text-sm font-medium text-primary transition-colors hover:bg-primary/20"
					>
						<svg
							class="size-4"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							aria-hidden="true"
						>
							<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
							<polyline points="7 10 12 15 17 10" />
							<line x1="12" y1="15" x2="12" y2="3" />
						</svg>
						Export DTR (Excel)
					</button>
				</div>
			{/snippet}
		</PageHeader>

		<!-- ── Filters ──────────────────────────────────────────────────────────── -->
		<section class="grid gap-4 px-6 py-8 sm:grid-cols-2 md:px-12 lg:grid-cols-4">
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
			</div>

			<!-- Student -->
			<div class="space-y-2">
				<div class="label-mono">Student</div>
				<StudentPicker
					{students}
					selectedId={studentId}
					{classId}
					placeholder="All students"
					on:select={(e) => (studentId = e.detail.id)}
				/>
			</div>

			<!-- Total -->
			<div class="space-y-2">
				<div class="label-mono">Total attendance records</div>
				<div class="flex h-10 items-center font-mono text-sm">{groupedAttendance().length}</div>
			</div>
		</section>

		<!-- ── Table ────────────────────────────────────────────────────────────── -->
		<section class="min-h-0 flex-1 px-6 pb-20 md:px-12" bind:clientHeight={availableHeight}>
			<div class="overflow-hidden rounded-2xl border border-border bg-card">
				<table class="w-full text-sm">
					<thead class="bg-surface text-left">
						<tr>
							<th class="label-mono px-4 py-3">Date</th>
							<th class="label-mono px-4 py-3">Student</th>
							<th class="label-mono px-4 py-3">Class</th>
							<th class="label-mono px-4 py-3">Check In</th>
							<th class="label-mono px-4 py-3">Check Out</th>
							<th class="label-mono w-20 px-4 py-3 text-right"> </th>
						</tr>
					</thead>
					<tbody class="divide-y divide-border">
						{#if groupedAttendance().length === 0}
							{@render emptyState()}
						{:else}
							{#each paginatedFiltered() as record (record.studentId + record.date)}
								<tr class="transition-colors hover:bg-surface/40">
									<td class="px-4 py-3 align-top font-mono">{record.date}</td>
									<td class="px-4 py-3 align-top">
										<div class="font-medium">{record.studentName}</div>
										<div class="label-mono">#{record.studentNumber}</div>
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
											<div class="flex items-center gap-2">
												{@render checkInPill(record.checkInTime, record.isLate)}
											</div>
										{:else}
											<span class="font-mono text-xs text-muted-foreground">—</span>
										{/if}
									</td>
									<td class="px-4 py-3 align-top">
										{#if record.checkOutTime}
											{@render checkOutPill(record.checkOutTime)}
										{:else}
											<span class="font-mono text-xs text-muted-foreground">—</span>
										{/if}
									</td>
									<td class="px-4 py-3 text-right align-top">
										{#if record.events.length > 0}
											<button
												onclick={() => {
													deleteTarget = {
														studentName: (record as StudentAttendance).studentName,
														date: (record as StudentAttendance).date,
														events: (record as StudentAttendance).events
													};
												}}
												aria-label="Delete attendance record"
												class="inline-flex size-8 items-center justify-center rounded-md border border-border text-destructive transition-colors hover:bg-destructive/10"
											>
												<svg
													class="size-3.5"
													viewBox="0 0 24 24"
													fill="none"
													stroke="currentColor"
													stroke-width="2"
													stroke-linecap="round"
													stroke-linejoin="round"
													aria-hidden="true"
												>
													<polyline points="3 6 5 6 21 6" />
													<path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6" />
													<path d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
												</svg>
											</button>
										{/if}
									</td>
								</tr>
							{/each}
						{/if}
					</tbody>
				</table>
			</div>
		</section>

		<div class="fixed right-6 bottom-6 z-30">
			<Pagination {currentPage} {totalPages} onPageChange={handlePageChange} />
		</div>
	</div>
</AppShell>

{#if toastMessage}
	<div
		class="fixed top-12 right-6 z-50 rounded-xl border px-4 py-3 text-sm font-medium shadow-lg
					{toastOk
			? 'border-border bg-background text-foreground'
			: 'border-destructive/40 bg-destructive/10 text-destructive'}"
		role="status"
		aria-live="polite"
	>
		{toastMessage}
	</div>
{/if}

<DateRangePicker
	open={dateRangePickerOpen}
	fromValue={from}
	toValue={to}
	on:close={() => (dateRangePickerOpen = false)}
	on:select={(e: CustomEvent<{ from: string; to: string }>) => {
		from = e.detail.from;
		to = e.detail.to;
	}}
/>

{#snippet emptyState()}
	<tr>
		<td colspan={6} class="px-4 py-12 text-center text-muted-foreground">
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

{#snippet checkOutPill(time: string)}
	<span
		class="rounded-pill border border-border bg-surface px-2 py-1 font-mono text-xs text-foreground"
	>
		{time}
	</span>
{/snippet}

<!-- ── Delete confirmation dialog ────────────────────────────────────────── -->
{#if deleteTarget}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => (deleteTarget = null)}
		onkeydown={(e) => e.key === 'Escape' && (deleteTarget = null)}
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
				<div>
					<h2 id="delete-dialog-title" class="text-lg font-semibold">Delete attendance records?</h2>
					<p class="mt-1 text-sm text-muted-foreground">
						<span class="font-medium text-foreground">{deleteTarget.studentName}</span> attendance
						on <span class="font-medium text-foreground">{deleteTarget.date}</span> will be permanently
						removed.
					</p>
				</div>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (deleteTarget = null)}
					class="flex-1 rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
				>
					Cancel
				</button>
				<button
					onclick={async () => {
						if (!deleteTarget) return;
						await Promise.all(deleteTarget.events.map((e: AttendanceEvent) => deleteEvent(e.id)));
						toast('Deleted');
						deleteTarget = null;
						reload();
					}}
					class="flex-1 rounded-pill bg-destructive px-4 py-2 text-sm font-medium text-white hover:opacity-90"
				>
					Delete
				</button>
			</div>
		</div>
	</div>
{/if}

{#if exportModalOpen && exportData}
	<DTRPreviewModal
		open={exportModalOpen}
		onClose={() => (exportModalOpen = false)}
		student={exportData.student}
		classData={exportData.classData}
		events={exportData.events}
		month={exportData.month}
		year={exportData.year}
	/>
{/if}
