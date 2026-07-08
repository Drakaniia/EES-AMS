<script lang="ts">
	import { Search, CheckCheck, Grid2X2, List } from 'lucide-svelte';
	import { fmtTime } from '$lib/csv';
	import type { Student, AttendanceType } from '$lib/db-rust';
	import type { ManualViewMode } from './attendance-state.svelte';
	import { getStudentInitials, getStudentClassName, studentName } from './attendance-state.svelte';

	let {
		manualStudents,
		manualViewMode = $bindable(),
		isProcessing,
		dateLoading,
		selectedClassId,
		selectedDateLabel,
		selectedDate,
		recentActivity,
		studentById,
		classById,
		recordedCount,
		pendingCount,
		pendingManualStudents,
		rosterQuery,
		isScheduledDayValue,
		isPresentingAll,
		onMarkStudent,
		onPresentAllStudents,
		onClearAllAttendance,
		onRosterQueryChange,
		onGetNextAttendanceType,
		onGetStudentStatus
	}: {
		manualStudents: Student[];
		manualViewMode: ManualViewMode;
		isProcessing: boolean;
		dateLoading: boolean;
		selectedClassId: string;
		selectedDateLabel: string;
		selectedDate: string;
		recentActivity: import('$lib/db-rust').AttendanceEvent[];
		studentById: Map<string, Student>;
		classById: Map<string, import('$lib/db-rust').Class>;
		recordedCount: number;
		pendingCount: number;
		pendingManualStudents: Student[];
		rosterQuery: string;
		isScheduledDayValue: boolean;
		isPresentingAll: boolean;
		onMarkStudent: (student: Student, action: AttendanceType | null) => void;
		onPresentAllStudents: () => void;
		onClearAllAttendance: () => void;
		onRosterQueryChange: (value: string) => void;
		onGetNextAttendanceType: (student: Student) => AttendanceType | null;
		onGetStudentStatus: (student: Student) => { label: string; tone: string };
	} = $props();
</script>

<div
	class="flex min-h-0 flex-1 flex-col gap-5 xl:grid xl:grid-cols-[minmax(0,1fr)_340px] 2xl:grid-cols-[minmax(0,1fr)_400px]"
>
	<div class="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-border bg-card">
		<div class="shrink-0 border-b border-border p-5">
			<div class="flex flex-wrap items-start justify-between gap-4">
				<div>
					<h3 class="text-xl font-semibold">Student boxes</h3>
					<p class="mt-1 max-w-xl text-sm text-muted-foreground">
						One click per learner. Boxes show whether attendance has been recorded for
						{selectedDateLabel}.
					</p>
				</div>
				<div class="grid grid-cols-3 overflow-hidden rounded-xl border border-border bg-surface">
					{@render manualStat('Names', manualStudents.length)}
					{@render manualStat('Recorded', recordedCount)}
					{@render manualStat('Pending', pendingCount)}
				</div>
			</div>
		</div>

		<div class="flex shrink-0 flex-wrap items-center gap-3 border-b border-border p-4">
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
				disabled={isProcessing ||
					dateLoading ||
					!isScheduledDayValue ||
					pendingManualStudents.length === 0 ||
					manualStudents.length === 0}
				onclick={onPresentAllStudents}
				title={isScheduledDayValue
					? rosterQuery.trim()
						? 'Marks every pending student currently shown by the search as present'
						: 'Marks every pending student in this roster as present'
					: 'Attendance can only be recorded on scheduled class days'}
				class="inline-flex h-10 shrink-0 items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if isPresentingAll}
					<span class="size-2 rounded-full bg-primary-foreground" aria-hidden="true"></span>
				{:else}
					<CheckCheck class="size-4" aria-hidden="true" />
				{/if}
				{isPresentingAll
					? 'Recording...'
					: pendingManualStudents.length > 0
						? `Present all (${pendingManualStudents.length})`
						: 'Present all'}
			</button>

			{#if recordedCount > 0}
				<button
					type="button"
					disabled={isProcessing || dateLoading || !isScheduledDayValue}
					onclick={onClearAllAttendance}
					title="Remove all recorded attendance for this session"
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
					Clear all ({recordedCount})
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
						schedule for this class on the Configuration page.
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
							title={`${student.name} - ${status.label}`}
							disabled={isProcessing || dateLoading}
							onclick={() => onMarkStudent(student, action)}
							class="group flex h-[116px] min-w-0 flex-col justify-between overflow-hidden rounded-xl border p-3 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-50 {action ===
							'in'
								? 'border-border bg-background hover:border-primary hover:bg-primary/10'
								: 'border-border bg-surface/80 text-muted-foreground'}"
						>
							<span class="flex min-w-0 items-start gap-2">
								<span
									class="grid size-9 shrink-0 place-items-center rounded-lg border text-[11px] font-bold {status.tone ===
									'in'
										? 'border-primary/30 bg-primary text-primary-foreground'
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
								<span class="min-w-0 truncate text-[10px] leading-snug text-muted-foreground">
									{selectedClassId ? status.label : getStudentClassName(student, classById)}
								</span>
								<span
									class="label-mono shrink-0 text-[10px] font-bold {action === 'in'
										? 'text-primary'
										: 'text-muted-foreground'}"
								>
									{action === 'in' ? 'IN' : 'RECORDED'}
								</span>
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
							>
								<div class="flex min-w-0 items-center gap-3">
									<div
										class="grid size-10 shrink-0 place-items-center rounded-lg border border-border bg-surface text-xs font-bold"
									>
										{getStudentInitials(student.name)}
									</div>
									<div class="min-w-0 flex-1">
										<div class="text-base leading-snug font-semibold break-words">
											{student.name}
										</div>
										<div
											class="mt-1 text-xs {status.tone === 'in'
												? 'text-primary'
												: 'text-muted-foreground'}"
										>
											{status.label}
										</div>
									</div>
								</div>
								<button
									disabled={isProcessing || dateLoading || !isScheduledDayValue}
									onclick={() => onMarkStudent(student, action)}
									class="w-fit min-w-28 rounded-pill px-4 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 {action ===
									'in'
										? 'bg-primary text-primary-foreground hover:bg-accent'
										: 'border border-border bg-surface text-muted-foreground'}"
								>
									{action === 'in' ? 'Record' : 'Recorded'}
								</button>
							</li>
						{/each}
					</ul>
				</div>
			{/if}
		</div>
	</div>

	<div class="flex min-h-0 flex-col rounded-2xl border border-border bg-card p-5">
		<div class="mb-4 flex shrink-0 items-start justify-between gap-3">
			<div>
				<h3 class="text-lg font-medium">Recent activity</h3>
				<span class="label-mono text-xs opacity-60">{selectedDate}</span>
			</div>
			<span class="label-mono rounded-pill border border-border bg-surface px-2 py-1 text-[10px]">
				{recentActivity.length} events
			</span>
		</div>

		<div class="min-h-0 flex-1 overflow-y-auto">
			{#if recentActivity.length === 0}
				<div
					class="flex h-full w-full flex-col items-center justify-center rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground"
				>
					No attendance has been recorded for {selectedDateLabel}.
				</div>
			{:else}
				<ul class="divide-y divide-border">
					{#each recentActivity as event (event.id)}
						<li class="flex items-center justify-between gap-3 py-3">
							<div class="min-w-0 flex-1">
								<div class="leading-snug font-medium break-words">
									{studentName(event.studentId, studentById)}
								</div>
								<div class="label-mono">{fmtTime(event.timestamp)}</div>
							</div>
							{@render pill(event.type)}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
</div>

{#snippet pill(type: AttendanceType | 'error')}
	<span
		class="shrink-0 rounded-pill px-2 py-1 font-mono text-[10px] font-bold
			{type === 'in'
			? 'bg-primary text-primary-foreground'
			: 'bg-destructive text-destructive-foreground'}"
	>
		{type === 'in' ? 'IN' : 'ERROR'}
	</span>
{/snippet}

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
