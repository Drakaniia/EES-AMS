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
		Previous
	</button>

	<!-- Page numbers -->
	<div class="flex gap-1">
		{#each Array.from({ length: totalPages }, (_, i) => i + 1) as page (page)}
			{@const isCurrent = page === currentPage}
			{@const isNearCurrent =
				Math.abs(page - currentPage) <= 2 || page === 1 || page === totalPages}

			{#if isNearCurrent}
				{#if isCurrent}
					<button
						class="bg-surface rounded-md px-3 py-2 text-sm font-medium shadow-sm"
						aria-label="Go to page {page}"
						aria-current="page"
					>
						{page}
					</button>
				{:else}
					<button
						onclick={() => goToPage(page)}
						class="border-border hover:bg-surface rounded-md border px-3 py-2 text-sm font-medium transition-colors"
						aria-label="Go to page {page}"
					>
						{page}
					</button>
				{/if}
			{:else if page === 2 && currentPage > 4}
				<span class="text-muted-foreground px-2 py-2 text-sm">...</span>
			{:else if page === totalPages - 1 && currentPage < totalPages - 3}
				<span class="text-muted-foreground px-2 py-2 text-sm">...</span>
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
		Next
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
