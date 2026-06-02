<script lang="ts">
	import { tick } from 'svelte';
	import { fade, fly } from 'svelte/transition';
	import { X } from 'lucide-svelte';

	type Props = {
		open: boolean;
		title: string;
		description?: string;
		maxWidth?: 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl';
		showCloseButton?: boolean;
		onClose?: () => void;
		children?: import('svelte').Snippet;
	};

	let {
		open,
		title,
		description,
		maxWidth = 'md',
		showCloseButton = true,
		onClose,
		children
	}: Props = $props();

	let panel = $state<HTMLDivElement | null>(null);
	let previousFocus: Element | null = null;
	const titleId = $derived(
		`dialog-title-${
			title
				.toLowerCase()
				.replace(/[^a-z0-9]+/g, '-')
				.replace(/(^-|-$)/g, '') || 'panel'
		}`
	);
	const descriptionId = $derived(`${titleId}-description`);

	const maxWidthClasses = {
		sm: 'max-w-sm',
		md: 'max-w-md',
		lg: 'max-w-lg',
		xl: 'max-w-xl',
		'2xl': 'max-w-2xl',
		'3xl': 'max-w-3xl',
		'4xl': 'max-w-4xl'
	};

	const widthClass = $derived(maxWidthClasses[maxWidth]);

	$effect(() => {
		if (!open) {
			if (previousFocus instanceof HTMLElement) previousFocus.focus();
			previousFocus = null;
			return;
		}

		previousFocus = document.activeElement;
		tick().then(() => {
			panel?.focus();
		});
	});

	function handleClose() {
		onClose?.();
	}

	function handleBackdropClick(event: MouseEvent) {
		if (event.target === event.currentTarget) handleClose();
	}

	function getFocusableElements() {
		if (!panel) return [];
		return Array.from(
			panel.querySelectorAll<HTMLElement>(
				'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
			)
		).filter((element) => !element.hasAttribute('disabled') && element.offsetParent !== null);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			handleClose();
			return;
		}

		if (event.key !== 'Tab') return;

		const focusable = getFocusableElements();
		if (focusable.length === 0) {
			event.preventDefault();
			panel?.focus();
			return;
		}

		const first = focusable[0];
		const last = focusable[focusable.length - 1];

		if (event.shiftKey && document.activeElement === first) {
			event.preventDefault();
			last.focus();
		} else if (!event.shiftKey && document.activeElement === last) {
			event.preventDefault();
			first.focus();
		}
	}
</script>

{#if open}
	<div
		class="fixed inset-0 z-40 bg-foreground/45 backdrop-blur-[2px]"
		role="presentation"
		onclick={handleBackdropClick}
		transition:fade={{ duration: 120 }}
	></div>

	<div class="fixed inset-0 z-50 flex items-center justify-center p-4">
		<div
			bind:this={panel}
			class="{widthClass} max-h-[min(86vh,760px)] w-full overflow-hidden rounded-2xl border border-border bg-background shadow-2xl"
			role="dialog"
			aria-modal="true"
			aria-labelledby={titleId}
			aria-describedby={description ? descriptionId : undefined}
			tabindex="-1"
			onkeydown={handleKeydown}
			transition:fly={{ duration: 160, y: -12 }}
		>
			<div class="flex items-start justify-between gap-4 border-b border-border px-5 py-4">
				<div class="min-w-0 flex-1">
					<h2
						id={titleId}
						class="text-balance-safe text-lg leading-tight font-semibold text-foreground"
					>
						{title}
					</h2>
					{#if description}
						<p
							id={descriptionId}
							class="text-balance-safe mt-1 text-sm leading-5 text-muted-foreground"
						>
							{description}
						</p>
					{/if}
				</div>
				{#if showCloseButton}
					<button
						type="button"
						onclick={handleClose}
						class="control-ring inline-grid size-9 shrink-0 place-items-center rounded-md border border-transparent text-muted-foreground hover:border-border hover:bg-surface hover:text-foreground"
						aria-label="Close dialog"
					>
						<X class="size-4" aria-hidden="true" />
					</button>
				{/if}
			</div>

			<div class="max-h-[calc(min(86vh,760px)-82px)] overflow-y-auto px-5 py-5">
				<div class="space-y-4">
					{@render children?.()}
				</div>
			</div>
		</div>
	</div>
{/if}
