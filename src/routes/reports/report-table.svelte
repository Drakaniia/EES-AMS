<script lang="ts">
	import { AlertTriangle, X, CheckCheck, PanelRightClose, PanelRightOpen } from 'lucide-svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import type { Sf2PreviewStudentRow, Sf2PreviewCell } from '$lib/api';
	import type {
		MatrixWeekGroup,
		MatrixDateSlot,
		MatrixStudentRow,
		MatrixCell
	} from './report-state.svelte';
	import { weekRangeLabel } from './report-state.svelte';
	import ReportScrollbars from './report-scrollbars.svelte';

	let scrollEl: HTMLDivElement | undefined = $state(undefined);
	let tableEl: HTMLTableElement | undefined = $state(undefined);

	// NOTE: no custom wheel handler. A Svelte `onwheel` binding is
	// non-passive by default, so the browser must run JS before it may
	// start scrolling — every vertical wheel tick stalls on the main
	// thread. Native Shift+wheel already pans horizontally, so
	// intercepting it buys nothing and janks vertical scroll.
	// Both scrollbars are overlay thumbs (report-scrollbars.svelte) driven
	// by the scroller's own scroll events - no handlers live here.

	let {
		previewTemplateGradeLevel,
		previewTemplateSection,
		genderFilter,
		matrixWeekGroups,
		matrixDates,
		matrixStudents,
		correctingCellKey,
		sidebarCollapsed,
		presentingAll,
		hasAbsentCells,
		onToggleAttendance,
		onPresentAll,
		onToggleSidebar,
		onGenderFilterChange
	}: {
		previewTemplateGradeLevel: string;
		previewTemplateSection: string;
		genderFilter: 'all' | 'male' | 'female';
		matrixWeekGroups: MatrixWeekGroup[];
		/** Week groups flattened into render order — index-aligned with `row.cellColumns`. */
		matrixDates: MatrixDateSlot[];
		matrixStudents: MatrixStudentRow[];
		correctingCellKey: string | null;
		sidebarCollapsed: boolean;
		presentingAll: boolean;
		hasAbsentCells: boolean;
		onToggleAttendance: (row: Sf2PreviewStudentRow, cell: Sf2PreviewCell | MatrixCell) => void;
		onPresentAll: () => void;
		onToggleSidebar: () => void;
		onGenderFilterChange?: (value: 'all' | 'male' | 'female') => void;
	} = $props();
</script>

<div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background">
	<div class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-5 py-4">
		<div>
			<h2 class="text-xl font-semibold">
				{previewTemplateGradeLevel} - {previewTemplateSection}
			</h2>
		</div>
		<div class="flex flex-wrap items-center gap-2 text-xs">
			{#if onGenderFilterChange}
				<div
					class="flex overflow-hidden rounded-md border border-border bg-surface"
					role="group"
					aria-label="Gender filter"
				>
					<button
						type="button"
						aria-pressed={genderFilter === 'all'}
						onclick={() => onGenderFilterChange('all')}
						class="px-2.5 py-1.5 text-xs font-medium transition-colors {genderFilter === 'all'
							? 'bg-background text-foreground shadow-sm'
							: 'text-muted-foreground hover:text-foreground'}"
					>
						All
					</button>
					<button
						type="button"
						aria-pressed={genderFilter === 'male'}
						onclick={() => onGenderFilterChange('male')}
						class="px-2.5 py-1.5 text-xs font-medium transition-colors {genderFilter === 'male'
							? 'bg-background text-foreground shadow-sm'
							: 'text-muted-foreground hover:text-foreground'}"
					>
						Male
					</button>
					<button
						type="button"
						aria-pressed={genderFilter === 'female'}
						onclick={() => onGenderFilterChange('female')}
						class="px-2.5 py-1.5 text-xs font-medium transition-colors {genderFilter === 'female'
							? 'bg-background text-foreground shadow-sm'
							: 'text-muted-foreground hover:text-foreground'}"
					>
						Female
					</button>
				</div>
			{/if}

			<div class="h-4 w-px bg-border" aria-hidden="true"></div>

			<button
				type="button"
				onclick={onPresentAll}
				disabled={!hasAbsentCells || presentingAll}
				class="control-ring inline-flex h-7 items-center gap-1.5 rounded-md border border-border bg-background px-2.5 text-xs font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
				title="Clears all X marks, resetting every cell to Present (empty)"
			>
				{#if presentingAll}
					<Spinner />
				{:else}
					<CheckCheck class="size-3.5" aria-hidden="true" />
				{/if}
				{presentingAll ? 'Clearing...' : 'Present All'}
			</button>

			<button
				type="button"
				onclick={onToggleSidebar}
				class="control-ring inline-flex h-7 items-center gap-1.5 rounded-md border border-border bg-background px-2.5 text-xs font-medium transition-colors hover:bg-surface"
				title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
			>
				{#if sidebarCollapsed}
					<PanelRightOpen class="size-3.5" aria-hidden="true" />
					Expand sidebar
				{:else}
					<PanelRightClose class="size-3.5" aria-hidden="true" />
					Collapse sidebar
				{/if}
			</button>
		</div>
	</div>

	<div class="report-table-clip relative min-h-0 flex-1 overflow-hidden">
		<div bind:this={scrollEl} class="report-table-scroll absolute inset-0">
			<table
				bind:this={tableEl}
				class="min-w-full table-fixed border-separate border-spacing-0 text-sm"
			>
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
								class="sf2-tint-week sticky top-0 z-20 border-b border-l-2 border-border border-l-primary/45 px-2 py-2 text-center"
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
							{#each week.slots as slot (slot.key)}
								<th
									class="sticky top-[43px] z-10 min-w-14 border-b border-border bg-card px-2 py-2 text-center {slot.weekStart
										? 'border-l-2 border-l-primary/45'
										: 'border-l border-l-border/60'}"
									title={slot.title}
								>
									<div class="font-mono text-sm leading-none font-bold">
										{slot.dayNumber}
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
						<tr class={row.mapped ? 'sf2-row-mapped' : 'sf2-row-unmapped'}>
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
							{#each matrixDates as slot, columnIndex (slot.key)}
								{@const cell = row.cellColumns[columnIndex] ?? null}
								<td
									class="border-b border-border/80 px-1.5 py-1.5 text-center {slot.weekStart
										? 'sf2-tint-day border-l-2 border-l-primary/30'
										: 'border-l border-l-border/40'}"
								>
									{#if cell}
										<button
											type="button"
											disabled={!cell.editable || !row.mapped || correctingCellKey !== null}
											onclick={() => onToggleAttendance(row, cell)}
											aria-label={cell.label}
											title={cell.label}
											class="control-ring inline-grid size-9 place-items-center rounded-md border text-xs font-bold disabled:cursor-not-allowed disabled:opacity-70 {cell.cls}"
										>
											{#if correctingCellKey === cell.key}
												<span class="font-mono text-[10px]">...</span>
											{:else if cell.status === 'absent'}
												<X class="size-4" aria-hidden="true" />
											{:else}
												<!-- Empty = Present/Open (clickable, no visual mark) -->
											{/if}
										</button>
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
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<!-- NOTE: no edge-fade gradient overlays. A full-height translucent
		     gradient forces the compositor to re-blend the entire viewport
		     edge on every scroll frame over a multi-thousand-node sticky
		     table — the dominant horizontal-scroll cost. -->
		<ReportScrollbars target={scrollEl} />
	</div>
</div>

<style>
	/* NOTE: row-level `content-visibility: auto` / `contain: layout paint style` used to
	   live here. It was inert: `content-visibility` only applies where size containment
	   can apply, and size containment does not apply to internal table boxes (<tr>), so
	   not one offscreen row was ever skipped. Skipping rows for real needs JS windowing
	   (or a non-table row element), not this. */

	.report-table-scroll {
		overflow: auto !important;
		/* Native bars are hidden on purpose: they belong to the OS, and under
		   overlay-scrollbar settings (or the old -12px clip trick misfiring)
		   the vertical bar can vanish with no recourse. The overlay thumbs in
		   report-scrollbars.svelte own both axes from live metrics, so they
		   render exactly when there is overflow - and take no layout space. */
		scrollbar-width: none;
		/* No rubber-banding past the grid edges in any direction. */
		overscroll-behavior: none;
		/* Hint the compositor the scroll offset animates every frame; paints
		   then track the layer instead of re-rasterizing the whole table. */
		will-change: scroll-position;
	}
	.report-table-scroll::-webkit-scrollbar {
		display: none !important;
		width: 0 !important;
		height: 0 !important;
	}
	/* 1500+ cell buttons each carry `.control-ring`'s 150ms border/bg/color
	   transition. During scroll that keeps the style engine diffing every
	   button per frame — kill transitions inside the grid (focus ring kept). */
	.report-table-scroll .control-ring {
		transition: none;
	}
	/* Keep sticky chrome on its own layers so scrolling re-composites the
	   headers/left column instead of repainting the table underneath. */
	.report-table-scroll thead th,
	.report-table-scroll tbody th {
		backface-visibility: hidden;
	}
</style>
