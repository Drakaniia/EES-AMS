<script lang="ts">
	import { Search, CheckCheck, Grid2X2, List, Check, X } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import type { Student, AttendanceType } from '$lib/db-rust';
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
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-border bg-card">
	<div class="shrink-0 border-b border-border p-5">
		<div class="flex flex-wrap items-start justify-between gap-4">
			<div>
				<h3 class="text-xl font-semibold">Student boxes</h3>
				<p class="mt-1 max-w-xl text-sm text-muted-foreground">
					Present by default. Like SF2, every learner starts as present. Click
					<span class="font-semibold text-foreground">Present all</span> to record the class, then click
					individual boxes to mark those learners absent.
				</p>
				<div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
					<span class="inline-flex items-center gap-1.5">
						<span
							class="size-2.5 rounded-sm border border-green-500/35 bg-green-50"
							aria-hidden="true"
						></span>
						Present
					</span>
					<span class="inline-flex items-center gap-1.5">
						<span class="size-2.5 rounded-sm border border-red-500/35 bg-red-50" aria-hidden="true"
						></span>
						Absent
					</span>
					<span class="inline-flex items-center gap-1.5">
						<span class="size-2.5 rounded-sm border border-border bg-background" aria-hidden="true"
						></span>
						Pending · Present by default
					</span>
				</div>
			</div>
			<div class="grid grid-cols-4 overflow-hidden rounded-xl border border-border bg-surface">
				{@render manualStat('Names', manualStudents.length)}
				{@render manualStat('Present', presentCount)}
				{@render manualStat('Pending', pendingCount)}
				{@render manualStat('Absent', absentCount)}
			</div>
		</div>
	</div>

	<div class="flex shrink-0 flex-wrap items-center gap-3 border-b border-border p-4">
		{#if dateNav}
			{@render dateNav()}
		{/if}
		<div class="relative min-w-64 flex-1">
			<Search
				class="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
			/>
			<label for="name-search" class="sr-only">Find name</label>
			<input
				id="name-search"
				value={rosterQuery}
				oninput={(e) => onRosterQueryChange((e.currentTarget as HTMLInputElement).value)}
				placeholder="Search by name..."
				class="h-10 w-full rounded-md border border-border bg-background pr-4 pl-10 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			/>
		</div>

		<button
			type="button"
			disabled={isProcessing || dateLoading || !isScheduledDayValue || rosterCount === 0}
			onclick={onPresentAllStudents}
			title={isScheduledDayValue
				? 'Marks every student in this class as present, regardless of the search filter'
				: 'Attendance can only be recorded on scheduled class days'}
			class="inline-flex h-10 shrink-0 items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
		>
			{#if isPresentingAll}
				<span class="size-2 rounded-full bg-primary-foreground" aria-hidden="true"></span>
			{:else}
				<CheckCheck class="size-4" aria-hidden="true" />
			{/if}
			{isPresentingAll ? 'Recording...' : 'Present all'}
		</button>

		{#if presentCount > 0 || absentCount > 0}
			<button
				type="button"
				disabled={isProcessing || dateLoading || !isScheduledDayValue}
				onclick={onClearAllAttendance}
				title="Remove all recorded attendance and reset absent marks for this session"
				class="inline-flex h-10 shrink-0 items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium text-muted-foreground transition-colors hover:border-destructive/50 hover:text-destructive disabled:cursor-not-allowed disabled:opacity-40"
			>
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
					<path d="M3 6h18" />
					<path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
					<path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
				</svg>
				{presentCount > 0 ? `Clear all (${presentCount})` : 'Clear all'}
			</button>
		{/if}

		<div
			class="flex shrink-0 overflow-hidden rounded-pill border border-border bg-surface p-1"
			role="group"
			aria-label="Attendance roster view"
		>
			<button
				type="button"
				aria-pressed={manualViewMode === 'boxes'}
				onclick={() => (manualViewMode = 'boxes')}
				class="inline-flex h-9 items-center gap-2 rounded-pill px-3 text-sm font-medium transition-colors {manualViewMode ===
				'boxes'
					? 'bg-background text-foreground shadow-sm'
					: 'text-muted-foreground hover:text-foreground'}"
			>
				<Grid2X2 class="size-4" />
				Boxes
			</button>
			<button
				type="button"
				aria-pressed={manualViewMode === 'list'}
				onclick={() => (manualViewMode = 'list')}
				class="inline-flex h-9 items-center gap-2 rounded-pill px-3 text-sm font-medium transition-colors {manualViewMode ===
				'list'
					? 'bg-background text-foreground shadow-sm'
					: 'text-muted-foreground hover:text-foreground'}"
			>
				<List class="size-4" />
				List
			</button>
		</div>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto p-4">
		{#if !isScheduledDayValue}
			<div
				class="flex h-full min-h-72 flex-col items-center justify-center rounded-xl border border-dashed border-border p-8 text-center"
			>
				<div
					class="mb-4 grid size-12 place-items-center rounded-full border border-border bg-surface text-muted-foreground"
				>
					<svg
						class="size-6"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
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
				<p class="font-medium text-foreground">Not a scheduled class day</p>
				<p class="mt-1 text-sm text-muted-foreground">
					Attendance can only be recorded on class days configured in Settings. View or edit the
					schedule for this class on the Settings page.
				</p>
			</div>
		{:else if manualStudents.length === 0}
			<div
				class="flex h-full min-h-72 items-center justify-center rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground"
			>
				No names match this class or search.
			</div>
		{:else if manualViewMode === 'boxes'}
			<div
				class="grid h-full auto-rows-[116px] grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-3 pr-1"
			>
				{#each manualStudents as student (student.id)}
					{@const action = onGetNextAttendanceType(student)}
					{@const status = onGetStudentStatus(student)}
					<button
						type="button"
						title={`${student.name} - ${status.label} · Right-click to mark absent`}
						disabled={isProcessing || dateLoading}
						onclick={() => onMarkStudent(student, action)}
						oncontextmenu={(e) => {
							e.preventDefault();
							onMarkAbsent(student);
						}}
						class="group flex h-[116px] min-w-0 flex-col justify-between overflow-hidden rounded-xl border p-3 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-50 {status.tone ===
						'present'
							? 'border-green-500/35 bg-green-50 hover:bg-green-100'
							: status.tone === 'absent'
								? 'border-red-500/35 bg-red-50 hover:bg-red-100'
								: 'border-border bg-background hover:border-primary hover:bg-primary/10'}"
					>
						<span class="flex min-w-0 items-start gap-2">
							<span
								class="grid size-9 shrink-0 place-items-center rounded-lg border text-[11px] font-bold {status.tone ===
								'present'
									? 'border-green-500/35 bg-green-100 text-green-700'
									: status.tone === 'absent'
										? 'border-red-500/35 bg-red-100 text-red-700'
										: 'border-border bg-surface text-foreground'}"
							>
								{getStudentInitials(student.name)}
							</span>
							<span class="min-w-0 flex-1">
								<span
									class="student-card-name text-sm leading-snug font-semibold break-words whitespace-normal"
								>
									{student.name}
								</span>
							</span>
						</span>
						<span class="flex items-center justify-between gap-2">
							<span
								class="min-w-0 truncate text-[10px] leading-snug {status.tone === 'present'
									? 'text-green-700'
									: status.tone === 'absent'
										? 'text-red-700'
										: 'text-muted-foreground'}"
							>
								{status.label}
							</span>
							{#if status.tone === 'present'}
								<Check class="size-3.5 shrink-0 text-green-700" aria-hidden="true" />
							{:else if status.tone === 'absent'}
								<X class="size-3.5 shrink-0 text-red-700" aria-hidden="true" />
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
			<div class="h-full overflow-y-auto rounded-xl border border-border">
				<ul class="divide-y divide-border">
					{#each manualStudents as student (student.id)}
						{@const action = onGetNextAttendanceType(student)}
						{@const status = onGetStudentStatus(student)}
						<li
							class="flex flex-col gap-3 px-4 py-3 hover:bg-surface/50 sm:flex-row sm:items-center sm:justify-between"
							oncontextmenu={(e) => {
								e.preventDefault();
								onMarkAbsent(student);
							}}
						>
							<div class="flex min-w-0 items-center gap-3">
								<div
									class="grid size-10 shrink-0 place-items-center rounded-lg border text-xs font-bold {status.tone ===
									'present'
										? 'border-green-500/35 bg-green-100 text-green-700'
										: status.tone === 'absent'
											? 'border-red-500/35 bg-red-100 text-red-700'
											: 'border-border bg-surface text-foreground'}"
								>
									{getStudentInitials(student.name)}
								</div>
								<div class="min-w-0 flex-1">
									<div class="text-base leading-snug font-semibold break-words">
										{student.name}
									</div>
									<div
										class="mt-1 flex items-center gap-1.5 text-xs {status.tone === 'present'
											? 'text-green-700'
											: status.tone === 'absent'
												? 'text-red-700'
												: 'text-muted-foreground'}"
									>
										{#if status.tone === 'present'}
											<Check class="size-3.5" aria-hidden="true" />
										{:else if status.tone === 'absent'}
											<X class="size-3.5" aria-hidden="true" />
										{/if}
										{status.label}
									</div>
								</div>
							</div>
							<button
								disabled={isProcessing || dateLoading || !isScheduledDayValue}
								onclick={() => onMarkStudent(student, action)}
								class="w-fit min-w-28 rounded-pill px-4 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 {status.tone ===
								'present'
									? 'border border-border bg-surface text-muted-foreground'
									: 'bg-primary text-primary-foreground hover:bg-accent'}"
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

{#snippet manualStat(label: string, value: number)}
	<div class="min-w-20 px-4 py-3 text-center">
		<div class="label-mono text-[10px]">{label}</div>
		<div class="mt-1 text-2xl font-semibold">{value}</div>
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
