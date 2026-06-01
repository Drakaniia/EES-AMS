<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import {
		exportSf2Workbook,
		getSf2ExportReadiness,
		listClasses,
		openSf2Workbook,
		type Class,
		type Sf2ExportReadiness
	} from '$lib/db-rust';
	import { ExternalLink, RefreshCw, Save } from 'lucide-svelte';

	let classes = $state<Class[]>([]);
	let selectedClassId = $state('');
	let readiness = $state<Sf2ExportReadiness | null>(null);
	let loading = $state(true);
	let exporting = $state(false);
	let opening = $state(false);
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	onMount(async () => {
		await loadInitial();
	});

	const selectedClass = $derived(classes.find((item) => item.id === selectedClassId));
	const activeClassId = $derived(selectedClassId || readiness?.template?.classId || '');

	async function loadInitial() {
		loading = true;
		try {
			classes = await listClasses();
			const current = await getSf2ExportReadiness();
			readiness = current;
			selectedClassId = current.template?.classId ?? classes[0]?.id ?? '';
			if (selectedClassId && selectedClassId !== current.template?.classId) {
				readiness = await getSf2ExportReadiness(selectedClassId);
			}
		} catch (error) {
			const msg = errorMessage(error, 'Failed to load reports');
			toast(`Reports failed: ${msg}`, false);
		} finally {
			loading = false;
		}
	}

	async function loadReadiness() {
		if (!selectedClassId) {
			readiness = await getSf2ExportReadiness();
			return;
		}
		readiness = await getSf2ExportReadiness(selectedClassId);
	}

	async function onClassChange() {
		loading = true;
		try {
			await loadReadiness();
		} catch (error) {
			const msg = errorMessage(error, 'Failed to load SF2 status');
			toast(`SF2 status failed: ${msg}`, false);
		} finally {
			loading = false;
		}
	}

	async function onExportSf2() {
		if (!activeClassId || !readiness?.canExport || exporting) return;
		const warnings = readiness.warnings ?? [];
		if (warnings.length > 0) {
			const confirmed = confirm(
				`The SF2 export is missing these workbook details:\n\n- ${warnings.join('\n- ')}\n\nAre you sure you want to export?`
			);
			if (!confirmed) return;
		}

		exporting = true;
		try {
			const result = await exportSf2Workbook(activeClassId);
			toast(`SF2 exported to ${result.outputPath}`);
			await loadReadiness();
		} catch (error) {
			const msg = errorMessage(error, 'SF2 export failed');
			toast(`SF2 export failed: ${msg}`, false);
		} finally {
			exporting = false;
		}
	}

	async function onOpenSf2() {
		if (!activeClassId || !readiness?.template || opening) return;
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
</script>

<svelte:head>
	<title>Reports — Attendance System</title>
	<meta name="description" content="Preview and export DepEd SF2 Excel reports." />
</svelte:head>

<AppShell>
	<div class="flex h-full flex-col overflow-hidden">
		<PageHeader
			category="Reports"
			title="DepEd SF2 Workbook"
			description="Preview the app's SF2 working copy, then export it to your chosen folder."
		>
			{#snippet actions()}
				<div class="flex flex-wrap items-center gap-3">
					<select
						aria-label="Class"
						bind:value={selectedClassId}
						onchange={onClassChange}
						class="h-10 min-w-56 rounded-pill border border-border bg-background px-4 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					>
						<option value="">Latest SF2 template</option>
						{#each classes as item (item.id)}
							<option value={item.id}>{item.name}</option>
						{/each}
					</select>
					<button
						onclick={loadInitial}
						class="inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
					>
						<RefreshCw class="size-4" />
						Refresh
					</button>
					<button
						onclick={onOpenSf2}
						disabled={!readiness?.template || opening || !activeClassId}
						class="inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
					>
						<ExternalLink class="size-4" />
						{opening ? 'Opening...' : 'Open SF2'}
					</button>
					<button
						onclick={onExportSf2}
						disabled={!readiness?.canExport || exporting || !activeClassId}
						class="inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-5 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
					>
						<Save class="size-4" />
						{exporting ? 'Exporting...' : 'Export Copy'}
					</button>
				</div>
			{/snippet}
		</PageHeader>

		{#if loading}
			<div class="px-6 py-12 text-sm text-muted-foreground md:px-12">Loading SF2 status...</div>
		{:else}
			<section class="grid gap-6 px-6 py-5 md:px-12 xl:grid-cols-[1fr_0.75fr]">
				<div class="space-y-6">
					<div class="rounded-2xl border border-border bg-card p-6">
						<div class="flex flex-wrap items-start justify-between gap-4">
							<div>
								<div class="label-mono text-primary">Workbook template</div>
								<h2 class="mt-2 text-2xl font-semibold">
									{readiness?.template
										? `${readiness.template.gradeLevel} - ${readiness.template.section}`
										: 'No SF2 workbook imported'}
								</h2>
								<p class="mt-2 text-sm text-muted-foreground">
									{#if readiness?.template}
										{readiness.template.schoolYear} · {selectedClass?.name ?? 'Linked class'}
									{:else}
										Import an SF2 workbook or create one from the bundled template in Settings.
									{/if}
								</p>
							</div>

							<div
								class="rounded-pill border px-3 py-1 text-xs font-medium {readiness?.canExport
									? 'border-primary bg-primary/10 text-primary'
									: 'border-border bg-surface text-muted-foreground'}"
							>
								{readiness?.canExport ? 'Ready' : 'Needs attention'}
							</div>
						</div>

						<div class="mt-6 grid gap-3 sm:grid-cols-3">
							<div class="rounded-xl border border-border bg-surface p-4">
								<div class="label-mono">Mapped learners</div>
								<div class="mt-2 text-3xl font-semibold">{readiness?.mappedStudents ?? 0}</div>
							</div>
							<div class="rounded-xl border border-border bg-surface p-4">
								<div class="label-mono">Mapped dates</div>
								<div class="mt-2 text-3xl font-semibold">{readiness?.mappedDates ?? 0}</div>
							</div>
							<div class="rounded-xl border border-border bg-surface p-4">
								<div class="label-mono">Closed days</div>
								<div class="mt-2 text-3xl font-semibold">{readiness?.closedDays.length ?? 0}</div>
							</div>
						</div>
					</div>

					<div class="rounded-2xl border border-border bg-card p-6">
						<div class="label-mono">Closed attendance days</div>
						{#if readiness && readiness.closedDays.length > 0}
							<div class="mt-4 flex flex-wrap gap-2">
								{#each readiness.closedDays as day (day)}
									<span
										class="rounded-pill border border-border bg-surface px-3 py-1 font-mono text-xs"
									>
										{day}
									</span>
								{/each}
							</div>
						{:else}
							<p class="mt-4 text-sm text-muted-foreground">
								End a class attendance session to write that day's marks into the SF2 working copy.
							</p>
						{/if}
					</div>
				</div>

				<aside class="rounded-2xl border border-border bg-surface p-6">
					<div class="label-mono">Export checks</div>
					{#if readiness && readiness.issues.length > 0}
						<ul class="mt-4 space-y-3 text-sm">
							{#each readiness.issues as issue (issue)}
								<li class="rounded-xl border border-border bg-background p-3 text-muted-foreground">
									{issue}
								</li>
							{/each}
						</ul>
					{:else if readiness && readiness.warnings.length > 0}
						<div
							class="mt-4 rounded-xl border border-primary/30 bg-background p-4 text-sm text-muted-foreground"
						>
							Export is available, but the app will ask for confirmation because these SF2 workbook
							details are blank:
						</div>
						<ul class="mt-3 space-y-2 text-sm">
							{#each readiness.warnings as warning (warning)}
								<li class="rounded-xl border border-border bg-background p-3 text-muted-foreground">
									{warning}
								</li>
							{/each}
						</ul>
					{:else}
						<div
							class="mt-4 rounded-xl border border-primary/40 bg-primary/10 p-4 text-sm text-primary"
						>
							The workbook and date cells are ready. Export copies the current saved SF2 working
							file.
						</div>
					{/if}
				</aside>
			</section>
		{/if}
	</div>
</AppShell>

{#if toastMessage}
	<div
		class="fixed top-12 right-6 z-60 rounded-xl border px-4 py-3 text-sm font-medium shadow-lg {toastOk
			? 'border-border bg-background'
			: 'border-destructive/40 bg-destructive/10 text-destructive'}"
		role="status"
		aria-live="polite"
	>
		{toastMessage}
	</div>
{/if}
