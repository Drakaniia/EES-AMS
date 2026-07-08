<script lang="ts">
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import {
		SF2_CALENDAR_WEEKDAYS,
		SF2_SCHOOL_MONTHS,
		isSf2SchoolDay,
		sf2CalendarCells,
		sf2SelectedFirstAttendanceLabel
	} from '$lib/features/settings/sf2-workbook';

	let {
		open = $bindable(false),
		mode = $bindable<'create' | 'edit'>('create'),
		notice = $bindable<string | null>(null),
		creating = $bindable(false),
		saving = $bindable(false),
		classId = $bindable(''),
		schoolId = $bindable(''),
		schoolName = $bindable(''),
		schoolYear = $bindable(''),
		reportMonth = $bindable(''),
		gradeLevel = $bindable(''),
		section = $bindable(''),
		adviserName = $bindable(''),
		schoolHeadName = $bindable(''),
		firstSchoolDay = $bindable(1),
		onselectReportMonth,
		onupdateSchoolYear,
		onselectFirstSchoolDay,
		onsubmit,
		onclose
	}: {
		open?: boolean;
		mode?: 'create' | 'edit';
		notice?: string | null;
		creating?: boolean;
		saving?: boolean;
		classId?: string;
		schoolId?: string;
		schoolName?: string;
		schoolYear?: string;
		reportMonth?: string;
		gradeLevel?: string;
		section?: string;
		adviserName?: string;
		schoolHeadName?: string;
		firstSchoolDay?: number;
		onselectReportMonth?: (monthValue: string) => void;
		onupdateSchoolYear?: (value: string) => void;
		onselectFirstSchoolDay?: (day: number | null) => void;
		onsubmit?: (e: SubmitEvent) => void;
		onclose?: (force?: boolean) => void;
	} = $props();

	const firstAttendanceCalendar = $derived(
		sf2CalendarCells(reportMonth, schoolYear, firstSchoolDay)
	);
	const firstAttendanceLabel = $derived(
		sf2SelectedFirstAttendanceLabel({
			reportMonth,
			schoolYear,
			firstSchoolDay
		})
	);
</script>

<Dialog
	{open}
	title={notice
		? 'Update SF2 Settings'
		: mode === 'create'
			? 'Create SF2 Workbook'
			: 'Update SF2 Settings'}
	description={notice
		? 'Review the imported workbook details before using this SF2 copy.'
		: mode === 'create'
			? 'Enter the form details for this workbook copy.'
			: 'Update the saved workbook copy and attendance date layout.'}
	maxWidth="xl"
	showCloseButton={!(creating || saving)}
	onClose={() => onclose?.()}
>
	<form {onsubmit} class="space-y-5">
		{#if notice}
			<div class="rounded-md border border-primary/30 bg-primary/10 p-4 text-sm text-foreground">
				<div class="label-mono text-primary">Month Review</div>
				<p class="mt-2 leading-6">{notice}</p>
			</div>
		{/if}

		<div class="grid gap-4 sm:grid-cols-2">
			<div class="space-y-1.5">
				<label for="sf2SchoolYear2" class="label-mono">School Year</label>
				<input
					id="sf2SchoolYear2"
					value={schoolYear}
					oninput={(event) => onupdateSchoolYear?.((event.currentTarget as HTMLInputElement).value)}
					required
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
		</div>

		<div class="grid gap-4 sm:grid-cols-2">
			<div class="space-y-1.5">
				<label for="sf2SchoolId2" class="label-mono">School ID</label>
				<input
					id="sf2SchoolId2"
					bind:value={schoolId}
					required
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
			<div class="space-y-1.5">
				<label for="sf2SchoolName2" class="label-mono">Name of School</label>
				<input
					id="sf2SchoolName2"
					bind:value={schoolName}
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
						aria-pressed={reportMonth === month.value}
						onclick={() => onselectReportMonth?.(month.value)}
						class={`h-10 rounded-md border px-3 text-sm font-medium transition-colors ${
							reportMonth === month.value
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
					<span class="text-xs font-medium text-muted-foreground">{firstAttendanceLabel}</span>
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
						{#each firstAttendanceCalendar as cell (cell.key)}
							{#if cell.day === null}
								<div class="h-9 rounded-md"></div>
							{:else}
								<button
									type="button"
									disabled={!cell.isSchoolDay}
									aria-pressed={cell.isSelected}
									onclick={() => onselectFirstSchoolDay?.(cell.day)}
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
					<label for="sf2GradeLevel2" class="label-mono">Grade Level</label>
					<input
						id="sf2GradeLevel2"
						bind:value={gradeLevel}
						required
						class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
				<div class="space-y-1.5">
					<label for="sf2Section2" class="label-mono">Section</label>
					<input
						id="sf2Section2"
						bind:value={section}
						required
						class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
			</div>
		</div>

		<div class="grid gap-4 sm:grid-cols-2">
			<div class="space-y-1.5">
				<label for="sf2AdviserName2" class="label-mono">Adviser / LIS Name</label>
				<input
					id="sf2AdviserName2"
					bind:value={adviserName}
					required
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
			<div class="space-y-1.5">
				<label for="sf2SchoolHeadName2" class="label-mono">School Head Name</label>
				<input
					id="sf2SchoolHeadName2"
					bind:value={schoolHeadName}
					required
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
		</div>

		<div class="flex justify-end gap-2 pt-2">
			<button
				type="button"
				onclick={() => onclose?.()}
				disabled={creating || saving}
				class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				Cancel
			</button>
			<button
				type="submit"
				disabled={creating || saving}
				class="inline-flex items-center justify-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if creating}
					<Spinner />
					Creating...
				{:else if saving}
					<Spinner />
					Saving...
				{:else}
					{mode === 'create' ? 'Create Workbook' : 'Save Workbook Settings'}
				{/if}
			</button>
		</div>
	</form>
</Dialog>
