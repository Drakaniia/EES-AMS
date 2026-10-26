<script lang="ts">
	import { Search, CheckCheck, Grid2X2, List, Check, X, Trash2 } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import type { Student, AttendanceType } from '$lib/api';
	import type { ManualViewMode } from './attendance-state.svelte';
	import { getStudentInitials } from './attendance-state.svelte';

	let {
		manualStudents,
		manualViewMode = $bindable(),
		isProcessing,
		dateLoading,
		presentCount,
		absentCount,
		pendingCount,
		rosterCount,
		rosterQuery,
		isScheduledDayValue,
		isPresentingAll,
		onMarkStudent,
		onPresentAllStudents,
		onClearAllAttendance,
		onRosterQueryChange,
		onGetNextAttendanceType,
		onGetStudentStatus,
		onMarkAbsent,
		dateNav
	}: {
		manualStudents: Student[];
		manualViewMode: ManualViewMode;
		isProcessing: boolean;
		dateLoading: boolean;
		presentCount: number;
		absentCount: number;
		pendingCount: number;
		rosterCount: number;
		rosterQuery: string;
		isScheduledDayValue: boolean;
		isPresentingAll: boolean;
		onMarkStudent: (student: Student, action: AttendanceType | null) => void;
		onPresentAllStudents: () => void;
		onClearAllAttendance: () => void;
		onRosterQueryChange: (value: string) => void;
		onGetNextAttendanceType: (student: Student) => AttendanceType | null;
		onGetStudentStatus: (student: Student) => { label: string; tone: string };
		onMarkAbsent: (student: Student) => void;
		dateNav?: Snippet;
	} = $props();

	/** The three states are the only colour on this screen, and each one is real. */
	function cardTone(tone: string): string {
		if (tone === 'present') {
			return 'border-emerald-600/20 bg-emerald-50/70 hover:bg-emerald-50 dark:border-emerald-400/20 dark:bg-emerald-950/20 dark:hover:bg-emerald-950/35';
		}
		if (tone === 'absent') {
			return 'border-red-600/25 bg-red-50/75 hover:bg-red-50 dark:border-red-400/25 dark:bg-red-950/25 dark:hover:bg-red-950/40';
		}
		return 'border-border bg-card hover:bg-surface';
	}

	function initialsTone(tone: string): string {
		if (tone === 'present') {
			return 'bg-card text-emerald-800 dark:text-emerald-300';
		}
		if (tone === 'absent') {
			return 'bg-card text-red-800 dark:text-red-300';
		}
		return 'bg-surface text-muted-foreground';
	}

	function statusTone(tone: string): string {
		if (tone === 'present') return 'text-emerald-800 dark:text-emerald-300';
		if (tone === 'absent') return 'text-red-800 dark:text-red-300';
		return 'text-muted-foreground';
	}
</script>

<div class="surface-panel flex min-h-0 flex-1 flex-col overflow-hidden">
	<!-- The header states the job, the rule, and the live split. -->
	<div class="shrink-0 border-b border-border px-5 py-4">
		<div class="flex flex-wrap items-start justify-between gap-x-6 gap-y-3">
			<div class="min-w-0">
				<h2 class="text-[17px] leading-tight font-semibold tracking-[-0.018em]">Student boxes</h2>
				<p class="mt-1 max-w-xl text-[12.5px] leading-5 text-muted-foreground">
					Every learner starts present. Record the class with <span
						class="font-medium text-foreground">Present all</span
					>, then mark only the exceptions.
				</p>
				<div
					class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-[11px] text-muted-foreground"
				>
					<span class="inline-flex items-center gap-1.5">
						<span class="size-2 rounded-sm bg-emerald-500/70" aria-hidden="true"></span>
						Present
					</span>
					<span class="inline-flex items-center gap-1.5">
						<span class="size-2 rounded-sm bg-red-500/70" aria-hidden="true"></span>
						Absent
					</span>
					<span class="inline-flex items-center gap-1.5">
						<span class="size-2 rounded-sm border border-border bg-card" aria-hidden="true"></span>
						Pending
					</span>
				</div>
			</div>

			<dl class="flex items-end gap-5">
				{@render stat('Names', manualStudents.length)}
				{@render stat('Present', presentCount, 'text-emerald-700 dark:text-emerald-400')}
				{@render stat('Pending', pendingCount)}
				{@render stat('Absent', absentCount, 'text-red-700 dark:text-red-400')}
			</dl>
		</div>
	</div>

	<!-- One toolbar. Nothing here competes with the roster itself. -->
	<div class="flex shrink-0 flex-wrap items-center gap-2 border-b border-border px-4 py-2.5">
		{#if dateNav}
			{@render dateNav()}
		{/if}

		<div class="search-field min-w-52 flex-1">
			<Search aria-hidden="true" />
			<label for="name-search" class="sr-only">Find name</label>
			<input
				id="name-search"
				value={rosterQuery}
				oninput={(e) => onRosterQueryChange((e.currentTarget as HTMLInputElement).value)}
				placeholder="Search by name"
				class="field pl-[31px]"
			/>
		</div>

		<button
			type="button"
			disabled={isProcessing || dateLoading || !isScheduledDayValue || rosterCount === 0}
			onclick={onPresentAllStudents}
			title={isScheduledDayValue
				? 'Marks every student in this class as present, regardless of the search filter'
				: 'Attendance can only be recorded on scheduled class days'}
			class="btn btn-primary h-8 shrink-0"
		>
			{#if isPresentingAll}
				<span class="size-2 rounded-full bg-primary-foreground" aria-hidden="true"></span>
			{:else}
				<CheckCheck class="size-4" strokeWidth={2} aria-hidden="true" />
			{/if}
			{isPresentingAll ? 'Recording…' : 'Present all'}
		</button>

		{#if presentCount > 0 || absentCount > 0}
			<button
				type="button"
				disabled={isProcessing || dateLoading || !isScheduledDayValue}
				onclick={onClearAllAttendance}
				title="Remove all recorded attendance and reset absent marks for this session"
				class="btn btn-ghost h-8 shrink-0 hover:text-destructive"
			>
				<Trash2 class="size-3.5" aria-hidden="true" />
				Clear{presentCount > 0 ? ` (${presentCount})` : ''}
			</button>
		{/if}

		<div class="segmented ml-auto" role="group" aria-label="Attendance roster view">
			<button
				type="button"
				aria-pressed={manualViewMode === 'boxes'}
				onclick={() => (manualViewMode = 'boxes')}
				class="segmented-item"
			>
				<Grid2X2 class="size-3.5" aria-hidden="true" />
				Boxes
			</button>
			<button
				type="button"
				aria-pressed={manualViewMode === 'list'}
				onclick={() => (manualViewMode = 'list')}
				class="segmented-item"
			>
				<List class="size-3.5" aria-hidden="true" />
				List
			</button>
		</div>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto p-4">
		{#if !isScheduledDayValue}
			<div class="flex h-full min-h-72 flex-col items-center justify-center px-6 text-center">
				<div class="grid size-10 place-items-center rounded-full bg-surface text-muted-foreground">
					<svg
						class="size-[18px]"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.8"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
						<line x1="16" y1="2" x2="16" y2="6" />
						<line x1="8" y1="2" x2="8" y2="6" />
						<line x1="3" y1="10" x2="21" y2="10" />
					</svg>
				</div>
				<p class="mt-3 text-[13px] font-medium">Not a scheduled class day</p>
				<p class="mt-1 max-w-md text-[12.5px] leading-5 text-muted-foreground">
					Attendance can only be recorded on class days. Set the schedule in Settings → Class before
					recording this date.
				</p>
			</div>
		{:else if manualStudents.length === 0}
			<div class="flex h-full min-h-72 flex-col items-center justify-center px-6 text-center">
				<p class="text-[13px] font-medium">
					{rosterQuery ? 'No names match this search.' : 'This class has no students yet.'}
				</p>
				<p class="mt-1 text-[12.5px] text-muted-foreground">
					{rosterQuery
						? 'Clear the search to see the full roster.'
						: 'Add students from the Class List page.'}
				</p>
			</div>
		{:else if manualViewMode === 'boxes'}
			<div
				class="grid h-full auto-rows-[112px] grid-cols-[repeat(auto-fill,minmax(164px,1fr))] gap-2.5 pr-0.5"
			>
				{#each manualStudents as student (student.id)}
					{@const action = onGetNextAttendanceType(student)}
					{@const status = onGetStudentStatus(student)}
					<button
						type="button"
						title={`${student.name} — ${status.label} · right-click to mark absent`}
						aria-label={`${student.name}, ${status.label}`}
						disabled={isProcessing || dateLoading}
						onclick={() => onMarkStudent(student, action)}
						oncontextmenu={(e) => {
							e.preventDefault();
							onMarkAbsent(student);
						}}
						class="group flex h-[112px] min-w-0 flex-col justify-between overflow-hidden rounded-xl border p-3 text-left transition-[background-color,border-color,transform] duration-150 active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50 {cardTone(
							status.tone
						)}"
					>
						<span class="flex min-w-0 items-start gap-2">
							<span
								class="grid size-8 shrink-0 place-items-center rounded-lg text-[10.5px] font-semibold {initialsTone(
									status.tone
								)}"
							>
								{getStudentInitials(student.name)}
							</span>
							<span class="min-w-0 flex-1">
								<span
									class="student-card-name text-[13px] leading-snug font-medium break-words whitespace-normal"
								>
									{student.name}
								</span>
							</span>
						</span>
						<span class="flex items-center justify-between gap-2">
							<span class="min-w-0 truncate text-[10.5px] font-medium {statusTone(status.tone)}">
								{status.label}
							</span>
							{#if status.tone === 'present'}
								<Check
									class="size-3.5 shrink-0 text-emerald-700 dark:text-emerald-400"
									strokeWidth={2.4}
									aria-hidden="true"
								/>
							{:else if status.tone === 'absent'}
								<X
									class="size-3.5 shrink-0 text-red-700 dark:text-red-400"
									strokeWidth={2.4}
									aria-hidden="true"
								/>
							{:else}
								<span
									class="size-1.5 shrink-0 rounded-full bg-muted-foreground/40"
									aria-hidden="true"
								></span>
							{/if}
						</span>
					</button>
				{/each}
			</div>
		{:else}
			<div class="h-full overflow-y-auto rounded-xl border border-border bg-card">
				<ul>
					{#each manualStudents as student, index (student.id)}
						{@const action = onGetNextAttendanceType(student)}
						{@const status = onGetStudentStatus(student)}
						<li
							class="flex flex-col gap-3 px-4 py-3 hover:bg-surface/70 sm:flex-row sm:items-center sm:justify-between {index >
							0
								? 'border-t border-border'
								: ''}"
							oncontextmenu={(e) => {
								e.preventDefault();
								onMarkAbsent(student);
							}}
						>
							<div class="flex min-w-0 items-center gap-3">
								<div
									class="grid size-9 shrink-0 place-items-center rounded-lg text-[11px] font-semibold {initialsTone(
										status.tone
									)}"
								>
									{getStudentInitials(student.name)}
								</div>
								<div class="min-w-0 flex-1">
									<div class="text-[13px] leading-snug font-medium break-words">{student.name}</div>
									<div
										class="mt-0.5 flex items-center gap-1.5 text-[11.5px] {statusTone(status.tone)}"
									>
										{#if status.tone === 'present'}
											<Check class="size-3" strokeWidth={2.4} aria-hidden="true" />
										{:else if status.tone === 'absent'}
											<X class="size-3" strokeWidth={2.4} aria-hidden="true" />
										{/if}
										{status.label}
									</div>
								</div>
							</div>
							<button
								disabled={isProcessing || dateLoading || !isScheduledDayValue}
								onclick={() => onMarkStudent(student, action)}
								class="w-fit min-w-24 shrink-0 {status.tone === 'present'
									? 'btn btn-secondary btn-sm'
									: 'btn btn-primary btn-sm'} disabled:cursor-not-allowed disabled:opacity-50"
							>
								{status.tone === 'present' ? 'Recorded' : 'Record'}
							</button>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>
</div>

{#snippet stat(label: string, value: number, tone = 'text-foreground')}
	<div class="text-right">
		<dt class="text-[10.5px] font-medium text-muted-foreground">{label}</dt>
		<dd class="mt-0.5 text-[18px] leading-none font-semibold tabular-nums {tone}">{value}</dd>
	</div>
{/snippet}

<style>
	.student-card-name {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
	}
</style>
