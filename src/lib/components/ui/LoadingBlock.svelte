<script lang="ts">
	type Props = {
		rows?: number;
		label?: string;
	};

	let { rows = 4, label = 'Loading content' }: Props = $props();

	const skeletonRows = $derived.by(() => {
		const indexes: number[] = [];
		for (let index = 0; index < rows; index += 1) indexes.push(index);
		return indexes;
	});
</script>

<div class="surface-panel p-5" role="status" aria-live="polite" aria-busy="true">
	<span class="sr-only">{label}</span>
	<div class="space-y-3" aria-hidden="true">
		<div class="skeleton h-5 w-44 rounded-md"></div>
		<div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
			{#each skeletonRows as index (index)}
				<div class="rounded-xl border border-border bg-background/80 p-4">
					<div class="skeleton h-4 w-2/3 rounded-md"></div>
					<div class="skeleton mt-4 h-8 w-1/3 rounded-md"></div>
					<div class="skeleton mt-4 h-3 w-full rounded-md"></div>
				</div>
			{/each}
		</div>
	</div>
</div>
