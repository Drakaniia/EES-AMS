<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import RecordsFilters from './records-filters.svelte';
	import RecordsTable from './records-table.svelte';
	import RecordsExportDialog from './records-export-dialog.svelte';
	import {
		listStudents,
		listEvents,
		listClasses,
		getSettings,
		exportCsvWithFolder,
		type Student,
		type AttendanceEvent,
		type Class
	} from '$lib/db-rust';
	import { resolve } from '$app/paths';
	import { page } from '$app/stores';
	import { fmtDate, fmtTime } from '$lib/csv';
	import { CalendarDays, Download, FileSpreadsheet } from 'lucide-svelte';
	import {
		type StudentAttendance,
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

	let currentPage = $state(1);
	let recordsDialog = $state<RecordsExportDialog>();

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

	async function onExport() {
		if (exportingLogs) return;
		exportingLogs = true;
		try {
			const filePath = await exportCsvWithFolder(filtered, students, classes, lateAfter);
			recordsDialog?.showToast(`CSV exported to: ${filePath}`);
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Export failed';
			recordsDialog?.showToast(`Export failed: ${msg}`, false);
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
			onEdit={(r) => recordsDialog?.onEditAttendanceRecord(r)}
			onAudit={(r) => recordsDialog?.openAudit(r)}
			onDelete={(e, r) => recordsDialog?.onDeleteAttendanceRecord(e, r)}
			onPageChange={handlePageChange}
		/>

		<RecordsExportDialog bind:this={recordsDialog} {classes} onRecordChanged={reload} />
	{/if}
</div>
