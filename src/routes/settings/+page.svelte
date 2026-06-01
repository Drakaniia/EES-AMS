<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import {
		listClasses,
		saveClass,
		deleteClass,
		exportDatabase,
		exportJsonWithFolder,
		createSf2WorkbookFromTemplate,
		getSf2WorkbookSettings,
		importSf2Workbook,
		importAll,
		openSf2Workbook,
		updateSf2WorkbookSettings,
		wipeAll,
		type AttendanceMode,
		type Settings,
		type Class,
		type Session,
		type Sf2ImportSummary,
		type Sf2TemplateDraft,
		type Sf2WorkbookSettings
	} from '$lib/db-rust';

	type Sf2SchoolMonth = {
		value: string;
		label: string;
		monthIndex: number;
	};

	type Sf2CalendarCell = {
		key: string;
		day: number | null;
		label: string;
		isSchoolDay: boolean;
		isSelected: boolean;
	};

	const SF2_CALENDAR_WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

	const SF2_SCHOOL_MONTHS: Sf2SchoolMonth[] = [
		{ value: 'JUNE', label: 'June', monthIndex: 5 },
		{ value: 'JULY', label: 'July', monthIndex: 6 },
		{ value: 'AUGUST', label: 'August', monthIndex: 7 },
		{ value: 'SEPTEMBER', label: 'September', monthIndex: 8 },
		{ value: 'OCTOBER', label: 'October', monthIndex: 9 },
		{ value: 'NOVEMBER', label: 'November', monthIndex: 10 },
		{ value: 'DECEMBER', label: 'December', monthIndex: 11 },
		{ value: 'JANUARY', label: 'January', monthIndex: 0 },
		{ value: 'FEBRUARY', label: 'February', monthIndex: 1 },
		{ value: 'MARCH', label: 'March', monthIndex: 2 },
		{ value: 'APRIL', label: 'April', monthIndex: 3 }
	];

	function sf2MonthByIndex(monthIndex: number) {
		return SF2_SCHOOL_MONTHS.find((month) => month.monthIndex === monthIndex);
	}

	function sf2MonthByValue(value: string) {
		return SF2_SCHOOL_MONTHS.find((month) => month.value === value);
	}

	function defaultSf2ReportMonth(today = new Date()) {
		return sf2MonthByIndex(today.getMonth())?.value ?? 'JUNE';
	}

	function defaultSf2SchoolYear(today = new Date()) {
		const currentMonthIndex = today.getMonth();
		const startYear = currentMonthIndex <= 3 ? today.getFullYear() - 1 : today.getFullYear();
		return `${startYear}-${startYear + 1}`;
	}

	function sf2ReportYear(monthValue: string, schoolYear: string) {
		const month = sf2MonthByValue(monthValue);
		const match = schoolYear.match(/(20\d{2})\D+(20\d{2})/);
		if (!month || !match) return new Date().getFullYear();
		return month.monthIndex >= 5 ? Number(match[1]) : Number(match[2]);
	}

	function defaultSf2FirstSchoolDay(monthValue: string, schoolYear: string) {
		const month = sf2MonthByValue(monthValue);
		if (!month) return 1;

		const year = sf2ReportYear(monthValue, schoolYear);
		const firstDayOfWeek = new Date(year, month.monthIndex, 1).getDay();
		if (firstDayOfWeek === 0) return 2;
		if (firstDayOfWeek === 6) return 3;
		return 1;
	}

	function sf2MonthDayCount(monthValue: string, schoolYear: string) {
		const month = sf2MonthByValue(monthValue);
		if (!month) return 31;
		return new Date(sf2ReportYear(monthValue, schoolYear), month.monthIndex + 1, 0).getDate();
	}

	function isSf2SchoolDay(monthValue: string, schoolYear: string, day: number) {
		const month = sf2MonthByValue(monthValue);
		if (!month) return false;

		const dayCount = sf2MonthDayCount(monthValue, schoolYear);
		if (day < 1 || day > dayCount) return false;

		const weekday = new Date(sf2ReportYear(monthValue, schoolYear), month.monthIndex, day).getDay();
		return weekday >= 1 && weekday <= 5;
	}

	function normalizedSf2FirstSchoolDay(monthValue: string, schoolYear: string, day: number) {
		if (isSf2SchoolDay(monthValue, schoolYear, day)) return day;
		return defaultSf2FirstSchoolDay(monthValue, schoolYear);
	}

	function sf2CalendarCells(
		monthValue: string,
		schoolYear: string,
		selectedDay: number
	): Sf2CalendarCell[] {
		const month = sf2MonthByValue(monthValue);
		if (!month) return [];

		const year = sf2ReportYear(monthValue, schoolYear);
		const dayCount = sf2MonthDayCount(monthValue, schoolYear);
		const firstWeekday = new Date(year, month.monthIndex, 1).getDay();
		const leadingBlankCount = (firstWeekday + 6) % 7;
		const cells: Sf2CalendarCell[] = [];

		for (let index = 0; index < leadingBlankCount; index += 1) {
			cells.push({
				key: `blank-start-${index}`,
				day: null,
				label: '',
				isSchoolDay: false,
				isSelected: false
			});
		}

		for (let day = 1; day <= dayCount; day += 1) {
			const isSchoolDay = isSf2SchoolDay(monthValue, schoolYear, day);
			cells.push({
				key: `day-${day}`,
				day,
				label: String(day),
				isSchoolDay,
				isSelected: selectedDay === day
			});
		}

		while (cells.length % 7 !== 0) {
			cells.push({
				key: `blank-end-${cells.length}`,
				day: null,
				label: '',
				isSchoolDay: false,
				isSelected: false
			});
		}

		return cells;
	}

	function sf2SelectedFirstAttendanceLabel() {
		const month = sf2MonthByValue(sf2DraftReportMonth);
		if (!month) return `Day ${sf2DraftFirstSchoolDay}`;
		const year = sf2ReportYear(sf2DraftReportMonth, sf2DraftSchoolYear);
		return `${month.label} ${sf2DraftFirstSchoolDay}, ${year}`;
	}

	function selectSf2FirstSchoolDay(day: number | null) {
		if (day === null) return;
		if (!isSf2SchoolDay(sf2DraftReportMonth, sf2DraftSchoolYear, day)) return;
		sf2DraftFirstSchoolDay = day;
	}

	function selectSf2ReportMonth(monthValue: string) {
		const schoolYear = sf2DraftSchoolYear.trim() || defaultSf2SchoolYear();
		sf2DraftReportMonth = monthValue;
		sf2DraftSchoolYear = schoolYear;
		sf2DraftFirstSchoolDay = defaultSf2FirstSchoolDay(monthValue, schoolYear);
	}

	function updateSf2SchoolYear(value: string) {
		sf2DraftSchoolYear = value;
		sf2DraftFirstSchoolDay = normalizedSf2FirstSchoolDay(
			sf2DraftReportMonth,
			value,
			sf2DraftFirstSchoolDay
		);
	}

	// ── State ────────────────────────────────────────────────────────────────
	let classes = $state<Class[]>([]);

	// Global settings fields - derived from store
	let defaultDayStart = $state('08:30');
	let defaultDayEnd = $state('15:30');
	let defaultLateAfter = $state('08:45');
	let defaultQuarter = $state('1st Quarter');
	let attendanceMode = $state<AttendanceMode>('manual');

	let q1Start = $state('');
	let q1End = $state('');
	let q2Start = $state('');
	let q2End = $state('');
	let q3Start = $state('');
	let q3End = $state('');

	// Quarter Dialog state
	let quarterDialogOpen = $state(false);

	// Class Dialog state
	let classDialogOpen = $state(false);
	let editingClass = $state<Class | null>(null);
	let formClassName = $state('');
	let formRoom = $state('');
	let formDayStart = $state('');
	let formDayEnd = $state('');
	let formLateAfter = $state('');
	let formSessions = $state<Session[]>([]);
	let formDays = $state<number[]>([1, 2, 3, 4, 5]);
	let sessionMode = $state<'single' | 'morning-afternoon' | 'custom'>('single');

	// Toast
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	// Delete confirmation dialog
	let deleteTarget = $state<{ id: string; name: string } | null>(null);
	let wipeTarget = $state(false);

	// Hidden file input reference
	let fileInput = $state<HTMLInputElement | null>(null);

	// Export dialog state
	let exportDialogOpen = $state(false);
	let exportFormat = $state<'json' | 'database'>('json');

	// SF2 workbook state
	let sf2Importing = $state(false);
	let sf2TemplateCreating = $state(false);
	let sf2TemplateClassId = $state('');
	let sf2Opening = $state(false);
	let sf2SettingsLoading = $state(false);
	let sf2SettingsSaving = $state(false);
	let sf2ImportSummary = $state<Sf2ImportSummary | null>(null);
	let sf2TemplateDialogOpen = $state(false);
	let sf2TemplateDialogMode = $state<'create' | 'edit'>('create');
	let sf2DraftSchoolId = $state('');
	let sf2DraftSchoolName = $state('');
	let sf2DraftSchoolYear = $state('');
	let sf2DraftReportMonth = $state('');
	let sf2DraftGradeLevel = $state('');
	let sf2DraftSection = $state('');
	let sf2DraftAdviserName = $state('');
	let sf2DraftSchoolHeadName = $state('');
	let sf2DraftFirstSchoolDay = $state(1);
	let sf2FirstAttendanceCalendar = $derived(
		sf2CalendarCells(sf2DraftReportMonth, sf2DraftSchoolYear, sf2DraftFirstSchoolDay)
	);
	let sf2FirstAttendanceLabel = $derived(sf2SelectedFirstAttendanceLabel());

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	async function reload() {
		try {
			const [c] = await Promise.all([listClasses(), settingsStore.load()]);
			classes = c;
			if (!sf2TemplateClassId && classes.length > 0) {
				sf2TemplateClassId = classes[0].id;
			}
			// Update form fields from the store
			if (settingsStore.settings) {
				defaultDayStart = settingsStore.settings.dayStart;
				defaultDayEnd = settingsStore.settings.dayEnd;
				defaultLateAfter = settingsStore.settings.lateAfter;
				defaultQuarter = settingsStore.settings.quarter;
				attendanceMode = settingsStore.settings.attendanceMode ?? 'manual';
				q1Start = settingsStore.settings.q1Start ?? '';
				q1End = settingsStore.settings.q1End ?? '';
				q2Start = settingsStore.settings.q2Start ?? '';
				q2End = settingsStore.settings.q2End ?? '';
				q3Start = settingsStore.settings.q3Start ?? '';
				q3End = settingsStore.settings.q3End ?? '';
			}
		} catch (err: unknown) {
			const msg = errorMessage(err, 'Database error');
			toast(`Failed to load: ${msg}`, false);
		}
	}

	// ── Actions ──────────────────────────────────────────────────────────────
	function currentSettingsPayload(): Settings {
		return {
			id: 'app',
			dayStart: defaultDayStart,
			dayEnd: defaultDayEnd,
			lateAfter: defaultLateAfter,
			quarter: defaultQuarter,
			attendanceMode,
			q1Start,
			q1End,
			q2Start,
			q2End,
			q3Start,
			q3End
		};
	}

	async function onSaveGlobal(e: SubmitEvent) {
		e.preventDefault();
		try {
			await settingsStore.save(currentSettingsPayload());
			toast('Global configuration saved');
		} catch (error) {
			const msg = errorMessage(error, 'Failed to save settings');
			toast(`Save failed: ${msg}`, false);
		}
	}

	function errorMessage(error: unknown, fallback: string) {
		if (error instanceof Error) return error.message;
		if (typeof error === 'string') return error;
		return fallback;
	}

	function openAddClass() {
		editingClass = null;
		formClassName = '';
		formRoom = '';
		formDayStart = defaultDayStart;
		formDayEnd = defaultDayEnd;
		formLateAfter = defaultLateAfter;
		formSessions = [
			{
				name: 'Full Day',
				startTime: defaultDayStart,
				endTime: defaultDayEnd,
				lateAfter: defaultLateAfter
			}
		];
		formDays = [1, 2, 3, 4, 5];
		sessionMode = 'single';
		classDialogOpen = true;
	}

	function openEditClass(c: Class) {
		editingClass = c;
		formClassName = c.name;
		formRoom = c.room ?? '';
		formDayStart = c.dayStart;
		formDayEnd = c.dayEnd;
		formLateAfter = c.lateAfter;
		formSessions =
			c.sessions && c.sessions.length > 0
				? JSON.parse(JSON.stringify(c.sessions))
				: [
						{
							name: 'Full Day',
							startTime: c.dayStart,
							endTime: c.dayEnd,
							lateAfter: c.lateAfter
						}
					];
		formDays = c.days && c.days.length > 0 ? [...c.days] : [1, 2, 3, 4, 5];

		if (formSessions.length === 1 && formSessions[0].name === 'Full Day') {
			sessionMode = 'single';
		} else if (
			formSessions.length === 2 &&
			formSessions[0].name === 'Morning' &&
			formSessions[1].name === 'Afternoon'
		) {
			sessionMode = 'morning-afternoon';
		} else {
			sessionMode = 'custom';
		}

		classDialogOpen = true;
	}

	function handleSessionModeChange(mode: typeof sessionMode) {
		sessionMode = mode;
		if (mode === 'single') {
			formSessions = [
				{
					name: 'Full Day',
					startTime: defaultDayStart,
					endTime: defaultDayEnd,
					lateAfter: defaultLateAfter
				}
			];
		} else if (mode === 'morning-afternoon') {
			formSessions = [
				{ name: 'Morning', startTime: '07:30', endTime: '11:30', lateAfter: '07:45' },
				{ name: 'Afternoon', startTime: '13:00', endTime: '17:00', lateAfter: '13:15' }
			];
		}
	}

	function addSession() {
		formSessions = [
			...formSessions,
			{
				name: `Session ${formSessions.length + 1}`,
				startTime: '08:00',
				endTime: '12:00',
				lateAfter: '08:15'
			}
		];
	}

	function removeSession(index: number) {
		formSessions = formSessions.filter((_, i) => i !== index);
	}

	async function onSaveClass(e: SubmitEvent) {
		e.preventDefault();
		const name = formClassName.trim();
		if (!name) return;

		// Use the first session as primary times for backwards compatibility
		const primary = formSessions[0] || {
			startTime: formDayStart,
			endTime: formDayEnd,
			lateAfter: formLateAfter
		};

		const c: Class = {
			id: editingClass?.id ?? '',
			name,
			room: formRoom.trim(),
			dayStart: primary.startTime,
			dayEnd: primary.endTime,
			lateAfter: primary.lateAfter,
			sessions: formSessions,
			days: formDays,
			createdAt: editingClass?.createdAt ?? ''
		};

		try {
			await saveClass(c, !!editingClass);
			toast(editingClass ? 'Class updated' : 'Class added');
			classDialogOpen = false;
			reload();
		} catch (error) {
			toast(`Failed to save class: ${error}`, false);
		}
	}

	async function confirmDeleteClass(target = deleteTarget) {
		if (!target) return;
		await deleteClass(target.id);
		toast('Class deleted');
		deleteTarget = null;
		reload();
	}

	async function onDeleteClass(event: MouseEvent, id: string) {
		const classToDelete = classes.find((c) => c.id === id);
		if (!classToDelete) return;

		const target = { id: classToDelete.id, name: classToDelete.name };
		if (event.shiftKey) {
			await confirmDeleteClass(target);
			return;
		}

		deleteTarget = target;
	}

	function openExportDialog() {
		exportDialogOpen = true;
	}

	async function onExport() {
		try {
			let filePath: string;

			if (exportFormat === 'database') {
				filePath = await exportDatabase();
				toast(`Database exported to: ${filePath}`);
			} else {
				filePath = await exportJsonWithFolder();
				toast(`JSON exported to: ${filePath}`);
			}

			exportDialogOpen = false;
		} catch (error) {
			const msg = errorMessage(error, 'Export failed');
			toast(`Export failed: ${msg}`, false);
		}
	}

	async function onImport(file: File) {
		try {
			const txt = await file.text();
			const data = JSON.parse(txt);
			await importAll(data);
			await reload();
			toast('Backup imported');
		} catch (err: unknown) {
			const msg = errorMessage(err, 'Unknown error');
			toast(`Import failed: ${msg}`, false);
		}
	}

	function handleFileChange(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (file) onImport(file);
		input.value = '';
	}

	async function onImportSf2() {
		if (sf2Importing) return;
		sf2Importing = true;

		try {
			const summary = await importSf2Workbook();
			sf2ImportSummary = summary;
			await reload();
			toast(`Imported ${summary.learnersFound} learners from SF2`);
		} catch (error) {
			const msg = errorMessage(error, 'SF2 import failed');
			toast(`SF2 import failed: ${msg}`, false);
		} finally {
			sf2Importing = false;
		}
	}

	function openSf2TemplateDialog() {
		const reportMonth = defaultSf2ReportMonth();
		const schoolYear = defaultSf2SchoolYear();
		sf2TemplateDialogMode = 'create';
		sf2DraftSchoolId = '';
		sf2DraftSchoolName = '';
		sf2DraftSchoolYear = schoolYear;
		sf2DraftReportMonth = reportMonth;
		sf2DraftGradeLevel = '';
		sf2DraftSection = '';
		sf2DraftAdviserName = '';
		sf2DraftSchoolHeadName = '';
		sf2DraftFirstSchoolDay = defaultSf2FirstSchoolDay(reportMonth, schoolYear);
		sf2TemplateDialogOpen = true;
	}

	function populateSf2Draft(settings: Sf2WorkbookSettings) {
		sf2TemplateClassId = settings.classId;
		sf2DraftSchoolId = settings.schoolId;
		sf2DraftSchoolName = settings.schoolName;
		sf2DraftSchoolYear = settings.schoolYear;
		sf2DraftReportMonth = settings.reportMonth;
		sf2DraftGradeLevel = settings.gradeLevel;
		sf2DraftSection = settings.section;
		sf2DraftAdviserName = settings.adviserName;
		sf2DraftSchoolHeadName = settings.schoolHeadName;
		sf2DraftFirstSchoolDay = normalizedSf2FirstSchoolDay(
			settings.reportMonth,
			settings.schoolYear,
			settings.firstSchoolDay || 1
		);
	}

	async function openSf2SettingsDialog() {
		if (sf2SettingsLoading) return;
		sf2SettingsLoading = true;

		try {
			const settings = await getSf2WorkbookSettings(sf2TemplateClassId);
			sf2TemplateDialogMode = 'edit';
			populateSf2Draft(settings);
			sf2TemplateDialogOpen = true;
		} catch (error) {
			const msg = errorMessage(error, 'SF2 settings failed');
			toast(`SF2 settings failed: ${msg}`, false);
		} finally {
			sf2SettingsLoading = false;
		}
	}

	function sf2DraftPayload(): Sf2TemplateDraft {
		const firstSchoolDay = normalizedSf2FirstSchoolDay(
			sf2DraftReportMonth,
			sf2DraftSchoolYear,
			sf2DraftFirstSchoolDay
		);
		sf2DraftFirstSchoolDay = firstSchoolDay;

		return {
			classId: sf2TemplateDialogMode === 'edit' ? sf2TemplateClassId || undefined : undefined,
			schoolId: sf2DraftSchoolId,
			schoolName: sf2DraftSchoolName,
			schoolYear: sf2DraftSchoolYear,
			reportMonth: sf2DraftReportMonth,
			gradeLevel: sf2DraftGradeLevel,
			section: sf2DraftSection,
			adviserName: sf2DraftAdviserName,
			schoolHeadName: sf2DraftSchoolHeadName,
			firstSchoolDay,
			learnerNames: []
		};
	}

	async function onCreateSf2FromTemplate(event: SubmitEvent) {
		event.preventDefault();
		if (sf2TemplateCreating || sf2SettingsSaving) return;

		const creating = sf2TemplateDialogMode === 'create';
		if (creating) {
			sf2TemplateCreating = true;
		} else {
			sf2SettingsSaving = true;
		}
		try {
			const draft = sf2DraftPayload();
			const summary = creating
				? await createSf2WorkbookFromTemplate(draft)
				: await updateSf2WorkbookSettings(draft);
			sf2ImportSummary = summary;
			sf2TemplateClassId = summary.classId;
			sf2TemplateDialogOpen = false;
			await reload();
			toast(
				creating
					? `Created SF2 working copy for ${summary.learnersFound} learners`
					: `Updated SF2 workbook for ${summary.learnersFound} learners`
			);
		} catch (error) {
			const msg = errorMessage(error, creating ? 'SF2 template setup failed' : 'SF2 update failed');
			toast(`${creating ? 'SF2 template setup' : 'SF2 update'} failed: ${msg}`, false);
		} finally {
			sf2TemplateCreating = false;
			sf2SettingsSaving = false;
		}
	}

	async function onOpenSf2() {
		if (sf2Opening) return;
		sf2Opening = true;

		try {
			const path = await openSf2Workbook(sf2ImportSummary?.classId || sf2TemplateClassId);
			toast(`Opened SF2 workbook: ${path}`);
		} catch (error) {
			const msg = errorMessage(error, 'Open SF2 failed');
			toast(`Open SF2 failed: ${msg}`, false);
		} finally {
			sf2Opening = false;
		}
	}

	function startSf2Attendance() {
		if (!sf2ImportSummary) return;
		goto(resolve(`/attendance?classId=${sf2ImportSummary.classId}&manual=true`));
	}

	async function onWipe() {
		wipeTarget = true;
	}

	function getDaysLabel(days: number[]) {
		if (!days || days.length === 0) return 'None';
		if (days.length === 7) return 'Everyday';
		const weekdays = [1, 2, 3, 4, 5];
		if (days.length === 5 && weekdays.every((d) => days.includes(d))) return 'Weekdays';

		const shortDayNames = ['S', 'M', 'T', 'W', 'TH', 'F', 'S'];
		return days
			.slice()
			.sort((a, b) => a - b)
			.map((d) => shortDayNames[d])
			.join(' ');
	}

	// ── Lifecycle ────────────────────────────────────────────────────────────
	onMount(() => {
		reload();
	});
</script>

<svelte:head>
	<title>Settings — Attendance System</title>
	<meta name="description" content="Manage your classes and system configuration." />
</svelte:head>

<AppShell>
	<PageHeader
		category="Settings"
		title="System Configuration"
		description="Manage your class schedule and system-wide attendance rules."
	/>

	{#if settingsStore.loading}
		<div class="px-6 py-12 text-sm text-muted-foreground md:px-12">Loading…</div>
	{:else if settingsStore.error}
		<div class="px-6 py-12 text-sm text-destructive md:px-12">
			Error: {settingsStore.error}
			<button onclick={reload} class="ml-2 underline">Retry</button>
		</div>
	{:else}
		<div class="grid gap-6 px-6 py-6 md:px-12 lg:grid-cols-12">
			<!-- ── Class Management ────────────────────────────────────────── -->
			<div class="space-y-6 lg:col-span-8">
				<section class="overflow-hidden rounded-2xl border border-border bg-card">
					<div class="flex items-center justify-between p-6 pb-4">
						<h3 class="text-lg font-medium">Classes & Schedule</h3>
						<button
							onclick={openAddClass}
							class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
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
								<path d="M12 5v14M5 12h14" />
							</svg>
							Add Class
						</button>
					</div>

					<div class="divide-y divide-border border-t border-border pt-5">
						{#if classes.length === 0}
							<div class="p-12 text-center text-sm text-muted-foreground">
								No classes configured. Add a class to start tracking attendance.
							</div>
						{:else}
							{#each classes as c (c.id)}
								<div
									class="flex items-center justify-between p-6 transition-colors hover:bg-surface"
								>
									<div class="space-y-1">
										<div class="flex items-center gap-3">
											<div class="font-medium">{c.name}</div>
											{#if c.days}
												<span
													class="rounded-md bg-accent/10 px-2 py-0.5 text-[10px] font-bold tracking-wide text-accent uppercase"
												>
													{getDaysLabel(c.days)}
												</span>
											{/if}
										</div>
										<div
											class="label-mono flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground"
										>
											{#if c.room}
												<span>Room {c.room}</span>
											{/if}
											{#if c.sessions && c.sessions.length > 0}
												{#each c.sessions as s (s.name)}
													<span class="inline-flex items-center gap-1">
														<span class="font-medium text-foreground">{s.name}:</span>
														{s.startTime}–{s.endTime}
													</span>
												{/each}
											{:else}
												<span>{c.dayStart} – {c.dayEnd}</span>
												<span class="text-accent">Late after {c.lateAfter}</span>
											{/if}
										</div>
									</div>
									<div class="flex gap-2">
										<button
											onclick={() => openEditClass(c)}
											class="inline-flex size-9 items-center justify-center rounded-md border border-border bg-background transition-colors hover:bg-surface"
											title="Edit class"
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
												<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
												<path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
											</svg>
										</button>
										<button
											onclick={(event) => onDeleteClass(event, c.id)}
											class="inline-flex size-9 items-center justify-center rounded-md border border-border bg-background text-destructive transition-colors hover:bg-surface"
											title="Delete class"
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
												<polyline points="3 6 5 6 21 6" />
												<path
													d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"
												/>
											</svg>
										</button>
									</div>
								</div>
							{/each}
						{/if}
					</div>
				</section>

				<!-- ── Backups ───────────────────────────────────────────────────── -->
				<section class="space-y-5 rounded-2xl border border-border bg-card p-6">
					<h3 class="text-lg font-medium">Data Management</h3>
					<p class="text-sm text-muted-foreground">
						Your data is stored locally. Backups include the student list, attendance records,
						classes, and system configuration.
					</p>

					<div class="flex flex-wrap gap-2">
						<button
							onclick={openExportDialog}
							class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
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
							Export Data
						</button>

						<button
							onclick={() => fileInput?.click()}
							class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
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
								<polyline points="17 8 12 3 7 8" />
								<line x1="12" y1="3" x2="12" y2="15" />
							</svg>
							Import Backup
						</button>
						<input
							bind:this={fileInput}
							type="file"
							accept="application/json"
							class="hidden"
							onchange={handleFileChange}
						/>
					</div>

					<div class="space-y-3 border-t border-border pt-5">
						<button
							onclick={onWipe}
							class="inline-flex items-center gap-2 rounded-pill border border-destructive/40 px-4 py-2 text-sm font-medium text-destructive transition-colors hover:bg-destructive/10"
						>
							Wipe all data
						</button>
					</div>
				</section>

				<section class="space-y-5 rounded-2xl border border-border bg-card p-6">
					<div class="flex flex-wrap items-start justify-between gap-4">
						<div>
							<h3 class="text-lg font-medium">SF2 Workbook</h3>
							<p class="mt-1 text-sm text-muted-foreground">
								Import the official SF2 .xls form, or create a first-month working copy from the
								bundled template.
							</p>
						</div>
						<div class="flex flex-wrap gap-2">
							<select
								aria-label="Class for SF2 template"
								bind:value={sf2TemplateClassId}
								disabled={classes.length === 0 || sf2TemplateCreating}
								class="h-10 min-w-52 rounded-pill border border-border bg-background px-4 text-sm focus:ring-2 focus:ring-primary focus:outline-none disabled:cursor-not-allowed disabled:opacity-60"
							>
								<option value="">Select class</option>
								{#each classes as item (item.id)}
									<option value={item.id}>{item.name}</option>
								{/each}
							</select>
							<button
								onclick={openSf2TemplateDialog}
								disabled={sf2TemplateCreating}
								class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
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
									<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
									<polyline points="14 2 14 8 20 8" />
									<path d="M12 11v6" />
									<path d="M9 14h6" />
								</svg>
								{sf2TemplateCreating ? 'Creating...' : 'Create From Template'}
							</button>
							<button
								onclick={openSf2SettingsDialog}
								disabled={sf2SettingsLoading || !sf2TemplateClassId}
								class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
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
									<path d="M12 20h9" />
									<path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
								</svg>
								{sf2SettingsLoading ? 'Loading...' : 'Edit SF2 Workbook'}
							</button>
							<button
								onclick={onImportSf2}
								disabled={sf2Importing}
								class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
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
									<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
									<polyline points="14 2 14 8 20 8" />
									<path d="M12 18v-6" />
									<path d="m9 15 3 3 3-3" />
								</svg>
								{sf2Importing ? 'Importing...' : 'Import SF2'}
							</button>
							<button
								onclick={onOpenSf2}
								disabled={sf2Opening}
								class="inline-flex items-center gap-2 rounded-md border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
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
									<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
									<polyline points="15 3 21 3 21 9" />
									<line x1="10" y1="14" x2="21" y2="3" />
								</svg>
								{sf2Opening ? 'Opening...' : 'Open Edited SF2'}
							</button>
						</div>
					</div>

					{#if sf2ImportSummary}
						<div class="space-y-4 border-t border-border pt-5">
							<div class="grid gap-3 sm:grid-cols-4">
								<div class="rounded-xl border border-border bg-surface p-4">
									<div class="label-mono">Class</div>
									<div class="mt-2 text-sm font-semibold">{sf2ImportSummary.className}</div>
								</div>
								<div class="rounded-xl border border-border bg-surface p-4">
									<div class="label-mono">Learners</div>
									<div class="mt-2 text-2xl font-semibold">{sf2ImportSummary.learnersFound}</div>
								</div>
								<div class="rounded-xl border border-border bg-surface p-4">
									<div class="label-mono">Created</div>
									<div class="mt-2 text-2xl font-semibold">{sf2ImportSummary.studentsCreated}</div>
								</div>
								<div class="rounded-xl border border-border bg-surface p-4">
									<div class="label-mono">Dates</div>
									<div class="mt-2 text-2xl font-semibold">{sf2ImportSummary.datesMapped}</div>
								</div>
							</div>
							<div class="flex justify-end">
								<button
									onclick={startSf2Attendance}
									class="rounded-pill bg-primary px-5 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
								>
									Start Attendance
								</button>
							</div>
						</div>
					{/if}
				</section>
			</div>

			<!-- ── Sidebar: Global Defaults ────────────────────────────────── -->
			<div class="space-y-6 lg:col-span-4">
				<form
					onsubmit={onSaveGlobal}
					class="space-y-5 rounded-2xl border border-border bg-card p-6"
				>
					<div class="space-y-1">
						<h3 class="text-lg font-medium">Global Configuration</h3>
						<p class="text-xs text-muted-foreground">
							Controls attendance flow and defaults for new classes.
						</p>
					</div>

					<div class="space-y-4">
						<fieldset class="space-y-2">
							<legend class="label-mono">Attendance Type</legend>
							<div class="grid gap-2 rounded-xl border border-border bg-surface p-1">
								<button
									type="button"
									aria-pressed={attendanceMode === 'manual'}
									onclick={() => (attendanceMode = 'manual')}
									class="rounded-lg border px-3 py-3 text-left transition-colors {attendanceMode ===
									'manual'
										? 'border-primary bg-background shadow-sm'
										: 'border-transparent text-muted-foreground hover:bg-background/70 hover:text-foreground'}"
								>
									<span class="block text-sm font-semibold">Without card reader</span>
									<span class="mt-1 block text-xs leading-5">
										Name-only manual attendance for daily use.
									</span>
								</button>
								<button
									type="button"
									aria-pressed={attendanceMode === 'card_reader'}
									onclick={() => (attendanceMode = 'card_reader')}
									class="rounded-lg border px-3 py-3 text-left transition-colors {attendanceMode ===
									'card_reader'
										? 'border-primary bg-background shadow-sm'
										: 'border-transparent text-muted-foreground hover:bg-background/70 hover:text-foreground'}"
								>
									<span class="block text-sm font-semibold">With card reader</span>
									<span class="mt-1 block text-xs leading-5">
										Live session optimized for ID card taps.
									</span>
								</button>
							</div>
						</fieldset>

						<div class="space-y-2">
							<label for="defDayStart" class="label-mono">Default Day Start</label>
							<input
								id="defDayStart"
								type="time"
								bind:value={defaultDayStart}
								class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
							/>
						</div>
						<div class="space-y-2">
							<label for="defDayEnd" class="label-mono">Default Day End</label>
							<input
								id="defDayEnd"
								type="time"
								bind:value={defaultDayEnd}
								class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
							/>
						</div>
						<div class="space-y-2">
							<label for="defLateAfter" class="label-mono">Default Late After</label>
							<input
								id="defLateAfter"
								type="time"
								bind:value={defaultLateAfter}
								class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
							/>
						</div>
						<div class="space-y-2">
							<label for="defQuarter" class="label-mono">Current Quarter</label>
							<button
								type="button"
								onclick={() => (quarterDialogOpen = true)}
								class="flex h-10 w-full items-center justify-between rounded-md border border-border bg-background px-3 text-sm transition-colors hover:bg-accent/50 focus:ring-2 focus:ring-primary focus:outline-none"
							>
								<span>{defaultQuarter}</span>
								<svg
									xmlns="http://www.w3.org/2000/svg"
									width="16"
									height="16"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									class="opacity-50"
								>
									<path d="m6 9 6 6 6-6" />
								</svg>
							</button>
						</div>
					</div>

					<button
						type="submit"
						class="w-full rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
					>
						Save Configuration
					</button>
				</form>
			</div>
		</div>
	{/if}
</AppShell>

<!-- SF2 Template Dialog -->
<Dialog
	open={sf2TemplateDialogOpen}
	title={sf2TemplateDialogMode === 'create' ? 'Create SF2 Workbook' : 'Edit SF2 Workbook'}
	description={sf2TemplateDialogMode === 'create'
		? 'Enter the form details for this workbook copy.'
		: 'Update the saved workbook copy and attendance date layout.'}
	maxWidth="xl"
	onClose={() => (sf2TemplateDialogOpen = false)}
>
	<form onsubmit={onCreateSf2FromTemplate} class="space-y-5">
		<div class="grid gap-4 sm:grid-cols-2">
			<div class="space-y-1.5">
				<label for="sf2SchoolYear" class="label-mono">School Year</label>
				<input
					id="sf2SchoolYear"
					value={sf2DraftSchoolYear}
					oninput={(event) => updateSf2SchoolYear((event.currentTarget as HTMLInputElement).value)}
					required
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
		</div>

		<div class="grid gap-4 sm:grid-cols-2">
			<div class="space-y-1.5">
				<label for="sf2SchoolId" class="label-mono">School ID</label>
				<input
					id="sf2SchoolId"
					bind:value={sf2DraftSchoolId}
					required
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
			<div class="space-y-1.5">
				<label for="sf2SchoolName" class="label-mono">Name of School</label>
				<input
					id="sf2SchoolName"
					bind:value={sf2DraftSchoolName}
					required
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
		</div>

		<div class="space-y-2">
			<span class="label-mono">Report Month</span>
			<div class="grid grid-cols-3 gap-2 sm:grid-cols-4">
				{#each SF2_SCHOOL_MONTHS as month (month.value)}
					<button
						type="button"
						aria-pressed={sf2DraftReportMonth === month.value}
						onclick={() => selectSf2ReportMonth(month.value)}
						class={`h-10 rounded-md border px-3 text-sm font-medium transition-colors ${
							sf2DraftReportMonth === month.value
								? 'border-primary bg-primary text-primary-foreground shadow-sm'
								: 'border-border bg-background hover:bg-surface'
						}`}
					>
						{month.label}
					</button>
				{/each}
			</div>
		</div>

		<div class="grid gap-4 lg:grid-cols-[minmax(0,1.25fr)_minmax(0,1fr)]">
			<div class="space-y-2">
				<div class="flex items-center justify-between gap-3">
					<span class="label-mono">First Attendance Day</span>
					<span class="text-xs font-medium text-muted-foreground">{sf2FirstAttendanceLabel}</span>
				</div>
				<div class="rounded-md border border-border bg-background p-3">
					<div class="grid grid-cols-7 gap-1 pb-2">
						{#each SF2_CALENDAR_WEEKDAYS as weekday (weekday)}
							<div class="text-center text-[0.68rem] font-semibold text-muted-foreground uppercase">
								{weekday}
							</div>
						{/each}
					</div>
					<div class="grid grid-cols-7 gap-1">
						{#each sf2FirstAttendanceCalendar as cell (cell.key)}
							{#if cell.day === null}
								<div class="h-9 rounded-md"></div>
							{:else}
								<button
									type="button"
									disabled={!cell.isSchoolDay}
									aria-pressed={cell.isSelected}
									onclick={() => selectSf2FirstSchoolDay(cell.day)}
									class={`h-9 rounded-md border text-sm font-medium transition-colors ${
										cell.isSelected
											? 'border-primary bg-primary text-primary-foreground shadow-sm'
											: cell.isSchoolDay
												? 'border-border bg-surface hover:border-primary hover:bg-background'
												: 'cursor-not-allowed border-transparent bg-transparent text-muted-foreground/50'
									}`}
								>
									{cell.label}
								</button>
							{/if}
						{/each}
					</div>
				</div>
			</div>
			<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-1">
				<div class="space-y-1.5">
					<label for="sf2GradeLevel" class="label-mono">Grade Level</label>
					<input
						id="sf2GradeLevel"
						bind:value={sf2DraftGradeLevel}
						required
						class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
				<div class="space-y-1.5">
					<label for="sf2Section" class="label-mono">Section</label>
					<input
						id="sf2Section"
						bind:value={sf2DraftSection}
						required
						class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
			</div>
		</div>

		<div class="grid gap-4 sm:grid-cols-2">
			<div class="space-y-1.5">
				<label for="sf2AdviserName" class="label-mono">Adviser / LIS Name</label>
				<input
					id="sf2AdviserName"
					bind:value={sf2DraftAdviserName}
					required
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
			<div class="space-y-1.5">
				<label for="sf2SchoolHeadName" class="label-mono">School Head Name</label>
				<input
					id="sf2SchoolHeadName"
					bind:value={sf2DraftSchoolHeadName}
					required
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
		</div>

		<div class="flex justify-end gap-2 pt-2">
			<button
				type="button"
				onclick={() => (sf2TemplateDialogOpen = false)}
				class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
			>
				Cancel
			</button>
			<button
				type="submit"
				disabled={sf2TemplateCreating || sf2SettingsSaving}
				class="rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if sf2TemplateCreating}
					Creating...
				{:else if sf2SettingsSaving}
					Saving...
				{:else}
					{sf2TemplateDialogMode === 'create' ? 'Create Workbook' : 'Save Workbook Settings'}
				{/if}
			</button>
		</div>
	</form>
</Dialog>

<!-- ── Quarter Dialog ───────────────────────────────────────────────────────── -->
<Dialog
	open={quarterDialogOpen}
	title="School Year Quarters"
	description="Set the current quarter and define the start/end dates for each period."
	onClose={() => (quarterDialogOpen = false)}
>
	<div class="space-y-6">
		<div class="space-y-2">
			<label for="currentQuarter" class="label-mono">Active Quarter</label>
			<select
				id="currentQuarter"
				bind:value={defaultQuarter}
				class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			>
				<option value="1st Quarter">1st Quarter</option>
				<option value="2nd Quarter">2nd Quarter</option>
				<option value="3rd Quarter">3rd Quarter</option>
			</select>
		</div>

		<div class="space-y-4">
			<h3 class="text-sm font-semibold tracking-wider text-muted-foreground uppercase">
				Quarter Dates
			</h3>

			<div class="grid grid-cols-2 gap-4">
				<div class="space-y-1">
					<label for="q1Start" class="text-xs font-medium text-muted-foreground">Q1 Start</label>
					<input
						id="q1Start"
						type="date"
						bind:value={q1Start}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
				<div class="space-y-1">
					<label for="q1End" class="text-xs font-medium text-muted-foreground">Q1 End</label>
					<input
						id="q1End"
						type="date"
						bind:value={q1End}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
			</div>

			<div class="grid grid-cols-2 gap-4">
				<div class="space-y-1">
					<label for="q2Start" class="text-xs font-medium text-muted-foreground">Q2 Start</label>
					<input
						id="q2Start"
						type="date"
						bind:value={q2Start}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
				<div class="space-y-1">
					<label for="q2End" class="text-xs font-medium text-muted-foreground">Q2 End</label>
					<input
						id="q2End"
						type="date"
						bind:value={q2End}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
			</div>

			<div class="grid grid-cols-2 gap-4">
				<div class="space-y-1">
					<label for="q3Start" class="text-xs font-medium text-muted-foreground">Q3 Start</label>
					<input
						id="q3Start"
						type="date"
						bind:value={q3Start}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
				<div class="space-y-1">
					<label for="q3End" class="text-xs font-medium text-muted-foreground">Q3 End</label>
					<input
						id="q3End"
						type="date"
						bind:value={q3End}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
			</div>
		</div>

		<button
			onclick={() => (quarterDialogOpen = false)}
			class="w-full rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
		>
			Done
		</button>
	</div>
</Dialog>

<!-- ── Class Dialog ───────────────────────────────────────────────────────── -->
<Dialog
	open={classDialogOpen}
	title={editingClass ? 'Edit Class' : 'Add New Class'}
	description="Define the schedule for this specific grade or section."
	onClose={() => (classDialogOpen = false)}
>
	<form onsubmit={onSaveClass} class="space-y-4">
		<div class="grid grid-cols-2 gap-4">
			<div class="space-y-1.5">
				<label for="className" class="label-mono">Class Name</label>
				<input
					id="className"
					bind:value={formClassName}
					placeholder=""
					required
					class="w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
			<div class="space-y-1.5">
				<label for="room" class="label-mono"
					>Room <span class="font-normal text-muted-foreground">(optional)</span></label
				>
				<input
					id="room"
					bind:value={formRoom}
					placeholder=" "
					class="w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
		</div>

		<!-- Days of Week Selector -->
		<fieldset class="space-y-1.5">
			<legend class="label-mono flex items-center justify-between">
				<span>Scheduled Days</span>
				<span class="text-[10px] font-medium tracking-wider text-muted-foreground uppercase">
					{getDaysLabel(formDays)}
				</span>
			</legend>
			<div class="flex justify-between gap-1">
				{#each ['S', 'M', 'T', 'W', 'T', 'F', 'S'] as day, i (i)}
					<button
						type="button"
						onclick={() => {
							if (formDays.includes(i)) {
								formDays = formDays.filter((d) => d !== i);
							} else {
								formDays = [...formDays, i].sort();
							}
						}}
						class="flex size-9 items-center justify-center rounded-md border text-xs font-semibold transition-colors
							{formDays.includes(i)
							? 'border-primary bg-primary text-primary-foreground'
							: 'border-border bg-background hover:bg-surface'}"
					>
						{day}{i === 4 ? 'H' : ''}
					</button>
				{/each}
			</div>
		</fieldset>

		<!-- Session Mode Selector -->
		<fieldset class="space-y-1.5">
			<legend class="label-mono">Session Mode</legend>
			<div class="flex gap-2">
				<button
					type="button"
					onclick={() => handleSessionModeChange('single')}
					class="flex-1 rounded-md border px-3 py-2 text-sm transition-colors {sessionMode ===
					'single'
						? 'border-primary bg-primary text-primary-foreground'
						: 'border-border bg-background hover:bg-surface'}"
				>
					Single Day
				</button>
				<button
					type="button"
					onclick={() => handleSessionModeChange('morning-afternoon')}
					class="flex-1 rounded-md border px-3 py-2 text-sm transition-colors {sessionMode ===
					'morning-afternoon'
						? 'border-primary bg-primary text-primary-foreground'
						: 'border-border bg-background hover:bg-surface'}"
				>
					Morning & Afternoon
				</button>
				<button
					type="button"
					onclick={() => (sessionMode = 'custom')}
					class="flex-1 rounded-md border px-3 py-2 text-sm transition-colors {sessionMode ===
					'custom'
						? 'border-primary bg-primary text-primary-foreground'
						: 'border-border bg-background hover:bg-surface'}"
				>
					Custom
				</button>
			</div>
		</fieldset>

		<!-- Sessions List -->
		<div class="space-y-3">
			<div class="flex items-center justify-between">
				<h4 class="label-mono text-xs text-muted-foreground uppercase">Sessions</h4>
				{#if sessionMode === 'custom'}
					<button
						type="button"
						onclick={addSession}
						class="text-xs font-medium text-accent hover:underline"
					>
						+ Add Session
					</button>
				{/if}
			</div>

			<div class="max-h-64 space-y-4 overflow-y-auto pr-1">
				{#each formSessions as session, i (i)}
					<div class="relative space-y-3 rounded-xl border border-border p-4">
						{#if sessionMode === 'custom' && formSessions.length > 1}
							<button
								type="button"
								aria-label="Remove session {i + 1}"
								onclick={() => removeSession(i)}
								class="absolute top-3 right-3 text-muted-foreground hover:text-destructive"
							>
								<svg
									class="size-4"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
								>
									<path d="M18 6L6 18M6 6l12 12" />
								</svg>
							</button>
						{/if}

						<div class="grid grid-cols-2 gap-4">
							<div class="space-y-1">
								<label class="text-xs font-medium text-muted-foreground">
									Session Name
									<input
										bind:value={session.name}
										placeholder="e.g. Morning"
										required
										readonly={sessionMode !== 'custom'}
										class="mt-1 w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
									/>
								</label>
							</div>
							<div class="space-y-1">
								<label class="text-xs font-medium text-muted-foreground">
									Late After
									<input
										type="time"
										bind:value={session.lateAfter}
										required
										class="mt-1 w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
									/>
								</label>
							</div>
						</div>

						<div class="grid grid-cols-2 gap-4">
							<div class="space-y-1">
								<label class="text-xs font-medium text-muted-foreground">
									Start Time
									<input
										type="time"
										bind:value={session.startTime}
										required
										class="mt-1 w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
									/>
								</label>
							</div>
							<div class="space-y-1">
								<label class="text-xs font-medium text-muted-foreground">
									End Time
									<input
										type="time"
										bind:value={session.endTime}
										required
										class="mt-1 w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
									/>
								</label>
							</div>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<div class="flex justify-end gap-2 pt-2">
			<button
				type="button"
				onclick={() => (classDialogOpen = false)}
				class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
			>
				Cancel
			</button>
			<button
				type="submit"
				class="rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
			>
				{editingClass ? 'Save Changes' : 'Create Class'}
			</button>
		</div>
	</form>
</Dialog>

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
				<div class="w-full text-left">
					<h2 id="delete-dialog-title" class="text-lg font-semibold">Delete class?</h2>
					<p class="mt-1 text-sm text-muted-foreground">
						<span class="font-medium text-foreground">{deleteTarget.name}</span> will be permanently removed.
						Students will remain but will be unassigned.
					</p>
					<p class="mt-4 text-xs leading-relaxed text-muted-foreground">
						<strong class="font-semibold text-accent">PROTIP:</strong>
						<span class="block">
							You can hold down <strong class="font-semibold">Shift</strong> when clicking the delete
							button to bypass this confirmation entirely.
						</span>
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
					onclick={() => confirmDeleteClass()}
					class="flex-1 rounded-pill bg-destructive px-4 py-2 text-sm font-medium text-white hover:opacity-90"
				>
					Delete
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- ── Wipe confirmation dialog ─────────────────────────────────────────── -->
{#if wipeTarget}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => (wipeTarget = false)}
		onkeydown={(e) => e.key === 'Escape' && (wipeTarget = false)}
	></div>

	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="wipe-dialog-title"
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
					<h2 id="wipe-dialog-title" class="text-lg font-semibold">Erase ALL data?</h2>
					<p class="mt-1 text-sm text-muted-foreground">
						This will permanently erase ALL students, events, classes, and settings. This action
						cannot be undone.
					</p>
				</div>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (wipeTarget = false)}
					class="flex-1 rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
				>
					Cancel
				</button>
				<button
					onclick={async () => {
						await wipeAll();
						await reload();
						toast('All data wiped');
						wipeTarget = false;
					}}
					class="flex-1 rounded-pill bg-destructive px-4 py-2 text-sm font-medium text-white hover:opacity-90"
				>
					Wipe All
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- ── Export Format Dialog ─────────────────────────────────────────────── -->
{#if exportDialogOpen}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => (exportDialogOpen = false)}
		onkeydown={(e) => e.key === 'Escape' && (exportDialogOpen = false)}
	></div>

	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="export-dialog-title"
	>
		<div
			class="w-full max-w-md space-y-5 rounded-2xl border border-border bg-background p-6 shadow-xl"
		>
			<div>
				<h2 id="export-dialog-title" class="text-lg font-semibold">Export Data</h2>
				<p class="mt-1 text-sm text-muted-foreground">
					Choose the format for your data export. You'll be able to select the save location.
				</p>
			</div>

			<div class="space-y-3">
				<label class="flex cursor-pointer items-center gap-3">
					<input
						type="radio"
						bind:group={exportFormat}
						value="json"
						class="text-primary focus:ring-primary"
					/>
					<div>
						<div class="font-medium">JSON Format</div>
						<div class="text-sm text-muted-foreground">
							Includes students, attendance records, classes, and system configuration
						</div>
					</div>
				</label>

				<label class="flex cursor-pointer items-center gap-3">
					<input
						type="radio"
						bind:group={exportFormat}
						value="database"
						class="text-primary focus:ring-primary"
					/>
					<div>
						<div class="font-medium">SQLite Database (.db)</div>
						<div class="text-sm text-muted-foreground">
							Complete database file, can be opened with SQLite tools
						</div>
					</div>
				</label>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (exportDialogOpen = false)}
					class="flex-1 rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
				>
					Cancel
				</button>
				<button
					onclick={onExport}
					class="flex-1 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-white hover:opacity-90"
				>
					Export
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- ── Toast ──────────────────────────────────────────────────────────────── -->
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
