<script lang="ts">
	import { MATRIX_WEEKDAYS } from './report-state.svelte';

	type Props = {
		/** How many week groups to draw. One real month is four to six. */
		weeks?: number;
		label?: string;
	};

	let { weeks = 5, label = 'Loading the month' }: Props = $props();

	const weekIndexes = $derived.by(() => {
		const indexes: number[] = [];
		for (let index = 0; index < weeks; index += 1) indexes.push(index);
		return indexes;
	});
</script>

<!--
	A skeleton for the grid area only.

	The full-page loader this replaces blanked the route, which meant a class
		switch felt like the app had stopped. Here the sidebar, the month picker and
	the class selector stay live and only the cells are placeholders - and because
	a month read is a single SQL query, this is on screen for milliseconds.
-->
<div
	class="surface-panel flex h-full min-h-0 flex-col gap-3 p-5"
	role="status"
	aria-live="polite"
	aria-busy="true"
>
	<span class="sr-only">{label}</span>
	<div class="skeleton h-5 w-48 rounded-md" aria-hidden="true"></div>
	<div class="min-h-0 flex-1 space-y-2 overflow-hidden" aria-hidden="true">
		{#each weekIndexes as week (week)}
			<div class="flex gap-1.5">
				<div class="skeleton h-7 w-16 shrink-0 rounded-md"></div>
				{#each MATRIX_WEEKDAYS as weekday (weekday)}
					<div class="skeleton h-7 flex-1 rounded-md"></div>
				{/each}
			</div>
		{/each}
	</div>
	<div class="space-y-2" aria-hidden="true">
		{#each weekIndexes.slice(0, 3) as row (row)}
			<div class="skeleton h-6 w-full rounded-md"></div>
		{/each}
	</div>
</div>
