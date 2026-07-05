<script lang="ts">
	import {
		AlertTriangle,
		Check,
		X
	} from 'lucide-svelte';
	import type { Sf2PreviewStudentRow, Sf2PreviewCell } from '$lib/db-rust';
	import type { MatrixWeekGroup, MatrixStudentRow } from './report-state.svelte';
	import {
		formatDayNumber,
		matrixDateLabel,
		weekRangeLabel,
		cellForDate,
		cellLabel,
		unmappedCellLabel,
		cellClass,
		cellKey
	} from './report-state.svelte';

	let {
		previewTemplateGradeLevel,
		previewTemplateSection,
		matrixWeekGroups,
		matrixStudents,
		correctingCellKey,
		fullReview,
		onToggleAttendance,
	}: {
		previewTemplateGradeLevel: string;
		previewTemplateSection: string;
		matrixWeekGroups: MatrixWeekGroup[];
		matrixStudents: MatrixStudentRow[];
		correctingCellKey: string | null;
		fullReview: boolean;
		onToggleAttendance: (row: Sf2PreviewStudentRow, cell: Sf2PreviewCell) => void;
	} = $props();
</script>

<div
	class="border border-border bg-card shadow-sm {fullReview
		? 'flex min-h-0 flex-1 flex-col rounded-xl'
		: 'rounded-2xl'}"
>
	<div
		class="flex flex-wrap items-start justify-between gap-3 border-b border-border px-5 py-4"
	>
		<div>
			<div class="label-mono text-primary">SF2 attendance grid</div>
			<h2 class="mt-1 text-xl font-semibold">
				{previewTemplateGradeLevel} - {previewTemplateSection}
			</h2>
			<p class="mt-1 text-sm text-muted-foreground">
				Click a cell to toggle the learner between present and absent.
			</p>
		</div>
		<div class="flex flex-wrap gap-2 text-xs">
			<span
				class="rounded-pill border border-emerald-500/30 bg-emerald-50 px-2.5 py-1 text-emerald-700"
			>
				Present
			</span>
			<span class="rounded-pill border border-red-500/35 bg-red-50 px-2.5 py-1 text-red-700">
				Absent
			</span>
			<span
				class="rounded-pill border border-border bg-background px-2.5 py-1 text-muted-foreground"
			>
				Open day
			</span>
		</div>
	</div>

	<div class={fullReview ? 'min-h-0 flex-1 overflow-auto' : 'max-h-[560px] overflow-auto'}>
		<table class="min-w-full border-separate border-spacing-0 text-sm">
			<thead>
				<tr>
					<th
						rowspan="2"
						class="sticky top-0 left-0 z-30 w-72 min-w-72 border-r border-b border-border bg-card px-4 py-3 text-left align-middle"
					>
						Learner
					</th>
					{#each matrixWeekGroups as week (week.key)}
						<th
							colspan={week.slots.length}
							class="sticky top-0 z-20 border-b border-l-2 border-border border-l-primary/45 bg-orange-50 px-2 py-2 text-center"
							title={weekRangeLabel(week)}
						>
							<div class="label-mono text-primary">{week.label}</div>
							<div class="mt-0.5 font-mono text-[10px] font-medium text-muted-foreground">
								{weekRangeLabel(week)}
							</div>
						</th>
					{/each}
				</tr>
				<tr>
					{#each matrixWeekGroups as week (week.key)}
						{#each week.slots as slot, dateIndex (slot.key)}
							<th
								class="sticky top-[43px] z-10 min-w-14 border-b border-border bg-card px-2 py-2 text-center {dateIndex ===
									0
									? 'border-l-2 border-l-primary/45'
									: 'border-l border-l-border/60'}"
								title={slot.date
									? `${matrixDateLabel(slot.date.date)} ${slot.date.columnLetter}${slot.date.columnIndex}`
									: slot.dateKey
										? `${matrixDateLabel(slot.dateKey)}, no SF2 column mapped`
										: `${slot.weekday}, no class day in this month`}
							>
								<div class="font-mono text-sm leading-none font-bold">
									{slot.dateKey ? formatDayNumber(slot.dateKey) : ''}
								</div>
								<div class="mt-1 font-mono text-[10px] font-semibold text-muted-foreground">
									{slot.weekday}
								</div>
							</th>
						{/each}
					{/each}
				</tr>
			</thead>
			<tbody>
				{#each matrixStudents as row (row.studentId)}
					<tr class={row.mapped ? 'bg-background' : 'bg-amber-50/60'}>
						<th
							class="sticky left-0 z-10 w-72 min-w-72 border-r border-b border-border bg-inherit px-4 py-2 text-left align-middle"
						>
							<div class="flex items-center gap-2">
								<div class="min-w-0 flex-1">
									<div class="truncate font-medium">{row.studentName}</div>
									<div
										class="mt-0.5 flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground"
									>
										<span>{row.gender ?? 'No gender'}</span>
										<span aria-hidden="true">/</span>
										<span>{row.mapped ? `Row ${row.rowIndex}` : 'Unmapped'}</span>
									</div>
								</div>
								{#if row.warnings.length > 0}
									<AlertTriangle class="size-4 shrink-0 text-amber-600" aria-hidden="true" />
								{/if}
							</div>
						</th>
						{#each matrixWeekGroups as week (week.key)}
							{#each week.slots as slot, dateIndex (slot.key)}
								{@const cell = slot.dateKey ? cellForDate(row, slot.dateKey) : null}
								<td
									class="border-b border-border/80 px-1.5 py-1.5 text-center {dateIndex === 0
										? 'border-l-2 border-l-primary/30 bg-primary/5'
										: 'border-l border-l-border/40'}"
								>
									{#if cell}
										<button
											type="button"
											disabled={!cell.editable || !row.mapped || correctingCellKey !== null}
											onclick={() => onToggleAttendance(row, cell)}
											aria-label={cellLabel(row, cell)}
											title={cellLabel(row, cell)}
											class="control-ring inline-grid size-9 place-items-center rounded-md border text-xs font-bold transition-colors disabled:cursor-not-allowed disabled:opacity-70 {cellClass(
												row,
												cell
											)}"
										>
											{#if correctingCellKey === cellKey(row.studentId, cell.date)}
												<span class="font-mono text-[10px]">...</span>
											{:else if cell.status === 'present'}
												<Check class="size-4" aria-hidden="true" />
											{:else if cell.status === 'absent'}
												<X class="size-4" aria-hidden="true" />
											{:else}
												<span aria-hidden="true">-</span>
											{/if}
										</button>
									{:else if slot.dateKey}
										<span
											role="img"
											aria-label={unmappedCellLabel(row, slot.dateKey)}
											title={unmappedCellLabel(row, slot.dateKey)}
											class="inline-grid size-9 place-items-center rounded-md border border-dashed border-border bg-background text-xs font-bold text-muted-foreground"
										>
											-
										</span>
									{:else}
										<span
											aria-hidden="true"
											class="inline-grid size-9 place-items-center text-muted-foreground"
										>
											&nbsp;
										</span>
									{/if}
								</td>
							{/each}
						{/each}
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
