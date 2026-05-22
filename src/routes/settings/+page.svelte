<script lang="ts">
	import { onMount } from 'svelte';
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
		importAll,
		wipeAll,
		type Settings,
		type Class,
		type Session
	} from '$lib/db-rust';

	// ── State ────────────────────────────────────────────────────────────────
	let classes = $state<Class[]>([]);

	// Global settings fields - derived from store
	let defaultDayStart = $state('08:30');
	let defaultDayEnd = $state('15:30');
	let defaultLateAfter = $state('08:45');
	let defaultQuarter = $state('1st Quarter');

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
			// Update form fields from the store
			if (settingsStore.settings) {
				defaultDayStart = settingsStore.settings.dayStart;
				defaultDayEnd = settingsStore.settings.dayEnd;
				defaultLateAfter = settingsStore.settings.lateAfter;
				defaultQuarter = settingsStore.settings.quarter;
				q1Start = settingsStore.settings.q1Start ?? '';
				q1End = settingsStore.settings.q1End ?? '';
				q2Start = settingsStore.settings.q2Start ?? '';
				q2End = settingsStore.settings.q2End ?? '';
				q3Start = settingsStore.settings.q3Start ?? '';
				q3End = settingsStore.settings.q3End ?? '';
			}
		} catch (err: unknown) {
			const msg = err instanceof Error ? err.message : 'Database error';
			toast(`Failed to load: ${msg}`, false);
		}
	}

	// ── Actions ──────────────────────────────────────────────────────────────
	async function onSaveGlobal(e: SubmitEvent) {
		e.preventDefault();
		try {
			const next: Settings = {
				id: 'app',
				dayStart: defaultDayStart,
				dayEnd: defaultDayEnd,
				lateAfter: defaultLateAfter,
				quarter: defaultQuarter,
				q1Start,
				q1End,
				q2Start,
				q2End,
				q3Start,
				q3End
			};
			await settingsStore.save(next);
			toast('Global configuration saved');
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Failed to save settings';
			toast(`Save failed: ${msg}`, false);
		}
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
			room: formRoom.trim() || undefined,
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
			const msg = error instanceof Error ? error.message : 'Export failed';
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
			const msg = err instanceof Error ? err.message : 'Unknown error';
			toast(`Import failed: ${msg}`, false);
		}
	}

	function handleFileChange(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (file) onImport(file);
		input.value = '';
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
		<div class="grid gap-8 px-6 py-10 md:px-12 lg:grid-cols-12">
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
			</div>

			<!-- ── Sidebar: Global Defaults ────────────────────────────────── -->
			<div class="space-y-6 lg:col-span-4">
				<form
					onsubmit={onSaveGlobal}
					class="space-y-5 rounded-2xl border border-border bg-card p-6"
				>
					<div class="space-y-1">
						<h3 class="text-lg font-medium">Global Defaults</h3>
						<p class="text-xs text-muted-foreground">Used as templates for new classes.</p>
					</div>

					<div class="space-y-4">
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

<!-- ── Quarter Dialog ───────────────────────────────────────────────────────── -->
<Dialog
	open={quarterDialogOpen}
	title="School Year Quarters"
	description="Set the current quarter and define the start/end dates for each period."
	on:close={() => (quarterDialogOpen = false)}
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
	on:close={() => (classDialogOpen = false)}
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
