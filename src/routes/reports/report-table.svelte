<script lang="ts">
	import {
		AlertTriangle,
		X,
		CheckCheck,
		Maximize2,
		Minimize2,
		ChevronLeft,
		ChevronRight
	} from 'lucide-svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import type { Sf2PreviewStudentRow, Sf2PreviewCell } from '$lib/db-rust';
	import type {
		MatrixWeekGroup,
		MatrixDateSlot,
		MatrixStudentRow,
		MatrixCell
	} from './report-state.svelte';
	import { weekRangeLabel } from './report-state.svelte';

	let scrollEl: HTMLDivElement | undefined = $state(undefined);
	let tableEl: HTMLTableElement | undefined = $state(undefined);
	let topScrollEl: HTMLDivElement | undefined = $state(undefined);
	let canScrollLeft = $state(false);
	let canScrollRight = $state(false);
	let showTopBar = $state(false);
	let contentWidth = $state(0);

	// Cached geometry for the affordances. The scroll listeners used to read
	// scrollWidth/clientWidth and then write scrollLeft on the other scroller,
	// which forced a synchronous reflow of the entire grid on every scroll event
	// (vertical scrolling included) — the reason scrolling dropped frames. Reads
	// now happen only from a ResizeObserver, and listeners work off that cache.
	let viewportWidth = 0;
	let lastScrollLeft = -1;
	let measureFrame = 0;
	let affordanceFrame = 0;

	function updateAffordances(scrollLeft: number) {
		const maxScroll = contentWidth - viewportWidth;
		const left = scrollLeft > 2;
		const right = maxScroll > 2 && scrollLeft < maxScroll - 2;
		// Assign only on change: unrelated writes still re-render the fades.
		if (left !== canScrollLeft) canScrollLeft = left;
		if (right !== canScrollRight) canScrollRight = right;
	}

	function scheduleAffordance(scrollLeft: number) {
		if (affordanceFrame) return;
		affordanceFrame = requestAnimationFrame(() => {
			affordanceFrame = 0;
			updateAffordances(scrollLeft);
		});
	}

	/** Layout reads live here — never call this from a scroll listener. */
	function measure() {
		measureFrame = 0;
		if (!scrollEl) return;
		viewportWidth = scrollEl.clientWidth;
		contentWidth = scrollEl.scrollWidth;
		const show = contentWidth > viewportWidth + 2;
		if (show !== showTopBar) showTopBar = show;
		updateAffordances(scrollEl.scrollLeft);
	}

	function scheduleMeasure() {
		if (measureFrame) return;
		measureFrame = requestAnimationFrame(measure);
	}

	/**
	 * The top strip is a passive indicator, not a second scroller: it is driven
	 * one-way from the grid. Mirroring it used to need a scroll listener, an
	 * `isSyncing` flag and a rAF to break the feedback loop, because the write
	 * fired a scroll event straight back at the grid.
	 */
	function syncTopBar(scrollLeft: number) {
		const target = topScrollEl;
		if (!target || Math.abs(target.scrollLeft - scrollLeft) < 0.5) return;
		target.scrollLeft = scrollLeft;
	}

	function onMainScroll() {
		const el = scrollEl;
		if (!el) return;
		const scrollLeft = el.scrollLeft;
		// Vertical scrolling fires 'scroll' too — bail before doing any work.
		if (scrollLeft === lastScrollLeft) return;
		lastScrollLeft = scrollLeft;
		scheduleAffordance(scrollLeft);
		syncTopBar(scrollLeft);
	}

	function nudge(dir: 1 | -1) {
		scrollEl?.scrollBy({ left: dir * 280, behavior: 'auto' });
	}

	// NOTE: no custom wheel handler. A Svelte `onwheel` binding is
	// non-passive by default, so the browser must run JS before it may
	// start scrolling — every vertical wheel tick stalls on the main
	// thread. Native Shift+wheel already pans horizontally, so
	// intercepting it buys nothing and janks vertical scroll.

	$effect(() => {
		// Re-measure whenever the table shape (and therefore its width) changes.
		void matrixDates.length;
		void matrixStudents.length;
		const ro = new ResizeObserver(scheduleMeasure);
		if (scrollEl) ro.observe(scrollEl);
		if (tableEl) ro.observe(tableEl);
		window.addEventListener('resize', scheduleMeasure, { passive: true });
		scheduleMeasure();
		return () => {
			ro.disconnect();
			window.removeEventListener('resize', scheduleMeasure);
			if (measureFrame) cancelAnimationFrame(measureFrame);
			if (affordanceFrame) cancelAnimationFrame(affordanceFrame);
			measureFrame = 0;
			affordanceFrame = 0;
		};
	});

	let {
		previewTemplateGradeLevel,
		previewTemplateSection,
		genderFilter,
		matrixWeekGroups,
		matrixDates,
		matrixStudents,
		correctingCellKey,
		fullReview,
		presentingAll,
		hasAbsentCells,
		onToggleAttendance,
		onPresentAll,
		onFullReviewOpen,
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
		fullReview: boolean;
		presentingAll: boolean;
		hasAbsentCells: boolean;
		onToggleAttendance: (row: Sf2PreviewStudentRow, cell: Sf2PreviewCell | MatrixCell) => void;
		onPresentAll: () => void;
		onFullReviewOpen: () => void;
		onGenderFilterChange?: (value: 'all' | 'male' | 'female') => void;
	} = $props();
</script>

<div
	class="flex min-h-0 flex-1 flex-col border border-border bg-card shadow-sm {fullReview
		? 'rounded-xl'
		: 'rounded-2xl'}"
>
	<div class="flex flex-wrap items-start justify-between gap-3 border-b border-border px-5 py-4">
		<div>
			<div class="label-mono text-primary">SF2 attendance grid</div>
			<h2 class="mt-1 text-xl font-semibold">
				{previewTemplateGradeLevel} - {previewTemplateSection}
			</h2>
			<p class="mt-1 text-sm text-muted-foreground">
				Click a cell to toggle the learner between present and absent.
			</p>
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
				onclick={onFullReviewOpen}
				class="control-ring inline-flex h-7 items-center gap-1.5 rounded-md border border-border bg-background px-2.5 text-xs font-medium transition-colors hover:bg-surface"
				title={fullReview ? 'Exit full preview' : 'Open full preview'}
			>
				{#if fullReview}
					<Minimize2 class="size-3.5" aria-hidden="true" />
					Exit Full Preview
				{:else}
					<Maximize2 class="size-3.5" aria-hidden="true" />
					Full Preview
				{/if}
			</button>
		</div>
	</div>

	{#if showTopBar}
		<!-- Passive indicator: reflects the grid's horizontal offset only. It takes
		     no pointer input, so there is no second live scroller on the scroll path.
		     Opaque background: a translucent strip would force the compositor to
		     blend it against the page on every scroll frame. -->
		<div
			bind:this={topScrollEl}
			class="report-top-scroll pointer-events-none shrink-0 overflow-x-auto overflow-y-hidden border-b border-border bg-surface"
			aria-hidden="true"
		>
			<div style:width="{contentWidth}px" class="h-3.5"></div>
		</div>
	{/if}

	<div class="report-table-clip relative min-h-0 flex-1 overflow-hidden">
		<div
			bind:this={scrollEl}
			onscroll={onMainScroll}
			class="report-table-scroll absolute inset-0 [scrollbar-gutter:stable]"
		>
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
		     table — the dominant horizontal-scroll cost. The chevron nudge
		     buttons below are the scroll affordance. -->

		{#if showTopBar}
			<button
				type="button"
				onclick={() => nudge(-1)}
				disabled={!canScrollLeft}
				class="absolute top-3 left-2 z-[14] hidden h-7 w-7 place-items-center rounded-full border border-border bg-card disabled:opacity-30 md:grid"
				aria-label="Scroll table left"
				title="Scroll left (also Shift+wheel)"
			>
				<ChevronLeft class="size-3.5" />
			</button>
			<button
				type="button"
				onclick={() => nudge(1)}
				disabled={!canScrollRight}
				class="absolute top-3 right-2 z-[14] hidden h-7 w-7 place-items-center rounded-full border border-border bg-card disabled:opacity-30 md:grid"
				aria-label="Scroll table right"
				title="Scroll right (also Shift+wheel)"
			>
				<ChevronRight class="size-3.5" />
			</button>
		{/if}
	</div>
</div>

<style>
	/* NOTE: row-level `content-visibility: auto` / `contain: layout paint style` used to
	   live here. It was inert: `content-visibility` only applies where size containment
	   can apply, and size containment does not apply to internal table boxes (<tr>), so
	   not one offscreen row was ever skipped. Skipping rows for real needs JS windowing
	   (or a non-table row element), not this. */

	/* Clip the bottom 12px where the native horizontal scrollbar would sit.
	   Push the native bar outside the parent's overflow-hidden so every
	   browser hides the duplicate bottom bar; the top .report-top-scroll
	   remains the single horizontal control. Vertical scrollbar stays visible. */
	.report-table-clip {
		scrollbar-gutter: stable;
	}
	.report-table-scroll {
		scrollbar-width: auto;
		scrollbar-color: color-mix(in oklab, var(--color-foreground) 38%, transparent) transparent;
		/* The grid scrolls itself; the top strip only mirrors its offset one-way. The
		   native horizontal bar is pushed 12px below the parent's overflow-hidden
		   clip so it stays hidden. */
		overflow-x: auto !important;
		overflow-y: auto !important;
		bottom: -12px !important;
		padding-bottom: 12px;
		/* Hint the compositor the scroll offset animates every frame; paints
		   then track the layer instead of re-rasterizing the whole table. */
		will-change: scroll-position;
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
	.report-table-scroll::-webkit-scrollbar {
		width: 12px;
		height: 0;
	}
	.report-table-scroll::-webkit-scrollbar:horizontal {
		height: 0 !important;
		display: none !important;
	}
	.report-table-scroll::-webkit-scrollbar-track:horizontal,
	.report-table-scroll::-webkit-scrollbar-thumb:horizontal,
	.report-table-scroll::-webkit-scrollbar-corner {
		display: none !important;
		background: transparent !important;
		height: 0 !important;
	}
	.report-table-scroll::-webkit-scrollbar-track:vertical {
		background: transparent;
	}
	.report-table-scroll::-webkit-scrollbar-thumb:vertical {
		border: 2px solid transparent;
		border-radius: 999px;
		background-clip: content-box;
		background-color: color-mix(in oklab, var(--color-foreground) 34%, transparent);
	}
	.report-table-scroll::-webkit-scrollbar-thumb:vertical:hover {
		background-color: color-mix(in oklab, var(--color-foreground) 48%, transparent);
	}
	.report-top-scroll {
		scrollbar-width: thin;
		scrollbar-color: color-mix(in oklab, var(--color-primary) 55%, transparent) transparent;
	}
	.report-top-scroll::-webkit-scrollbar {
		height: 12px;
	}
	.report-top-scroll::-webkit-scrollbar-thumb {
		border: 2px solid transparent;
		border-radius: 999px;
		background-clip: content-box;
		background-color: color-mix(in oklab, var(--color-primary) 45%, transparent);
	}
	.report-top-scroll::-webkit-scrollbar-thumb:hover {
		background-color: color-mix(in oklab, var(--color-primary) 65%, transparent);
	}
</style>
