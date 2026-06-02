<script lang="ts">
	import { ChevronLeft, ChevronRight } from 'lucide-svelte';

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

{#if totalPages > 1}
	<nav aria-label="Pagination" class="flex justify-center">
		<div
			class="inline-flex max-w-[calc(100vw-2rem)] items-center gap-1 overflow-x-auto rounded-xl border border-border bg-background/95 p-1 shadow-lg backdrop-blur"
		>
			<button
				onclick={prevPage}
				disabled={currentPage === 1}
				class="control-ring inline-flex h-9 items-center gap-1.5 rounded-md border border-transparent px-3 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-45"
				aria-label="Previous page"
			>
				<ChevronLeft class="size-4" aria-hidden="true" />
				<span class="hidden sm:inline">Previous</span>
			</button>

			<div class="flex items-center gap-1">
				{#each pages as p, i (i)}
					{#if p === 'ellipsis'}
						<span class="grid size-9 place-items-center text-sm text-muted-foreground">...</span>
					{:else}
						{@const isCurrent = p === currentPage}
						<button
							onclick={() => goToPage(p)}
							class="control-ring grid size-9 shrink-0 place-items-center rounded-md border border-transparent text-sm font-medium transition-colors
								{isCurrent
								? 'bg-primary text-primary-foreground shadow-sm'
								: 'text-muted-foreground hover:bg-surface hover:text-foreground'}"
							aria-label="Go to page {p}"
							aria-current={isCurrent ? 'page' : undefined}
							disabled={isCurrent}
						>
							{p}
						</button>
					{/if}
				{/each}
			</div>

			<button
				onclick={nextPage}
				disabled={currentPage === totalPages}
				class="control-ring inline-flex h-9 items-center gap-1.5 rounded-md border border-transparent px-3 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-45"
				aria-label="Next page"
			>
				<span class="hidden sm:inline">Next</span>
				<ChevronRight class="size-4" aria-hidden="true" />
			</button>
		</div>
	</nav>
{/if}
