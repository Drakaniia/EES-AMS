<script lang="ts">
	import { sf2State } from './settings-state.svelte';
	import { sf2ImportState } from './sf2-import-state.svelte';
	import Sf2ImportDialog from './sf2-import-dialog.svelte';
	import { sf2MonthLabel } from '$lib/features/settings/sf2-months';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import SettingsSection from './settings-section.svelte';
	import SettingsGroup from './settings-group.svelte';
	import SettingRow from './setting-row.svelte';
	import { RefreshCw, TriangleAlert } from 'lucide-svelte';

	let { onOpenData }: { onOpenData: () => void } = $props();

	const toneClasses: Record<string, string> = {
		ready: 'text-foreground',
		attention: 'text-destructive',
		pending: 'text-muted-foreground'
	};

	/** The grade levels the create form offers — the elementary school's own six. */
	const GRADE_LEVELS = ['1', '2', '3', '4', '5', '6'];

	const fieldClass = 'field';
	const labelClass = 'block text-[11px] font-medium text-muted-foreground';
</script>

<SettingsSection
	title="SF2 Workbook"
	description="One Excel file per month. The app keeps them in step with your records."
>
	{#snippet actions()}
		<button
			type="button"
			onclick={() => sf2State.load()}
			disabled={sf2State.monthsLoading}
			title="Reload the month workbooks"
			class="grid size-8 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-surface hover:text-foreground disabled:opacity-50"
		>
			<RefreshCw class="size-4" aria-hidden="true" />
			<span class="sr-only">Reload the month workbooks</span>
		</button>
	{/snippet}

	<SettingsGroup>
		<SettingRow label="Class" description="The roster follows the Class List page.">
			<select
				bind:value={sf2ImportState.selectedClassId}
				aria-label="Workbook class"
				class="field max-w-56"
			>
				<option value="">Derive from grade + section</option>
				{#each sf2ImportState.availableClasses as c (c.id)}
					<option value={c.id}>{c.name}</option>
				{/each}
			</select>
		</SettingRow>

		<SettingRow
			label="Create or import"
			description="Start from the DepEd template, or adopt a school workbook."
		>
			<button
				type="button"
				onclick={() => (sf2ImportState.showCreateForm = !sf2ImportState.showCreateForm)}
				class="btn btn-secondary btn-sm"
			>
				New workbook
			</button>
			<button
				type="button"
				onclick={() => sf2ImportState.onPickImportFile()}
				disabled={sf2ImportState.staging}
				class="btn btn-secondary btn-sm"
			>
				{#if sf2ImportState.staging}
					<Spinner />
				{/if}
				{sf2ImportState.staging ? 'Reading…' : 'Import'}
			</button>
		</SettingRow>

		{#if sf2ImportState.showCreateForm}
			<div class="space-y-3 bg-surface/40 px-4 py-4">
				<div class="grid gap-3 sm:grid-cols-2">
					<label class={labelClass}>
						School ID
						<input
							type="text"
							bind:value={sf2ImportState.createDraft.schoolId}
							class="{fieldClass} mt-1"
						/>
					</label>
					<label class={labelClass}>
						School name
						<input
							type="text"
							bind:value={sf2ImportState.createDraft.schoolName}
							class="{fieldClass} mt-1"
						/>
					</label>
					<label class={labelClass}>
						School year
						<input
							type="text"
							bind:value={sf2ImportState.createDraft.schoolYear}
							placeholder="2026-2027"
							class="{fieldClass} mt-1"
						/>
					</label>
					<label class={labelClass}>
						Report month
						<input
							type="text"
							bind:value={sf2ImportState.createDraft.reportMonth}
							placeholder="SEPTEMBER"
							class="{fieldClass} mt-1"
						/>
					</label>
					<label class={labelClass}>
						Grade level
						<select bind:value={sf2ImportState.createDraft.gradeLevel} class="{fieldClass} mt-1">
							<option value="">Choose a grade</option>
							{#each GRADE_LEVELS as grade (grade)}
								<option value={grade}>{grade}</option>
							{/each}
						</select>
					</label>
					<label class={labelClass}>
						Section
						<input
							type="text"
							bind:value={sf2ImportState.createDraft.section}
							class="{fieldClass} mt-1"
						/>
					</label>
					<label class={labelClass}>
						Adviser name
						<input
							type="text"
							bind:value={sf2ImportState.createDraft.adviserName}
							class="{fieldClass} mt-1"
						/>
					</label>
					<label class={labelClass}>
						School head
						<input
							type="text"
							bind:value={sf2ImportState.createDraft.schoolHeadName}
							class="{fieldClass} mt-1"
						/>
					</label>
				</div>

				<label class={labelClass}>
					Learner names, one per line
					<textarea
						bind:value={sf2ImportState.learnerNamesText}
						rows="4"
						class="textarea-field mt-1"
					></textarea>
				</label>

				<div class="flex justify-end">
					<button
						type="button"
						onclick={() => sf2ImportState.onCreateFromTemplate()}
						disabled={sf2ImportState.creating}
						class="btn btn-primary btn-sm"
					>
						{#if sf2ImportState.creating}
							<Spinner />
						{/if}
						{sf2ImportState.creating ? 'Creating…' : 'Create workbook'}
					</button>
				</div>
			</div>
		{/if}
	</SettingsGroup>

	<Sf2ImportDialog
		open={sf2ImportState.importDialogOpen}
		validation={sf2ImportState.importReview?.validation ?? null}
		busy={sf2ImportState.importing}
		onProceed={() => sf2ImportState.onConfirmImport(true)}
		onCancel={() => sf2ImportState.onCancelImport()}
	/>

	<!-- ── Month workbooks ─────────────────────────────────────────────────── -->
	<SettingsGroup>
		<div class="px-4 pt-3 pb-1">
			<p class="text-[13px] font-medium">Month workbooks</p>
			<p class="mt-0.5 text-xs text-muted-foreground">
				{#if sf2State.monthsLoading}
					Reading the month workbooks…
				{:else if sf2State.monthRows.length > 0}
					{sf2State.monthSummary}
				{:else}
					No month workbooks yet.
				{/if}
			</p>
		</div>

		{#if sf2State.monthsError}
			<div class="flex items-start gap-2 px-4 py-3 text-xs text-destructive">
				<TriangleAlert class="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
				<span>
					The month workbooks could not be read: {sf2State.monthsError}. Nothing was changed.
				</span>
			</div>
		{:else if !sf2State.monthsLoading && sf2State.monthRows.length === 0}
			<p class="px-4 py-3 text-xs text-muted-foreground">
				Import a school workbook above and the months are built automatically.
			</p>
		{:else}
			{#each sf2State.monthRows as row (row.month)}
				<div class="flex items-center justify-between gap-4 px-4 py-2">
					<div class="min-w-0">
						<p class="text-[13px] leading-5 font-medium">{row.label}</p>
						<p class="truncate font-mono text-[11px] leading-4 text-muted-foreground">
							<span class={toneClasses[row.tone] ?? ''}>{row.fileState}</span>
							<span class="mx-1">·</span>
							{#if row.undated}
								Not dated
							{:else}
								Starts day {row.firstSchoolDay}{row.firstSchoolDayOverridden ? ' (by hand)' : ''}
							{/if}
						</p>
					</div>
					<div class="shrink-0 text-right">
						<p class="font-mono text-xs">
							{#if row.xCount === null}
								<span class="text-muted-foreground">Not measured</span>
							{:else}
								{row.xCount} X
							{/if}
						</p>
						<p class="text-[11px] leading-4 text-muted-foreground">{row.lastSynced}</p>
					</div>
				</div>
			{/each}
		{/if}
	</SettingsGroup>

	<!-- ── E1: today's month has no file ────────────────────────────────────── -->
	{#if sf2State.launch?.fellBack && sf2State.launch.todayCanCreate}
		<div
			class="rounded-xl border border-amber-500/30 bg-amber-50/80 p-3.5 text-[12.5px] text-amber-900 dark:border-amber-400/25 dark:bg-amber-950/25 dark:text-amber-200"
		>
			<div class="font-medium">
				Showing {sf2MonthLabel(sf2State.launch.month)} — today's month is
				{sf2MonthLabel(sf2State.launch.todayMonth)} and it has no workbook yet.
			</div>
			<p class="mt-1 text-xs leading-5">
				Create {sf2MonthLabel(sf2State.launch.todayMonth)} from Reports to switch to it.
			</p>
		</div>
	{/if}

	<SettingsGroup>
		<SettingRow
			label="Re-run the workbook split"
			description="Safe to run again. A month already split is left alone."
		>
			<button
				type="button"
				onclick={() => sf2State.onRunSplit()}
				disabled={sf2State.splitRunning}
				class="btn btn-secondary btn-sm"
			>
				{#if sf2State.splitRunning}
					<Spinner />
				{/if}
				{sf2State.splitRunning ? 'Splitting…' : 'Run split'}
			</button>
		</SettingRow>

		<SettingRow
			label="Back up workbooks"
			description="Lives in Data & Backup, beside Back Up Data."
		>
			<button type="button" onclick={onOpenData} class="btn btn-secondary btn-sm"> Open </button>
		</SettingRow>
	</SettingsGroup>

	{#if sf2State.splitSummary}
		<p class="px-1 text-xs leading-5 text-muted-foreground">
			{sf2State.splitSummary}
			{#if sf2State.splitNeedsAttention.length > 0}
				Needs attention: {sf2State.splitNeedsAttention.join(', ')}.
			{/if}
		</p>
	{/if}
</SettingsSection>
