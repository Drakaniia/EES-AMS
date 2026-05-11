<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import {
		getSettings,
		saveSettings,
		listClasses,
		saveClass,
		deleteClass,
		exportDatabase,
		exportJsonWithFolder,
		importAll,
		wipeAll,
		type Settings,
		type Class
	} from '$lib/db-rust';

	// ── State ────────────────────────────────────────────────────────────────
	let settings = $state<Settings | null>(null);
	let classes = $state<Class[]>([]);

	// Global settings fields
	let defaultDayStart = $state('08:30');
	let defaultDayEnd = $state('15:30');
	let defaultLateAfter = $state('08:45');

	// Class Dialog state
	let classDialogOpen = $state(false);
	let editingClass = $state<Class | null>(null);
	let formClassName = $state('');
	let formDayStart = $state('');
	let formDayEnd = $state('');
	let formLateAfter = $state('');

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
			const [s, c] = await Promise.all([getSettings(), listClasses()]);
			settings = s;
			classes = c;
			if (s) {
				defaultDayStart = s.dayStart;
				defaultDayEnd = s.dayEnd;
				defaultLateAfter = s.lateAfter;
			}
		} catch (err: unknown) {
			const msg = err instanceof Error ? err.message : 'Database error';
			toast(`Failed to load: ${msg}`, false);
			// Set a fallback state so it doesn't spin forever
			settings = settings || { id: 'app', dayStart: '08:30', dayEnd: '15:30', lateAfter: '08:45' };
		}
	}

	// ── Actions ──────────────────────────────────────────────────────────────
	async function onSaveGlobal(e: SubmitEvent) {
		e.preventDefault();
		const next: Settings = {
			id: 'app',
			dayStart: defaultDayStart,
			dayEnd: defaultDayEnd,
			lateAfter: defaultLateAfter
		};
		await saveSettings(next);
		settings = next;
		toast('Global defaults saved');
	}

	function openAddClass() {
		editingClass = null;
		formClassName = '';
		formDayStart = defaultDayStart;
		formDayEnd = defaultDayEnd;
		formLateAfter = defaultLateAfter;
		classDialogOpen = true;
	}

	function openEditClass(c: Class) {
		editingClass = c;
		formClassName = c.name;
		formDayStart = c.dayStart;
		formDayEnd = c.dayEnd;
		formLateAfter = c.lateAfter;
		classDialogOpen = true;
	}

	async function onSaveClass(e: SubmitEvent) {
		e.preventDefault();
		const name = formClassName.trim();
		if (!name) return;

		const c: Class = {
			id: editingClass?.id ?? '',
			name,
			dayStart: formDayStart,
			dayEnd: formDayEnd,
			lateAfter: formLateAfter,
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

	async function onDeleteClass(id: string) {
		const classToDelete = classes.find((c) => c.id === id);
		if (classToDelete) {
			deleteTarget = { id: classToDelete.id, name: classToDelete.name };
		}
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

	{#if settings === null}
		<div class="text-muted-foreground px-6 py-12 text-sm md:px-12">Loading…</div>
	{:else}
		<div class="grid gap-8 px-6 py-10 md:px-12 lg:grid-cols-12">
			<!-- ── Class Management ────────────────────────────────────────── -->
			<div class="space-y-6 lg:col-span-8">
				<section class="border-border bg-card overflow-hidden rounded-2xl border">
					<div class="flex items-center justify-between p-6 pb-4">
						<h3 class="text-lg font-medium">Classes & Schedule</h3>
						<button
							onclick={openAddClass}
							class="rounded-pill bg-primary text-primary-foreground hover:bg-accent inline-flex items-center gap-2 px-4 py-2 text-sm font-medium transition-colors"
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

					<div class="divide-border border-border divide-y border-t pt-5">
						{#if classes.length === 0}
							<div class="text-muted-foreground p-12 text-center text-sm">
								No classes configured. Add a class to start tracking attendance.
							</div>
						{:else}
							{#each classes as c (c.id)}
								<div
									class="hover:bg-surface flex items-center justify-between p-6 transition-colors"
								>
									<div class="space-y-1">
										<div class="font-medium">{c.name}</div>
										<div class="text-muted-foreground label-mono flex gap-4 text-xs">
											<span>{c.dayStart} – {c.dayEnd}</span>
											<span class="text-accent">Late after {c.lateAfter}</span>
										</div>
									</div>
									<div class="flex gap-2">
										<button
											onclick={() => openEditClass(c)}
											class="border-border bg-background hover:bg-surface inline-flex size-9 items-center justify-center rounded-md border transition-colors"
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
											onclick={() => onDeleteClass(c.id)}
											class="border-border bg-background hover:bg-surface text-destructive inline-flex size-9 items-center justify-center rounded-md border transition-colors"
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
				<section class="border-border bg-card space-y-5 rounded-2xl border p-6">
					<h3 class="text-lg font-medium">Data Management</h3>
					<p class="text-muted-foreground text-sm">
						Your data is stored locally. Use backups to transfer data between devices or browsers.
					</p>

					<div class="flex flex-wrap gap-2">
						<button
							onclick={openExportDialog}
							class="rounded-pill border-border bg-background hover:bg-surface inline-flex items-center gap-2 border px-4 py-2 text-sm font-medium transition-colors"
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
							class="rounded-pill border-border bg-background hover:bg-surface inline-flex items-center gap-2 border px-4 py-2 text-sm font-medium transition-colors"
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

					<div class="border-border space-y-3 border-t pt-5">
						<button
							onclick={onWipe}
							class="rounded-pill border-destructive/40 text-destructive hover:bg-destructive/10 inline-flex items-center gap-2 border px-4 py-2 text-sm font-medium transition-colors"
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
					class="border-border bg-card space-y-5 rounded-2xl border p-6"
				>
					<div class="space-y-1">
						<h3 class="text-lg font-medium">Global Defaults</h3>
						<p class="text-muted-foreground text-xs">Used as templates for new classes.</p>
					</div>

					<div class="space-y-4">
						<div class="space-y-2">
							<label for="defDayStart" class="label-mono">Default Day Start</label>
							<input
								id="defDayStart"
								type="time"
								bind:value={defaultDayStart}
								class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
							/>
						</div>
						<div class="space-y-2">
							<label for="defDayEnd" class="label-mono">Default Day End</label>
							<input
								id="defDayEnd"
								type="time"
								bind:value={defaultDayEnd}
								class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
							/>
						</div>
						<div class="space-y-2">
							<label for="defLateAfter" class="label-mono">Default Late After</label>
							<input
								id="defLateAfter"
								type="time"
								bind:value={defaultLateAfter}
								class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
							/>
						</div>
					</div>

					<button
						type="submit"
						class="rounded-pill bg-primary text-primary-foreground hover:bg-accent w-full px-4 py-2 text-sm font-medium transition-colors"
					>
						Save Defaults
					</button>
				</form>
			</div>
		</div>
	{/if}
</AppShell>

<!-- ── Class Dialog ───────────────────────────────────────────────────────── -->
<Dialog
	open={classDialogOpen}
	title={editingClass ? 'Edit Class' : 'Add New Class'}
	description="Define the schedule for this specific grade or section."
	on:close={() => (classDialogOpen = false)}
>
	<form onsubmit={onSaveClass} class="space-y-4">
		<div class="space-y-1.5">
			<label for="className" class="label-mono">Class Name</label>
			<input
				id="className"
				bind:value={formClassName}
				placeholder="e.g. Grade 6 - Apple"
				required
				class="border-border bg-background focus:ring-primary w-full rounded-md border px-3 py-2 text-sm focus:ring-2 focus:outline-none"
			/>
		</div>

		<div class="grid grid-cols-3 gap-4">
			<div class="space-y-1.5">
				<label for="dayStart" class="label-mono">Start</label>
				<input
					id="dayStart"
					type="time"
					bind:value={formDayStart}
					required
					class="border-border bg-background focus:ring-primary w-full rounded-md border px-3 py-2 text-sm focus:ring-2 focus:outline-none"
				/>
			</div>
			<div class="space-y-1.5">
				<label for="dayEnd" class="label-mono">End</label>
				<input
					id="dayEnd"
					type="time"
					bind:value={formDayEnd}
					required
					class="border-border bg-background focus:ring-primary w-full rounded-md border px-3 py-2 text-sm focus:ring-2 focus:outline-none"
				/>
			</div>
			<div class="space-y-1.5">
				<label for="lateAfter" class="label-mono">Late After</label>
				<input
					id="lateAfter"
					type="time"
					bind:value={formLateAfter}
					required
					class="border-border bg-background focus:ring-primary w-full rounded-md border px-3 py-2 text-sm focus:ring-2 focus:outline-none"
				/>
			</div>
		</div>

		<div class="flex justify-end gap-2 pt-2">
			<button
				type="button"
				onclick={() => (classDialogOpen = false)}
				class="border-border hover:bg-surface rounded-md border px-4 py-2 text-sm transition-colors"
			>
				Cancel
			</button>
			<button
				type="submit"
				class="rounded-pill bg-primary text-primary-foreground hover:bg-accent px-4 py-2 text-sm font-medium transition-colors"
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
			class="border-border bg-background w-full max-w-sm space-y-5 rounded-2xl border p-6 shadow-xl"
		>
			<div class="flex flex-col items-center gap-3 text-center">
				<div class="bg-destructive/10 flex size-12 items-center justify-center rounded-full">
					<svg
						class="text-destructive size-6"
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
					<h2 id="delete-dialog-title" class="text-lg font-semibold">Delete class?</h2>
					<p class="text-muted-foreground mt-1 text-sm">
						<span class="text-foreground font-medium">{deleteTarget.name}</span> will be permanently removed.
						Students will remain but will be unassigned.
					</p>
				</div>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (deleteTarget = null)}
					class="border-border hover:bg-surface flex-1 rounded-md border px-4 py-2 text-sm transition-colors"
				>
					Cancel
				</button>
				<button
					onclick={async () => {
						if (!deleteTarget) return;
						await deleteClass(deleteTarget.id);
						toast('Class deleted');
						deleteTarget = null;
						reload();
					}}
					class="rounded-pill bg-destructive flex-1 px-4 py-2 text-sm font-medium text-white hover:opacity-90"
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
			class="border-border bg-background w-full max-w-sm space-y-5 rounded-2xl border p-6 shadow-xl"
		>
			<div class="flex flex-col items-center gap-3 text-center">
				<div class="bg-destructive/10 flex size-12 items-center justify-center rounded-full">
					<svg
						class="text-destructive size-6"
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
					<p class="text-muted-foreground mt-1 text-sm">
						This will permanently erase ALL students, events, classes, and settings. This action
						cannot be undone.
					</p>
				</div>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (wipeTarget = false)}
					class="border-border hover:bg-surface flex-1 rounded-md border px-4 py-2 text-sm transition-colors"
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
					class="rounded-pill bg-destructive flex-1 px-4 py-2 text-sm font-medium text-white hover:opacity-90"
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
			class="border-border bg-background w-full max-w-md space-y-5 rounded-2xl border p-6 shadow-xl"
		>
			<div>
				<h2 id="export-dialog-title" class="text-lg font-semibold">Export Data</h2>
				<p class="text-muted-foreground mt-1 text-sm">
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
						<div class="text-muted-foreground text-sm">
							Human-readable format, easy to share and import back into the system
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
						<div class="text-muted-foreground text-sm">
							Complete database file, can be opened with SQLite tools
						</div>
					</div>
				</label>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (exportDialogOpen = false)}
					class="border-border hover:bg-surface flex-1 rounded-md border px-4 py-2 text-sm transition-colors"
				>
					Cancel
				</button>
				<button
					onclick={onExport}
					class="rounded-pill bg-primary flex-1 px-4 py-2 text-sm font-medium text-white hover:opacity-90"
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
		class="fixed right-6 bottom-6 z-50 rounded-xl border px-4 py-3 text-sm font-medium shadow-lg
			{toastOk
			? 'bg-background border-border text-foreground'
			: 'bg-destructive/10 border-destructive/40 text-destructive'}"
		role="status"
		aria-live="polite"
	>
		{toastMessage}
	</div>
{/if}
