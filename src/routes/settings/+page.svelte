<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import { getSettings, saveSettings, exportAll, importAll, wipeAll, type Settings } from '$lib/db';

	// ── State ────────────────────────────────────────────────────────────────
	let settings = $state<Settings | null>(null);

	// Form fields — kept in sync with loaded settings
	let className = $state('');
	let dayStart = $state('');
	let dayEnd = $state('');
	let lateAfter = $state('');

	// Toast
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	// Hidden file input reference
	let fileInput = $state<HTMLInputElement | null>(null);

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	function syncFields(s: Settings) {
		className = s.className;
		dayStart = s.dayStart;
		dayEnd = s.dayEnd;
		lateAfter = s.lateAfter;
	}

	// ── Actions ──────────────────────────────────────────────────────────────
	async function onSave(e: SubmitEvent) {
		e.preventDefault();
		const next: Settings = {
			id: 'app',
			className,
			dayStart,
			dayEnd,
			lateAfter
		};
		await saveSettings(next);
		settings = next;
		toast('Settings saved');
	}

	async function onExport() {
		const data = await exportAll();
		const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `horizon-backup-${Date.now()}.json`;
		a.click();
		URL.revokeObjectURL(url);
		toast('Backup downloaded');
	}

	async function onImport(file: File) {
		try {
			const txt = await file.text();
			const data = JSON.parse(txt);
			await importAll(data);
			const fresh = await getSettings();
			settings = fresh;
			syncFields(fresh);
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
		if (!confirm('Erase ALL students, events, and settings? This cannot be undone.')) return;
		await wipeAll();
		const fresh = await getSettings();
		settings = fresh;
		syncFields(fresh);
		toast('All data wiped');
	}

	// ── Lifecycle ────────────────────────────────────────────────────────────
	onMount(async () => {
		const s = await getSettings();
		settings = s;
		syncFields(s);
	});
</script>

<svelte:head>
	<title>Settings — Horizon Attendance</title>
	<meta name="description" content="Configure class details and back up your data." />
</svelte:head>

<AppShell>
	<PageHeader
		category="System Configuration"
		title="Workspace Settings"
		description="Define institutional parameters, manage data backups, and configure system-wide attendance rules."
	/>

	{#if settings === null}
		<div class="text-muted-foreground px-6 py-12 text-sm md:px-12">Loading…</div>
	{:else}
		<section class="grid gap-8 px-6 py-10 md:px-12 lg:grid-cols-2">
			<!-- ── Class & schedule ──────────────────────────────────────────── -->
			<form onsubmit={onSave} class="border-border bg-card space-y-5 rounded-2xl border p-6">
				<h3 class="text-lg font-medium">Class &amp; schedule</h3>

				<div class="space-y-2">
					<label for="className" class="label-mono">Class name</label>
					<input
						id="className"
						name="className"
						type="text"
						bind:value={className}
						class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
					/>
				</div>

				<div class="grid grid-cols-3 gap-4">
					<div class="space-y-2">
						<label for="dayStart" class="label-mono">Day start</label>
						<input
							id="dayStart"
							name="dayStart"
							type="time"
							bind:value={dayStart}
							class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
						/>
					</div>
					<div class="space-y-2">
						<label for="dayEnd" class="label-mono">Day end</label>
						<input
							id="dayEnd"
							name="dayEnd"
							type="time"
							bind:value={dayEnd}
							class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
						/>
					</div>
					<div class="space-y-2">
						<label for="lateAfter" class="label-mono">Late after</label>
						<input
							id="lateAfter"
							name="lateAfter"
							type="time"
							bind:value={lateAfter}
							class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
						/>
					</div>
				</div>

				<button
					type="submit"
					class="rounded-pill bg-primary text-primary-foreground hover:bg-accent inline-flex items-center gap-2 px-4 py-2 text-sm font-medium transition-colors"
				>
					Save settings
				</button>
			</form>

			<!-- ── Backups ───────────────────────────────────────────────────── -->
			<div class="border-border bg-card space-y-5 rounded-2xl border p-6">
				<h3 class="text-lg font-medium">Backups</h3>
				<p class="text-muted-foreground text-sm">
					Data is stored in this browser only. Export a JSON backup before clearing your browser,
					switching devices, or experimenting.
				</p>

				<div class="flex flex-wrap gap-2">
					<!-- Export -->
					<button
						onclick={onExport}
						class="rounded-pill border-border bg-background hover:bg-surface inline-flex items-center gap-2 border px-4 py-2 text-sm font-medium transition-colors"
					>
						<!-- Download icon -->
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
						Export backup
					</button>

					<!-- Import -->
					<button
						onclick={() => fileInput?.click()}
						class="rounded-pill border-border bg-background hover:bg-surface inline-flex items-center gap-2 border px-4 py-2 text-sm font-medium transition-colors"
					>
						<!-- Upload icon -->
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
							<polyline points="17 8 12 3 7 8" />
							<line x1="12" y1="3" x2="12" y2="15" />
						</svg>
						Import backup
					</button>

					<!-- Hidden file input -->
					<input
						bind:this={fileInput}
						type="file"
						accept="application/json"
						class="hidden"
						onchange={handleFileChange}
					/>
				</div>

				<!-- Danger zone -->
				<div class="border-border space-y-3 border-t pt-5">
					<div class="text-destructive flex items-start gap-3 text-sm">
						<!-- AlertTriangle icon -->
						<svg
							class="mt-0.5 size-4 shrink-0"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							aria-hidden="true"
						>
							<path
								d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
							/>
							<line x1="12" y1="9" x2="12" y2="13" />
							<line x1="12" y1="17" x2="12.01" y2="17" />
						</svg>
						<div>Permanently delete all students, events, and settings.</div>
					</div>
					<button
						onclick={onWipe}
						class="rounded-pill border-destructive/40 text-destructive hover:bg-destructive/10 inline-flex items-center gap-2 border px-4 py-2 text-sm font-medium transition-colors"
					>
						Wipe all data
					</button>
				</div>
			</div>
		</section>
	{/if}
</AppShell>

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
