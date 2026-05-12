<script lang="ts">
	interface Props {
		currentPage: number;
		totalPages: number;
		onPageChange: (page: number) => void;
	}

	let { currentPage, totalPages, onPageChange }: Props = $props();

	function goToPage(page: number) {
		if (page >= 1 && page <= totalPages) {
			onPageChange(page);
		}
	}

	function nextPage() {
		goToPage(currentPage + 1);
	}

	function prevPage() {
		goToPage(currentPage - 1);
	}

	// Page numbers logic: show exactly a 3-page window around current page
	const pages = $derived.by(() => {
		const range = 3;
		if (totalPages <= range) {
			return Array.from({ length: totalPages }, (_, i) => i + 1);
		}

		let start = Math.max(1, currentPage - 1);
		let end = Math.min(totalPages, start + range - 1);

		if (end === totalPages) {
			start = totalPages - range + 1;
		}

		const result: (number | 'ellipsis')[] = [];

		if (start > 1) {
			result.push(1);
			if (start > 2) result.push('ellipsis');
		}

		for (let i = start; i <= end; i++) {
			// Skip if already added as first page
			if (i === 1 && start > 1) continue;
			result.push(i);
		}

		if (end < totalPages) {
			if (end < totalPages - 1) result.push('ellipsis');
			result.push(totalPages);
		}

		return result;
	});
</script>

<!-- Pagination controls -->
<div
	class="border-border bg-background inline-flex items-center gap-1 rounded-lg border p-1 shadow-sm"
>
	<!-- Previous button -->
	<button
		onclick={prevPage}
		disabled={currentPage === 1}
		class="border-border hover:bg-surface inline-flex items-center gap-2 rounded-md border px-3 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50"
		aria-label="Previous page"
	>
		<svg
			class="size-4"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<polyline points="15 18 9 12 15 6" />
		</svg>
		<span class="hidden sm:inline">Previous</span>
	</button>

	<!-- Page numbers -->
	<div class="flex gap-1">
		{#each pages as p, i (i)}
			{#if p === 'ellipsis'}
				<span class="text-muted-foreground px-2 py-2 text-sm">...</span>
			{:else}
				{@const isCurrent = p === currentPage}
				{#if isCurrent}
					<button
						class="bg-surface rounded-md px-3 py-2 text-sm font-medium shadow-sm"
						aria-label="Go to page {p}"
						aria-current="page"
					>
						{p}
					</button>
				{:else}
					<button
						onclick={() => goToPage(p as number)}
						class="border-border hover:bg-surface rounded-md border px-3 py-2 text-sm font-medium transition-colors"
						aria-label="Go to page {p}"
					>
						{p}
					</button>
				{/if}
			{/if}
		{/each}
	</div>

	<!-- Next button -->
	<button
		onclick={nextPage}
		disabled={currentPage === totalPages}
		class="border-border hover:bg-surface inline-flex items-center gap-2 rounded-md border px-3 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50"
		aria-label="Next page"
	>
		<span class="hidden sm:inline">Next</span>
		<svg
			class="size-4"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<polyline points="9 18 15 12 9 6" />
		</svg>
	</button>
</div>
