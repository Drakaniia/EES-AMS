<script lang="ts">
	import { sf2State } from './settings-state.svelte';
	import { sf2ImportState } from './sf2-import-state.svelte';
	import Sf2ImportDialog from './sf2-import-dialog.svelte';
	import { sf2MonthLabel } from '$lib/features/settings/sf2-months';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import { FileSpreadsheet, RefreshCw, TriangleAlert } from 'lucide-svelte';

	const toneClasses: Record<string, string> = {
		ready: 'text-foreground',
		attention: 'text-destructive',
		pending: 'text-muted-foreground'
	};

	/** The grade levels the create form offers — the elementary school's own six. */
	const GRADE_LEVELS = ['1', '2', '3', '4', '5', '6'];
</script>

<section id="sf2-workbooks" class="order-3 space-y-6 rounded-2xl border border-border bg-card p-6">
	<div>
		<h3 class="text-lg font-medium">SF2 Workbook</h3>
		<p class="mt-1 text-sm text-muted-foreground">
			One Excel file per month, each holding only that month. Start a class from the template or
			adopt a school workbook above; the app also checks this month's file against its own records
			every time it starts, and recovers any mark the file holds and the app does not.
		</p>
	</div>

	<!-- ── Start / Import ───────────────────────────────────────────────────── -->
	<div class="space-y-3 rounded-xl border border-border bg-surface p-4">
		<div>
			<h4 class="text-sm font-semibold">Start, import</h4>
			<p class="mt-0.5 text-xs text-muted-foreground">
				Create a fresh workbook from the DepEd template, or adopt a school workbook as the class
				record. The roster follows the Students page on its own - a student added there is on the
				grid here without a step in between.
			</p>
		</div>
		<div class="flex flex-wrap items-center gap-2">
			<label class="flex items-center gap-2 text-xs">
				Class
				<select
					bind:value={sf2ImportState.selectedClassId}
					class="rounded-pill border border-border bg-background px-3 py-2 text-sm"
				>
					<option value="">Derive from grade + section</option>
					{#each sf2ImportState.availableClasses as c (c.id)}
						<option value={c.id}>{c.name}</option>
					{/each}
				</select>
			</label>
			<button
				type="button"
				onclick={() => (sf2ImportState.showCreateForm = !sf2ImportState.showCreateForm)}
				class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
			>
				<FileSpreadsheet class="size-4" aria-hidden="true" />
				Create from template
			</button>
			<button
				type="button"
				onclick={() => sf2ImportState.onPickImportFile()}
				disabled={sf2ImportState.staging}
				class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if sf2ImportState.staging}
					<Spinner />
				{/if}
				{sf2ImportState.staging ? 'Reading…' : 'Import workbook'}
			</button>
		</div>

		{#if sf2ImportState.showCreateForm}
			<div class="grid grid-cols-2 gap-3">
				<label class="text-xs">
					School ID
					<input
						type="text"
						bind:value={sf2ImportState.createDraft.schoolId}
						class="mt-1 w-full rounded-pill border border-border bg-background px-4 py-2 text-sm"
					/>
				</label>
				<label class="text-xs">
					School name
					<input
						type="text"
						bind:value={sf2ImportState.createDraft.schoolName}
						class="mt-1 w-full rounded-pill border border-border bg-background px-4 py-2 text-sm"
					/>
				</label>
				<label class="text-xs">
					School year
					<input
						type="text"
						bind:value={sf2ImportState.createDraft.schoolYear}
						placeholder="2026-2027"
						class="mt-1 w-full rounded-pill border border-border bg-background px-4 py-2 text-sm"
					/>
				</label>
				<label class="text-xs">
					Report month
					<input
						type="text"
						bind:value={sf2ImportState.createDraft.reportMonth}
						placeholder="SEPTEMBER"
						class="mt-1 w-full rounded-pill border border-border bg-background px-4 py-2 text-sm"
					/>
				</label>
				<label class="text-xs">
					Grade level
					<select
						bind:value={sf2ImportState.createDraft.gradeLevel}
						class="mt-1 w-full rounded-pill border border-border bg-background px-4 py-2 text-sm"
					>
						<option value="">Choose a grade</option>
						{#each GRADE_LEVELS as grade (grade)}
							<option value={grade}>{grade}</option>
						{/each}
					</select>
				</label>
				<label class="text-xs">
					Section
					<input
						type="text"
						bind:value={sf2ImportState.createDraft.section}
						class="mt-1 w-full rounded-pill border border-border bg-background px-4 py-2 text-sm"
					/>
				</label>
				<label class="text-xs">
					Adviser name
					<input
						type="text"
						bind:value={sf2ImportState.createDraft.adviserName}
						class="mt-1 w-full rounded-pill border border-border bg-background px-4 py-2 text-sm"
					/>
				</label>
				<label class="text-xs">
					School head name
					<input
						type="text"
						bind:value={sf2ImportState.createDraft.schoolHeadName}
						class="mt-1 w-full rounded-pill border border-border bg-background px-4 py-2 text-sm"
					/>
				</label>
			</div>
			<label class="block text-xs">
				Learner names, one per line
				<textarea
					bind:value={sf2ImportState.learnerNamesText}
					rows="4"
					class="mt-1 w-full rounded-xl border border-border bg-background px-4 py-2 text-sm"
				></textarea>
			</label>
			<div>
				<button
					type="button"
					onclick={() => sf2ImportState.onCreateFromTemplate()}
					disabled={sf2ImportState.creating}
					class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
				>
					{#if sf2ImportState.creating}
						<Spinner />
					{/if}
					{sf2ImportState.creating ? 'Creating…' : 'Create workbook'}
				</button>
			</div>
		{/if}
	</div>

	<Sf2ImportDialog
		open={sf2ImportState.importDialogOpen}
		validation={sf2ImportState.importReview?.validation ?? null}
		busy={sf2ImportState.importing}
		onProceed={() => sf2ImportState.onConfirmImport(true)}
		onCancel={() => sf2ImportState.onCancelImport()}
	/>

	<!-- ── Month workbooks (read-only) ──────────────────────────────────────── -->
	<div class="space-y-3">
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div>
				<h4 class="text-sm font-semibold">Month workbooks</h4>
				<p class="mt-0.5 text-xs text-muted-foreground">
					Twelve files, one per month, in school-year order. “Not measured” means nobody has counted
					the marks in that file yet — it does not mean the file is empty.
				</p>
			</div>
			<button
				type="button"
				onclick={() => sf2State.load()}
				disabled={sf2State.monthsLoading}
				title="Reload the month workbooks"
				class="inline-flex size-9 items-center justify-center rounded-md border border-border bg-background transition-colors hover:bg-surface disabled:opacity-60"
			>
				<RefreshCw class="size-4" aria-hidden="true" />
				<span class="sr-only">Reload the month workbooks</span>
			</button>
		</div>

		{#if sf2State.monthsLoading}
			<div class="flex items-center gap-2 text-sm text-muted-foreground">
				<Spinner />
				Reading the month workbooks…
			</div>
		{:else if sf2State.monthsError}
			<div
				class="flex items-start gap-2 rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900"
			>
				<TriangleAlert class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
				<div>
					The month workbooks could not be read: {sf2State.monthsError}. Nothing was changed; the
					files and the database are untouched.
				</div>
			</div>
		{:else if sf2State.monthRows.length === 0}
			<p class="text-sm text-muted-foreground">
				No SF2 month workbooks yet. Import a school workbook above and they are built automatically
				— <em>Re-run the workbook split</em> below is only the fallback when a month needs attention.
			</p>
		{:else}
			<div class="overflow-x-auto rounded-xl border border-border">
				<table class="w-full text-left text-sm">
					<thead
						class="border-b border-border bg-surface text-xs tracking-wide text-muted-foreground uppercase"
					>
						<tr>
							<th scope="col" class="px-4 py-3 font-semibold">Month</th>
							<th scope="col" class="px-4 py-3 font-semibold">File</th>
							<th scope="col" class="px-4 py-3 font-semibold">X marks</th>
							<th scope="col" class="px-4 py-3 font-semibold">Last sync</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-border">
						{#each sf2State.monthRows as row (row.month)}
							<tr>
								<td class="px-4 py-3">
									<div class="font-medium">{row.label}</div>
									<div class="font-mono text-[11px] text-muted-foreground">
										{#if row.undated}
											Not dated
										{:else if row.firstSchoolDayOverridden}
											Starts day {row.firstSchoolDay} (set by hand)
										{:else}
											Starts day {row.firstSchoolDay}
										{/if}
									</div>
								</td>
								<td class="px-4 py-3">
									<div class="font-mono text-[11px] text-muted-foreground">{row.fileName}</div>
									<div class="text-xs {toneClasses[row.tone] ?? ''}">{row.fileState}</div>
								</td>
								<td class="px-4 py-3 font-mono text-xs">
									{#if row.xCount === null}
										<span class="text-muted-foreground">Not measured</span>
									{:else}
										{row.xCount}
									{/if}
								</td>
								<td class="px-4 py-3 text-xs text-muted-foreground">{row.lastSynced}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<p class="text-xs text-muted-foreground">{sf2State.monthSummary}</p>
		{/if}
	</div>

	<!-- ── E1: today's month has no file ────────────────────────────────────── -->
	{#if sf2State.launch?.fellBack && sf2State.launch.todayCanCreate}
		<div
			class="space-y-2 rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900"
		>
			<div class="font-semibold">
				Showing {sf2MonthLabel(sf2State.launch.month)} — today's month is
				{sf2MonthLabel(sf2State.launch.todayMonth)} and it has no workbook yet.
			</div>
			<p>
				Create {sf2MonthLabel(sf2State.launch.todayMonth)} from Reports to switch to it. Nothing is created
				here: a month with no school days is never created for you.
			</p>
		</div>
	{/if}

	<!-- ── The split (§11, D14) ─────────────────────────────────────────────── -->
	<div class="space-y-3 border-t border-border pt-5">
		<div class="flex flex-wrap items-start justify-between gap-4">
			<div class="min-w-0">
				<h4 class="text-sm font-semibold">Re-run the workbook split</h4>
				<p class="mt-0.5 max-w-xl text-xs text-muted-foreground">
					Runs automatically on startup and after an import. Safe to run more than once: a month
					that is already split is left alone, so marks written into it are never overwritten. The
					original workbook is always kept.
				</p>
			</div>
			<button
				type="button"
				onclick={() => sf2State.onRunSplit()}
				disabled={sf2State.splitRunning}
				class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if sf2State.splitRunning}
					<Spinner />
				{:else}
					<FileSpreadsheet class="size-4" aria-hidden="true" />
				{/if}
				{sf2State.splitRunning ? 'Splitting…' : 'Re-run the workbook split'}
			</button>
		</div>
		{#if sf2State.splitSummary}
			<div
				class="rounded-xl border p-4 text-sm {sf2State.splitComplete
					? 'border-border bg-surface'
					: 'border-amber-200 bg-amber-50 text-amber-900'}"
			>
				<div>{sf2State.splitSummary}</div>
				{#if sf2State.splitNeedsAttention.length > 0}
					<div class="mt-2 text-xs">
						Needs attention: {sf2State.splitNeedsAttention.join(', ')}
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- ── Back up workbooks now (D13, acceptance #16) ──────────────────────── -->
	<div class="space-y-2 border-t border-border pt-5">
		<h4 class="text-sm font-semibold">Back up workbooks now</h4>
		<p class="max-w-xl text-xs text-muted-foreground">
			Writes a folder holding just the SF2 Excel files and a manifest of how many marks each one
			holds — no copy of the database. The button itself is in
			<a href="#settings-backup" class="underline">Data Management</a>, beside <em>Back Up Now</em>.
		</p>
	</div>
</section>
