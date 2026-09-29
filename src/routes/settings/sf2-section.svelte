<script lang="ts">
	import { sf2State } from './settings-state.svelte';
	import { sf2MonthLabel } from '$lib/features/settings/sf2-months';
	import { SF2_SCHOOL_START_DATE_PROMPT } from '$lib/features/settings/sf2-heal-toast';
	import { isSchoolStartDateValid } from '$lib/features/settings/sf2-months';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import { FileSpreadsheet, RefreshCw, CalendarDays, TriangleAlert } from 'lucide-svelte';

	const toneClasses: Record<string, string> = {
		ready: 'text-foreground',
		attention: 'text-destructive',
		pending: 'text-muted-foreground'
	};
</script>

<section id="sf2-workbooks" class="order-3 space-y-6 rounded-2xl border border-border bg-card p-6">
	<div>
		<h3 class="text-lg font-medium">SF2 Workbook</h3>
		<p class="mt-1 text-sm text-muted-foreground">
			One Excel file per month, each holding only that month. Nothing is imported by hand: the app
			checks this month's file against its own records every time it starts, and recovers any mark
			the file holds and the app does not.
		</p>
	</div>

	<!-- ── Classes started on (D16, §11.1, edge case E3) ────────────────────── -->
	<div class="space-y-3 rounded-xl border border-border bg-surface p-4">
		<label for="sf2-school-start-date" class="flex items-center gap-2 text-sm font-semibold">
			<CalendarDays class="size-4" aria-hidden="true" />
			Classes started on
		</label>
		<p class="text-xs text-muted-foreground">
			The real first day of classes. Every month file is dated from this one date, so a month's
			attendance grid starts on the right day without being set up twelve times.
		</p>
		<div class="flex flex-wrap items-center gap-3">
			<input
				id="sf2-school-start-date"
				type="date"
				bind:value={sf2State.schoolStartDate}
				disabled={sf2State.schoolStartDateLoading || sf2State.schoolStartDateSaving}
				aria-invalid={!isSchoolStartDateValid(sf2State.schoolStartDate)}
				aria-describedby="sf2-school-start-date-help"
				class="rounded-pill border border-border bg-background px-4 py-2 text-sm disabled:opacity-60"
			/>
			<button
				type="button"
				onclick={() => sf2State.saveSchoolStartDate()}
				disabled={sf2State.schoolStartDateLoading ||
					sf2State.schoolStartDateSaving ||
					!isSchoolStartDateValid(sf2State.schoolStartDate)}
				class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if sf2State.schoolStartDateSaving}
					<Spinner />
				{/if}
				{sf2State.schoolStartDateSaving ? 'Saving…' : 'Save date'}
			</button>
			{#if sf2State.schoolStartDateLoading}
				<span class="text-xs text-muted-foreground">Loading…</span>
			{/if}
		</div>
		<p id="sf2-school-start-date-help" class="text-xs text-muted-foreground">
			{#if !isSchoolStartDateValid(sf2State.schoolStartDate)}
				<span class="text-destructive">Enter the date as YYYY-MM-DD.</span>
			{:else if sf2State.showSchoolStartDatePrompt}
				{SF2_SCHOOL_START_DATE_PROMPT}
			{:else if sf2State.schoolStartDate}
				Set to <span class="font-mono">{sf2State.schoolStartDate}</span>. Clear the field to unset
				it.
			{:else}
				Not set. Months fall back to their own first day until this is entered.
			{/if}
		</p>
	</div>

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
				No SF2 month workbooks yet. Use <em>Re-run the workbook split</em> below to build them from the
				original workbook.
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
					Splits the original workbook into one file per month. Safe to run more than once: a month
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
