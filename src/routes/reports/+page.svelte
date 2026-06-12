<script lang="ts">
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import TaskProgress from '$lib/components/ui/TaskProgress.svelte';
	import {
		exportSf2Workbook,
		getSf2ExportPreview,
		getSf2WorkbookSettings,
		listClasses,
		openSf2Workbook,
		setSf2PreviewAttendance,
		updateSf2WorkbookSettings,
		type Class,
		type Sf2ExportPreview,
		type Sf2PreviewCell,
		type Sf2PreviewStudentRow,
		type Sf2TemplateDraft,
		type Sf2WorkbookSettings
	} from '$lib/db-rust';
	import type { Sf2PreviewDate } from '$lib/types';
	import {
		SF2_SCHOOL_MONTHS,
		defaultSf2FirstSchoolDay,
		sf2MonthByValue,
		sf2ReportMonthLabel
	} from '$lib/features/settings/sf2-workbook';
	import {
		AlertTriangle,
		ArrowLeft,
		CalendarDays,
		Check,
		CircleAlert,
		ExternalLink,
		FileCheck2,
		Maximize2,
		RefreshCw,
		Save,
		Settings2,
		UserX,
		X
	} from 'lucide-svelte';

	let classes = $state<Class[]>([]);
	let selectedClassId = $state('');
	let preview = $state<Sf2ExportPreview | null>(null);
	let workbookSettings = $state<Sf2WorkbookSettings | null>(null);
	let loading = $state(true);
	let loadError = $state<string | null>(null);
	let exporting = $state(false);
	let opening = $state(false);
	let savingDetails = $state(false);
	let correctingCellKey = $state<string | null>(null);
	let exportDialogOpen = $state(false);
	let exportLoadingOpen = $state(false);
	let fullReviewOpen = $state(false);
	let fullReviewHeaderVisible = $state(false);
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	let draftSchoolId = $state('');
	let draftSchoolName = $state('');
	let draftSchoolYear = $state('');
	let draftReportMonth = $state('');
	let draftGradeLevel = $state('');
	let draftSection = $state('');
	let draftAdviserName = $state('');
	let draftSchoolHeadName = $state('');

	const MATRIX_WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri'] as const;

	type MatrixWeekday = (typeof MATRIX_WEEKDAYS)[number];

	type MatrixDateSlot = {
		key: string;
		weekday: MatrixWeekday;
		date: Sf2PreviewDate | null;
		dateKey: string | null;
	};

	type MatrixWeekGroup = {
		key: string;
		label: string;
		slots: MatrixDateSlot[];
	};

	type MatrixStudentRow = Sf2PreviewStudentRow & {
		cellsByDate: Map<string, Sf2PreviewCell>;
	};

	onMount(async () => {
		await loadInitial();
	});

	const activeClassId = $derived(
		selectedClassId || preview?.classId || preview?.template?.classId || ''
	);
	const selectedClass = $derived(classes.find((item) => item.id === activeClassId));
	const exportDisabled = $derived(
		!preview?.canExport || exporting || savingDetails || !activeClassId
	);
	const issueCount = $derived(preview?.issues.length ?? 0);
	const warningCount = $derived(preview?.warnings.length ?? 0);
	const activeReportMonth = $derived(draftReportMonth || preview?.template?.reportMonth || '');
	const matrixWeekGroups = $derived(buildMatrixWeekGroups(preview?.dates ?? [], activeReportMonth));
	const matrixStudents = $derived.by((): MatrixStudentRow[] =>
		(preview?.students ?? []).map((row) => ({
			...row,
			cellsByDate: new Map(row.cells.map((cell) => [cell.date, cell]))
		}))
	);

	async function loadInitial() {
		loading = true;
		loadError = null;
		try {
			classes = await listClasses();
			const current = await getSf2ExportPreview();
			preview = current;
			selectedClassId = current.classId ?? classes[0]?.id ?? '';

			if (selectedClassId && selectedClassId !== current.classId) {
				await loadReport(selectedClassId);
			} else {
				await loadWorkbookSettings(current.classId);
			}
		} catch (error) {
			const msg = errorMessage(error, 'Failed to load reports');
			loadError = msg;
			toast(`Reports failed: ${msg}`, false);
		} finally {
			loading = false;
		}
	}

	async function loadReport(classId?: string) {
		const nextPreview = await getSf2ExportPreview(classId);
		preview = nextPreview;
		if (nextPreview.classId) selectedClassId = nextPreview.classId;
		await loadWorkbookSettings(nextPreview.classId ?? classId);
	}

	async function loadWorkbookSettings(classId?: string) {
		if (!classId) {
			workbookSettings = null;
			clearDraft();
			return;
		}

		try {
			const settings = await getSf2WorkbookSettings(classId);
			workbookSettings = settings;
			hydrateDraft(settings);
		} catch {
			workbookSettings = null;
			clearDraft();
		}
	}

	async function onOpenSf2() {
		if (!activeClassId || !preview?.template || opening) return;
		opening = true;
		try {
			const path = await openSf2Workbook(activeClassId);
			toast(`Opened SF2 working copy: ${path}`);
		} catch (error) {
			const msg = errorMessage(error, 'Failed to open SF2');
			toast(`Open SF2 failed: ${msg}`, false);
		} finally {
			opening = false;
		}
	}

	async function requestExport() {
		if (exportDisabled) return;

		const missingFields = blankSf2HeaderFields();
		if (missingFields.length > 0) {
			toast(
				`Fill required SF2 header fields before exporting: ${missingFields.join(', ')}.`,
				false
			);
			return;
		}

		if (hasWorkbookDraftChanges()) {
			const saved = await saveWorkbookDetails(null);
			if (!saved) return;
		}

		exportDialogOpen = true;
	}

	async function confirmExport() {
		if (!activeClassId || !preview?.canExport || exporting) return;
		exportDialogOpen = false;
		exporting = true;
		exportLoadingOpen = true;
		try {
			const result = await exportSf2Workbook(activeClassId);
			toast(`SF2 exported and opened: ${result.outputPath}`);
			await loadReport(activeClassId);
		} catch (error) {
			const msg = errorMessage(error, 'SF2 export failed');
			toast(`SF2 export failed: ${msg}`, false);
		} finally {
			exporting = false;
			exportLoadingOpen = false;
		}
	}

	async function saveWorkbookDetails(successMessage: string | null = 'SF2 workbook details saved') {
		const draft = workbookDraftPayload();
		if (!draft || savingDetails) return false;

		savingDetails = true;
		try {
			await updateSf2WorkbookSettings(draft);
			if (successMessage) toast(successMessage);
			await loadReport(draft.classId);
			return true;
		} catch (error) {
			const msg = errorMessage(error, 'SF2 workbook update failed');
			toast(`SF2 workbook update failed: ${msg}`, false);
			return false;
		} finally {
			savingDetails = false;
		}
	}

	async function onReportMonthChange() {
		const previousReportMonth =
			workbookSettings?.reportMonth || preview?.template?.reportMonth || '';
		const saved = await saveWorkbookDetails('SF2 report month updated');
		if (!saved) draftReportMonth = previousReportMonth;
	}

	function onWindowKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && fullReviewOpen) {
			fullReviewOpen = false;
		}
	}

	async function toggleAttendance(row: Sf2PreviewStudentRow, cell: Sf2PreviewCell) {
		if (!preview?.classId || !row.mapped || !cell.editable || correctingCellKey) return;
		const key = cellKey(row.studentId, cell.date);
		const markPresent = cell.status !== 'present';
		correctingCellKey = key;
		try {
			preview = await setSf2PreviewAttendance(
				preview.classId,
				row.studentId,
				cell.date,
				markPresent
			);
			toast(
				`${row.studentName} marked ${markPresent ? 'present' : 'absent'} for ${formatDate(cell.date)}`
			);
		} catch (error) {
			const msg = errorMessage(error, 'Attendance correction failed');
			toast(`Attendance correction failed: ${msg}`, false);
		} finally {
			correctingCellKey = null;
		}
	}

	function blankSf2HeaderFields() {
		return sf2HeaderFields()
			.filter((field) => field.value.trim() === '')
			.map((field) => field.label);
	}

	function sf2HeaderFields() {
		return [
			{ label: 'School ID', value: draftSchoolId },
			{ label: 'Name of School', value: draftSchoolName },
			{ label: 'School Year', value: draftSchoolYear },
			{ label: 'Report Month', value: draftReportMonth },
			{ label: 'Grade Level', value: draftGradeLevel },
			{ label: 'Section', value: draftSection },
			{ label: 'Adviser / LIS Name', value: draftAdviserName },
			{ label: 'School Head Name', value: draftSchoolHeadName }
		];
	}

	function hasWorkbookDraftChanges() {
		if (!workbookSettings) return false;
		return (
			draftSchoolId !== workbookSettings.schoolId ||
			draftSchoolName !== workbookSettings.schoolName ||
			draftSchoolYear !== workbookSettings.schoolYear ||
			draftReportMonth !== workbookSettings.reportMonth ||
			draftGradeLevel !== workbookSettings.gradeLevel ||
			draftSection !== workbookSettings.section ||
			draftAdviserName !== workbookSettings.adviserName ||
			draftSchoolHeadName !== workbookSettings.schoolHeadName
		);
	}

	function workbookDraftPayload(): Sf2TemplateDraft | null {
		if (!workbookSettings || !activeClassId) return null;
		return {
			classId: activeClassId,
			schoolId: draftSchoolId,
			schoolName: draftSchoolName,
			schoolYear: draftSchoolYear,
			reportMonth: draftReportMonth,
			gradeLevel: draftGradeLevel,
			section: draftSection,
			adviserName: draftAdviserName,
			schoolHeadName: draftSchoolHeadName,
			firstSchoolDay: defaultSf2FirstSchoolDay(draftReportMonth, draftSchoolYear),
			learnerNames: workbookSettings.learnerNames
		};
	}

	function hydrateDraft(settings: Sf2WorkbookSettings) {
		draftSchoolId = settings.schoolId;
		draftSchoolName = settings.schoolName;
		draftSchoolYear = settings.schoolYear;
		draftReportMonth = settings.reportMonth;
		draftGradeLevel = settings.gradeLevel;
		draftSection = settings.section;
		draftAdviserName = settings.adviserName;
		draftSchoolHeadName = settings.schoolHeadName;
	}

	function clearDraft() {
		draftSchoolId = '';
		draftSchoolName = '';
		draftSchoolYear = '';
		draftReportMonth = '';
		draftGradeLevel = '';
		draftSection = '';
		draftAdviserName = '';
		draftSchoolHeadName = '';
	}

	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 4000);
	}

	function errorMessage(error: unknown, fallback: string) {
		if (error instanceof Error) return error.message;
		if (typeof error === 'string') return error;
		return fallback;
	}

	function formatDate(date: string) {
		const value = new Date(`${date}T00:00:00`);
		return value.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}

	function formatWeekday(date: string) {
		const value = new Date(`${date}T00:00:00`);
		return value.toLocaleDateString(undefined, { weekday: 'short' });
	}

	function formatDayNumber(date: string) {
		const value = new Date(`${date}T00:00:00`);
		return String(value.getDate());
	}

	function matrixDateLabel(date: string) {
		return `${formatWeekday(date)} ${formatDayNumber(date)}`;
	}

	function buildMatrixWeekGroups(dates: Sf2PreviewDate[], reportMonth: string): MatrixWeekGroup[] {
		const groups: MatrixWeekGroup[] = [];
		const month = sf2MonthByValue(reportMonth);

		if (month) {
			const year = new Date().getFullYear();
			const dayCount = new Date(year, month.monthIndex + 1, 0).getDate();

			for (let day = 1; day <= dayCount; day += 1) {
				const dateKey = localDateKey(new Date(year, month.monthIndex, day));
				const weekdayIndex = weekdayIndexForDate(dateKey);
				if (weekdayIndex < 0 || weekdayIndex > 4) continue;

				let group = groups.find((item) => item.key === mondayDateKey(dateKey));
				if (!group) {
					group = createMatrixWeekGroup(mondayDateKey(dateKey));
					groups.push(group);
				}

				group.slots[weekdayIndex] = {
					key: dateKey,
					weekday: MATRIX_WEEKDAYS[weekdayIndex],
					date: dates.find((date) => date.date === dateKey) ?? null,
					dateKey
				};
			}

			return groups.map((group, index) => ({
				...group,
				label: `Week ${index + 1}`
			}));
		}

		for (const date of dates) {
			const weekdayIndex = weekdayIndexForDate(date.date);
			if (weekdayIndex < 0 || weekdayIndex > 4) continue;

			const key = mondayDateKey(date.date);
			let group = groups.find((item) => item.key === key);

			if (!group) {
				group = createMatrixWeekGroup(key);
				groups.push(group);
			}

			group.slots[weekdayIndex] = {
				key: date.date,
				weekday: MATRIX_WEEKDAYS[weekdayIndex],
				date,
				dateKey: date.date
			};
		}

		return groups.map((group, index) => ({
			...group,
			label: `Week ${index + 1}`
		}));
	}

	function createMatrixWeekGroup(key: string): MatrixWeekGroup {
		return {
			key,
			label: '',
			slots: MATRIX_WEEKDAYS.map((weekday) => ({
				key: `${key}-${weekday}`,
				weekday,
				date: null,
				dateKey: null
			}))
		};
	}

	function mondayDateKey(date: string) {
		const [year, month, day] = date.split('-').map(Number);
		const value = new Date(year, month - 1, day);
		const weekday = value.getDay();
		const mondayOffset = weekday === 0 ? -6 : 1 - weekday;
		return localDateKey(new Date(year, month - 1, day + mondayOffset));
	}

	function weekdayIndexForDate(date: string) {
		const value = new Date(`${date}T00:00:00`);
		const weekday = value.getDay();
		if (weekday === 0 || weekday === 6) return -1;
		return weekday - 1;
	}

	function localDateKey(date: Date) {
		const year = date.getFullYear();
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const day = String(date.getDate()).padStart(2, '0');
		return `${year}-${month}-${day}`;
	}

	function weekRangeLabel(group: MatrixWeekGroup) {
		const dates = group.slots
			.map((slot) => slot.dateKey)
			.filter((date): date is string => date !== null);
		const first = dates[0];
		const last = dates.at(-1);
		if (!first || !last) return 'Mon-Fri';
		if (first === last) return matrixDateLabel(first);

		return `${formatWeekday(first)}-${formatWeekday(last)} / ${formatDayNumber(
			first
		)}-${formatDayNumber(last)}`;
	}

	function formatImportedAt(value?: number) {
		if (!value) return 'Not imported';
		return new Date(value * 1000).toLocaleDateString(undefined, {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function cellKey(studentId: string, date: string) {
		return `${studentId}:${date}`;
	}

	function cellLabel(row: Sf2PreviewStudentRow, cell: Sf2PreviewCell) {
		const state =
			cell.status === 'present' ? 'present' : 'absent';
		return `${row.studentName}, ${matrixDateLabel(cell.date)}: ${state}`;
	}

	function cellForDate(row: MatrixStudentRow, date: string) {
		return row.cellsByDate.get(date);
	}

	function unmappedCellLabel(row: Sf2PreviewStudentRow, date: string) {
		return `${row.studentName}, ${matrixDateLabel(date)}: no SF2 column mapped`;
	}

	function cellClass(row: Sf2PreviewStudentRow, cell: Sf2PreviewCell) {
		if (!row.mapped) return 'border-border bg-surface text-muted-foreground';
		if (cell.status === 'present') return 'border-emerald-500/30 bg-emerald-50 text-emerald-700';
		if (cell.status === 'absent') return 'border-red-500/35 bg-red-50 text-red-700';
		return 'border-border bg-background text-muted-foreground';
	}

	function reportMonthLabel(value: string) {
		return sf2ReportMonthLabel(value) || 'Blank';
	}

	function headerReviewValue(draftValue: string, templateValue: string) {
		const value = workbookSettings ? draftValue : draftValue || templateValue;
		return value.trim() || 'Blank';
	}

	function headerReviewMonthValue(templateValue: string) {
		const value = workbookSettings ? draftReportMonth : draftReportMonth || templateValue;
		return reportMonthLabel(value);
	}
</script>

<svelte:head>
	<title>Reports - Attendance System</title>
	<meta name="description" content="Review and export DepEd SF2 Excel reports." />
</svelte:head>

<svelte:window onkeydown={onWindowKeydown} />

<div class="flex h-full flex-col overflow-hidden">
	<PageHeader
		category="Reports"
		title="SF2 Workbook Pre-export Review"
		description="Review workbook details, mapped dates, absences, and learner row mappings before exporting the official SF2 copy."
	>
		{#snippet actions()}
			<div class="flex flex-wrap items-center gap-2">
				<button
					type="button"
					onclick={loadInitial}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface"
				>
					<RefreshCw class="size-4" aria-hidden="true" />
					Refresh
				</button>
				<button
					type="button"
					onclick={onOpenSf2}
					disabled={!preview?.template || opening || !activeClassId}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
				>
					<ExternalLink class="size-4" aria-hidden="true" />
					{opening ? 'Opening...' : 'Open SF2'}
				</button>
				<button
					type="button"
					onclick={() => (fullReviewOpen = true)}
					disabled={!preview?.template}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
				>
					<Maximize2 class="size-4" aria-hidden="true" />
					View Full Review
				</button>
				<button
					type="button"
					onclick={requestExport}
					disabled={exportDisabled}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
				>
					<Save class="size-4" aria-hidden="true" />
					{exporting ? 'Exporting...' : 'Review Export'}
				</button>
			</div>
		{/snippet}
	</PageHeader>

	{#if loading}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<LoadingBlock rows={4} label="Loading SF2 workbook preview" />
		</div>
	{:else if loadError}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<EmptyState tone="warning" title="SF2 reports are unavailable" description={loadError}>
				{#snippet actions()}
					<button
						type="button"
						onclick={loadInitial}
						class="control-ring rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface"
					>
						Retry
					</button>
				{/snippet}
			</EmptyState>
		</div>
	{:else if !preview?.template}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<EmptyState
				tone="warning"
				title="No SF2 workbook is ready for review"
				description={preview?.issues[0] ??
					'Import an SF2 workbook or create one from the bundled template first.'}
			>
				{#snippet actions()}
					<a
						href={resolve('/settings')}
						class="control-ring inline-flex rounded-pill bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground hover:bg-accent"
					>
						Open SF2 Settings
					</a>
				{/snippet}
			</EmptyState>
		</div>
	{:else}
		<section
			class="grid min-h-0 flex-1 gap-5 overflow-hidden px-4 py-5 md:px-8 lg:px-10 xl:grid-cols-[minmax(0,1fr)_360px]"
		>
			<div class="min-h-0 space-y-5 overflow-auto pr-0 xl:pr-1">
				<div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
					{@render summaryTile(
						'Workbook',
						preview.className || selectedClass?.name || 'Linked class',
						preview.canExport ? 'Ready for confirmation' : 'Needs attention',
						FileCheck2,
						preview.canExport ? 'ready' : 'warning'
					)}
					{@render summaryTile(
						'Absences',
						String(preview.absenceCount),
						`${preview.presentCount} present marks recorded`,
						UserX,
						preview.absenceCount > 0 ? 'alert' : 'ready'
					)}
					{@render summaryTile(
						'Checks',
						`${issueCount}/${warningCount}`,
						'Issues / warnings',
						CircleAlert,
						issueCount > 0 ? 'alert' : warningCount > 0 ? 'warning' : 'ready'
					)}
				</div>

				{@render sf2HeaderDetails(false)}

				{@render classDayMatrix(false)}
			</div>

			<aside class="min-h-0 space-y-5 overflow-auto">
				<div class="rounded-2xl border border-border bg-surface p-5">
					<div class="label-mono text-primary">Workbook identity</div>
					<dl class="mt-4 space-y-3 text-sm">
						{@render metaRow('Class', preview.className || selectedClass?.name || 'Unlinked')}
						{@render metaRow('Report Month', reportMonthLabel(preview.template.reportMonth))}
						{@render metaRow('School ID', preview.template.schoolId || 'Blank')}
						{@render metaRow('School Year', preview.template.schoolYear || 'Blank')}
						{@render metaRow('Adviser', preview.template.adviserName || 'Blank')}
						{@render metaRow('School Head', preview.template.schoolHeadName || 'Blank')}
						{@render metaRow('Imported', formatImportedAt(preview.template.importedAt))}
					</dl>
				</div>

				<div class="rounded-2xl border border-border bg-card p-5">
					<div class="flex items-start justify-between gap-3">
						<div>
							<div class="label-mono text-primary">Absent list</div>
							<h2 class="mt-1 text-lg font-semibold">{preview.absentList.length} entries</h2>
						</div>
						<UserX class="size-5 text-red-700" aria-hidden="true" />
					</div>

					{#if preview.absentList.length > 0}
						<div class="mt-4 max-h-80 space-y-2 overflow-auto pr-1">
							{#each preview.absentList as absence (`${absence.studentId}-${absence.date}`)}
								<div class="rounded-md border border-border bg-background p-3 text-sm">
									<div class="font-medium">{absence.studentName}</div>
									<div
										class="mt-1 flex items-center justify-between gap-3 text-xs text-muted-foreground"
									>
										<span>{formatDate(absence.date)}</span>
										<span>Row {absence.rowIndex}</span>
									</div>
								</div>
							{/each}
						</div>
					{:else}
						<p class="mt-4 text-sm leading-6 text-muted-foreground">
							No absences are currently marked for this report month.
						</p>
					{/if}
				</div>
			</aside>
		</section>
	{/if}
</div>

{#if fullReviewOpen && preview?.template}
	<div
		role="dialog"
		aria-modal="true"
		aria-label="Full SF2 review"
		class="fixed inset-x-0 top-8 bottom-0 z-[65] bg-background text-foreground"
		transition:fade={{ duration: 160 }}
	>
		<div class="flex h-full min-h-0 flex-col overflow-hidden">
			<div
				class="flex shrink-0 items-center justify-between gap-3 border-b border-border bg-background/95 px-4 py-3 backdrop-blur md:px-6"
			>
				<button
					type="button"
					onclick={() => (fullReviewOpen = false)}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface"
				>
					<ArrowLeft class="size-4" aria-hidden="true" />
					Back
				</button>
				<div class="min-w-0 text-right">
					<div class="label-mono text-primary">Full review</div>
					<div class="truncate text-sm font-semibold">
						{draftSchoolName || preview.template.schoolName || 'SF2 Workbook'}
					</div>
				</div>
				<button
					type="button"
					onclick={() => (fullReviewHeaderVisible = !fullReviewHeaderVisible)}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface"
				>
					<Settings2 class="size-4" aria-hidden="true" />
					{fullReviewHeaderVisible ? 'Hide Details' : 'Show Details'}
				</button>
			</div>

			<div
				class="min-h-0 flex-1 overflow-hidden px-4 py-4 md:px-6"
				transition:fade={{ duration: 180 }}
			>
				<div class="flex h-full min-h-0 flex-col gap-4">
					{#if fullReviewHeaderVisible}
						{@render sf2HeaderDetails(true)}
					{/if}
					{@render classDayMatrix(true)}
				</div>
			</div>
		</div>
	</div>
{/if}

<Dialog
	open={exportDialogOpen}
	title="Confirm SF2 Export"
	description="Export copies the reviewed SF2 working workbook to your chosen file path."
	maxWidth="2xl"
	onClose={() => (exportDialogOpen = false)}
>
	<div class="grid gap-3 sm:grid-cols-2">
		{@render confirmStat('Mapped learners', preview?.mappedStudents ?? 0)}
		{@render confirmStat('Absences', preview?.absenceCount ?? 0)}
	</div>

	{#if preview && preview.warnings.length > 0}
		<div class="rounded-md border border-amber-500/30 bg-amber-50 p-4 text-sm text-amber-900">
			<div class="font-semibold">Review these warnings before exporting.</div>
			<ul class="mt-3 max-h-48 space-y-2 overflow-auto">
				{#each preview.warnings as warning, index (`confirm-warning-${index}-${warning}`)}
					<li>{warning}</li>
				{/each}
			</ul>
		</div>
	{:else}
		<div class="rounded-md border border-emerald-500/30 bg-emerald-50 p-4 text-sm text-emerald-800">
			The workbook details, date mappings, and learner mappings have no detected warnings.
		</div>
	{/if}

	<div class="flex flex-wrap justify-end gap-2">
		<button
			type="button"
			onclick={() => (exportDialogOpen = false)}
			class="control-ring h-10 rounded-md border border-border bg-background px-4 text-sm font-medium hover:bg-surface"
		>
			Cancel
		</button>
		<button
			type="button"
			onclick={confirmExport}
			disabled={exporting || !preview?.canExport}
			class="control-ring inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
		>
			<Save class="size-4" aria-hidden="true" />
			{exporting ? 'Exporting...' : 'Export Workbook'}
		</button>
	</div>
</Dialog>

<Dialog
	open={exportLoadingOpen}
	title="Exporting SF2 Workbook"
	description="Saving the reviewed workbook and opening the exported file."
	maxWidth="lg"
	showCloseButton={false}
>
	<TaskProgress
		active={exportLoadingOpen}
		title="Exporting SF2 workbook"
		description="Writing attendance marks, copying the workbook, and opening the generated file."
		simple
	/>
</Dialog>

<FeedbackToast message={toastMessage} ok={toastOk} onClose={() => (toastMessage = null)} />

{#snippet sf2HeaderDetails(fullReview: boolean)}
	{#if preview?.template}
		<div
			class="border border-border bg-card p-5 shadow-sm {fullReview ? 'rounded-xl' : 'rounded-2xl'}"
		>
			<div class="flex flex-wrap items-start justify-between gap-3">
				<div>
					<div class="label-mono text-primary">SF2 workbook details</div>
					<h2 class="mt-1 text-xl font-semibold">
						{draftSchoolName || preview.template.schoolName || 'Name of School'}
					</h2>
					{#if !fullReview}
						<p class="mt-1 text-sm text-muted-foreground">
							These fields are written into the SF2 workbook before export.
						</p>
					{/if}
				</div>
				{#if !fullReview}
					<button
						type="button"
						onclick={() => saveWorkbookDetails()}
						disabled={!workbookSettings || savingDetails}
						class="control-ring inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
					>
						<Settings2 class="size-4" aria-hidden="true" />
						{savingDetails ? 'Saving...' : 'Save Details'}
					</button>
				{/if}
			</div>

			{#if fullReview}
				<dl class="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-4">
					{@render headerReviewField(
						'School ID',
						headerReviewValue(draftSchoolId, preview.template.schoolId)
					)}
					{@render headerReviewField(
						'School Year',
						headerReviewValue(draftSchoolYear, preview.template.schoolYear)
					)}
					{@render headerReviewField(
						'Report Month',
						headerReviewMonthValue(preview.template.reportMonth)
					)}
					{@render headerReviewField(
						'Grade Level',
						headerReviewValue(draftGradeLevel, preview.template.gradeLevel)
					)}
					<div class="md:col-span-2">
						{@render headerReviewField(
							'Name of School',
							headerReviewValue(draftSchoolName, preview.template.schoolName)
						)}
					</div>
					{@render headerReviewField(
						'Section',
						headerReviewValue(draftSection, preview.template.section)
					)}
					{@render headerReviewField(
						'Adviser / LIS Name',
						headerReviewValue(draftAdviserName, preview.template.adviserName)
					)}
					<div class="md:col-span-2">
						{@render headerReviewField(
							'School Head Name',
							headerReviewValue(draftSchoolHeadName, preview.template.schoolHeadName)
						)}
					</div>
				</dl>
			{:else}
				<div class="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-4">
					{@render textField('School ID', draftSchoolId, 'draftSchoolId')}
					{@render textField('School Year', draftSchoolYear, 'draftSchoolYear')}
					<label class="space-y-1.5">
						<span class="label-mono">Report Month</span>
						<select
							bind:value={draftReportMonth}
							onchange={onReportMonthChange}
							disabled={!workbookSettings || savingDetails}
							class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none disabled:opacity-60"
						>
							<option value="">Select month</option>
							{#each SF2_SCHOOL_MONTHS as month (month.value)}
								<option value={month.value}>{month.label}</option>
							{/each}
						</select>
					</label>
					{@render textField('Grade Level', draftGradeLevel, 'draftGradeLevel')}
					<div class="md:col-span-2">
						{@render textField('Name of School', draftSchoolName, 'draftSchoolName')}
					</div>
					{@render textField('Section', draftSection, 'draftSection')}
					{@render textField('Adviser / LIS Name', draftAdviserName, 'draftAdviserName')}
					<div class="md:col-span-2">
						{@render textField('School Head Name', draftSchoolHeadName, 'draftSchoolHeadName')}
					</div>
				</div>
			{/if}
		</div>
	{/if}
{/snippet}

{#snippet classDayMatrix(fullReview: boolean)}
	{#if preview?.template}
		<div
			class="border border-border bg-card shadow-sm {fullReview
				? 'flex min-h-0 flex-1 flex-col rounded-xl'
				: 'rounded-2xl'}"
		>
			<div
				class="flex flex-wrap items-start justify-between gap-3 border-b border-border px-5 py-4"
			>
				<div>
					<div class="label-mono text-primary">SF2 attendance grid</div>
					<h2 class="mt-1 text-xl font-semibold">
						{preview.template.gradeLevel} - {preview.template.section}
					</h2>
					<p class="mt-1 text-sm text-muted-foreground">
						Click a cell to toggle the learner between present and absent.
					</p>
				</div>
				<div class="flex flex-wrap gap-2 text-xs">
					<span
						class="rounded-pill border border-emerald-500/30 bg-emerald-50 px-2.5 py-1 text-emerald-700"
					>
						Present
					</span>
					<span class="rounded-pill border border-red-500/35 bg-red-50 px-2.5 py-1 text-red-700">
						Absent
					</span>
					<span
						class="rounded-pill border border-border bg-background px-2.5 py-1 text-muted-foreground"
					>
						Open day
					</span>
				</div>
			</div>

			<div class={fullReview ? 'min-h-0 flex-1 overflow-auto' : 'max-h-[560px] overflow-auto'}>
				<table class="min-w-full border-separate border-spacing-0 text-sm">
					<thead>
						<tr>
							<th
								rowspan="2"
								class="sticky top-0 left-0 z-30 w-72 min-w-72 border-r border-b border-border bg-card px-4 py-3 text-left align-middle"
							>
								Learner
							</th>
							{#each matrixWeekGroups as week (week.key)}
								<th
									colspan={week.slots.length}
									class="sticky top-0 z-20 border-b border-l-2 border-border border-l-primary/45 bg-orange-50 px-2 py-2 text-center"
									title={weekRangeLabel(week)}
								>
									<div class="label-mono text-primary">{week.label}</div>
									<div class="mt-0.5 font-mono text-[10px] font-medium text-muted-foreground">
										{weekRangeLabel(week)}
									</div>
								</th>
							{/each}
						</tr>
						<tr>
							{#each matrixWeekGroups as week (week.key)}
								{#each week.slots as slot, dateIndex (slot.key)}
									<th
										class="sticky top-[43px] z-10 min-w-14 border-b border-border bg-card px-2 py-2 text-center {dateIndex ===
										0
											? 'border-l-2 border-l-primary/45'
											: 'border-l border-l-border/60'}"
										title={slot.date
											? `${matrixDateLabel(slot.date.date)} ${slot.date.columnLetter}${slot.date.columnIndex}`
											: slot.dateKey
												? `${matrixDateLabel(slot.dateKey)}, no SF2 column mapped`
												: `${slot.weekday}, no class day in this month`}
									>
										<div class="font-mono text-sm leading-none font-bold">
											{slot.dateKey ? formatDayNumber(slot.dateKey) : ''}
										</div>
										<div class="mt-1 font-mono text-[10px] font-semibold text-muted-foreground">
											{slot.weekday}
										</div>
									</th>
								{/each}
							{/each}
						</tr>
					</thead>
					<tbody>
						{#each matrixStudents as row (row.studentId)}
							<tr class={row.mapped ? 'bg-background' : 'bg-amber-50/60'}>
								<th
									class="sticky left-0 z-10 w-72 min-w-72 border-r border-b border-border bg-inherit px-4 py-2 text-left align-middle"
								>
									<div class="flex items-center gap-2">
										<div class="min-w-0 flex-1">
											<div class="truncate font-medium">{row.studentName}</div>
											<div
												class="mt-0.5 flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground"
											>
												<span>{row.gender ?? 'No gender'}</span>
												<span aria-hidden="true">/</span>
												<span>{row.mapped ? `Row ${row.rowIndex}` : 'Unmapped'}</span>
											</div>
										</div>
										{#if row.warnings.length > 0}
											<AlertTriangle class="size-4 shrink-0 text-amber-600" aria-hidden="true" />
										{/if}
									</div>
								</th>
								{#each matrixWeekGroups as week (week.key)}
									{#each week.slots as slot, dateIndex (slot.key)}
										{@const cell = slot.dateKey ? cellForDate(row, slot.dateKey) : null}
										<td
											class="border-b border-border/80 px-1.5 py-1.5 text-center {dateIndex === 0
												? 'border-l-2 border-l-primary/30 bg-primary/5'
												: 'border-l border-l-border/40'}"
										>
											{#if cell}
												<button
													type="button"
													disabled={!cell.editable || !row.mapped || correctingCellKey !== null}
													onclick={() => toggleAttendance(row, cell)}
													aria-label={cellLabel(row, cell)}
													title={cellLabel(row, cell)}
													class="control-ring inline-grid size-9 place-items-center rounded-md border text-xs font-bold transition-colors disabled:cursor-not-allowed disabled:opacity-70 {cellClass(
														row,
														cell
													)}"
												>
													{#if correctingCellKey === cellKey(row.studentId, cell.date)}
														<span class="font-mono text-[10px]">...</span>
													{:else if cell.status === 'present'}
														<Check class="size-4" aria-hidden="true" />
													{:else if cell.status === 'absent'}
														<X class="size-4" aria-hidden="true" />
													{:else}
														<span aria-hidden="true">-</span>
													{/if}
												</button>
											{:else if slot.dateKey}
												<span
													role="img"
													aria-label={unmappedCellLabel(row, slot.dateKey)}
													title={unmappedCellLabel(row, slot.dateKey)}
													class="inline-grid size-9 place-items-center rounded-md border border-dashed border-border bg-background text-xs font-bold text-muted-foreground"
												>
													-
												</span>
											{:else}
												<span
													aria-hidden="true"
													class="inline-grid size-9 place-items-center text-muted-foreground"
												>
													&nbsp;
												</span>
											{/if}
										</td>
									{/each}
								{/each}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	{/if}
{/snippet}

{#snippet headerReviewField(label: string, value: string)}
	<div class="rounded-md border border-border bg-background px-3 py-2">
		<dt class="label-mono">{label}</dt>
		<dd class="mt-1 truncate text-sm font-semibold">{value}</dd>
	</div>
{/snippet}

{#snippet summaryTile(
	label: string,
	value: string,
	description: string,
	Icon: typeof FileCheck2,
	tone: 'neutral' | 'ready' | 'warning' | 'alert'
)}
	<div class="rounded-2xl border border-border bg-card p-4 shadow-sm">
		<div class="flex items-start justify-between gap-3">
			<div class="min-w-0">
				<div class="label-mono">{label}</div>
				<div class="mt-2 truncate text-2xl font-semibold">{value}</div>
				<div class="mt-1 truncate text-xs text-muted-foreground">{description}</div>
			</div>
			<div
				class="grid size-10 shrink-0 place-items-center rounded-md {tone === 'ready'
					? 'bg-emerald-50 text-emerald-700'
					: tone === 'warning'
						? 'bg-amber-50 text-amber-700'
						: tone === 'alert'
							? 'bg-red-50 text-red-700'
							: 'bg-surface text-muted-foreground'}"
			>
				<Icon class="size-5" aria-hidden="true" />
			</div>
		</div>
	</div>
{/snippet}

{#snippet textField(label: string, value: string, field: string)}
	<label class="space-y-1.5">
		<span class="label-mono">{label}</span>
		<input
			{value}
			oninput={(event) => {
				const nextValue = (event.currentTarget as HTMLInputElement).value;
				if (field === 'draftSchoolId') draftSchoolId = nextValue;
				if (field === 'draftSchoolName') draftSchoolName = nextValue;
				if (field === 'draftSchoolYear') draftSchoolYear = nextValue;
				if (field === 'draftGradeLevel') draftGradeLevel = nextValue;
				if (field === 'draftSection') draftSection = nextValue;
				if (field === 'draftAdviserName') draftAdviserName = nextValue;
				if (field === 'draftSchoolHeadName') draftSchoolHeadName = nextValue;
			}}
			disabled={!workbookSettings || savingDetails}
			class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none disabled:opacity-60"
		/>
	</label>
{/snippet}

{#snippet metaRow(label: string, value: string)}
	<div class="grid grid-cols-[112px_minmax(0,1fr)] gap-3">
		<dt class="text-xs font-medium text-muted-foreground">{label}</dt>
		<dd class="truncate font-medium">{value}</dd>
	</div>
{/snippet}

{#snippet confirmStat(label: string, value: number)}
	<div class="rounded-md border border-border bg-surface p-3">
		<div class="label-mono">{label}</div>
		<div class="mt-2 text-2xl font-semibold">{value}</div>
	</div>
{/snippet}
