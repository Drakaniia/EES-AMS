<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { fly } from 'svelte/transition';

	type Props = {
		open: boolean;
		title: string;
		description?: string;
		maxWidth?: 'sm' | 'md' | 'lg' | 'xl';
		showCloseButton?: boolean;
		children?: import('svelte').Snippet;
	};

	let {
		open,
		title,
		description,
		maxWidth = 'md',
		showCloseButton = true,
		children
	}: Props = $props();

	const dispatch = createEventDispatcher();

	function handleClose() {
		dispatch('close');
	}

	function handleBackdropClick(event: MouseEvent) {
		if (event.target === event.currentTarget) {
			handleClose();
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			handleClose();
		}
	}

	const maxWidthClasses = {
		sm: 'max-w-sm',
		md: 'max-w-md',
		lg: 'max-w-lg',
		xl: 'max-w-xl'
	};

	const widthClass = $derived(maxWidthClasses[maxWidth]);
</script>

{#if open}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={handleBackdropClick}
		onkeydown={handleKeydown}
		tabindex="-1"
	></div>

	<!-- Dialog Panel -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="dialog-title"
		tabindex="-1"
		onkeydown={handleKeydown}
	>
		<div
			class="border-border bg-background {widthClass} w-full space-y-5 rounded-2xl border p-6 shadow-xl"
			transition:fly={{ duration: 200, y: -20 }}
		>
			<!-- Header -->
			<div class="flex items-start justify-between gap-4">
				<div class="flex-1">
					<h2 id="dialog-title" class="text-lg font-semibold text-foreground">
						{title}
					</h2>
					{#if description}
						<p class="mt-1 text-sm text-muted-foreground">{description}</p>
					{/if}
				</div>
				{#if showCloseButton}
					<button
						type="button"
						onclick={handleClose}
						class="text-muted-foreground transition-colors hover:text-foreground"
						aria-label="Close dialog"
					>
						<svg
							class="size-5"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<line x1="18" y1="6" x2="6" y2="18"></line>
							<line x1="6" y1="6" x2="18" y2="18"></line>
						</svg>
					</button>
				{/if}
			</div>

			<!-- Content -->
			<div class="space-y-4">
				{@render children?.()}
			</div>
		</div>
	</div>
{/if}
