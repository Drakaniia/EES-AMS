<script lang="ts">
	import { onDestroy, tick } from 'svelte';
	import { animate } from 'motion';
	import { X } from 'lucide-svelte';
	import { SPRING, enterPanel, exitPanel, prefersReducedMotion } from '$lib/motion';

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
	let scrim = $state<HTMLDivElement | null>(null);
	/** Held true through the exit animation so the panel can animate out. */
	let mounted = $state(false);
	let closing = false;
	let previousFocus: Element | null = null;
	let previousBodyOverflow = '';
	let hasAutoFocused = $state(false);

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
			hasAutoFocused = false;
			if (previousFocus instanceof HTMLElement) previousFocus.focus();
			previousFocus = null;
			if (typeof document !== 'undefined') document.body.style.overflow = previousBodyOverflow;
			return;
		}

		if (hasAutoFocused) return;

		previousFocus = document.activeElement;
		previousBodyOverflow = document.body.style.overflow;
		document.body.style.overflow = 'hidden';
		tick().then(() => {
			if (!panel) return;
			// Don't steal focus if user already focused an element inside the dialog
			// (e.g. clicked an input before tick resolved, or typing caused a re-render)
			const active = document.activeElement;
			if (active instanceof HTMLElement && panel.contains(active) && active !== panel) {
				hasAutoFocused = true;
				return;
			}
			const firstFocusable = getFocusableElements()[0];
			if (firstFocusable) {
				firstFocusable.focus();
			} else {
				panel?.focus();
			}
			hasAutoFocused = true;
		});
	});

	// Presence: spring in from just below the final position; fade and scale back
	// out on close. Starting from the live on-screen value is what makes a
	// re-opened dialog feel grabbable rather than queued.
	$effect(() => {
		if (open) {
			mounted = true;
			closing = false;
			tick().then(() => {
				if (panel) enterPanel(panel, { from: { opacity: 0, scale: 0.97, y: 8 } });
				if (scrim && !prefersReducedMotion()) {
					animate(scrim, { opacity: [0, 1] }, { duration: 0.18, ease: 'easeOut' });
				}
			});
			return;
		}

		if (mounted && !closing) {
			closing = true;
			const panelEl = panel;
			const scrimEl = scrim;
			Promise.all([
				panelEl ? exitPanel(panelEl, { to: { opacity: 0, scale: 0.985, y: 4 } }) : null,
				scrimEl && !prefersReducedMotion()
					? animate(scrimEl, { opacity: [1, 0] }, { duration: 0.14 }).finished.catch(() => {})
					: null
			]).then(() => {
				mounted = false;
				closing = false;
			});
		}
	});

	onDestroy(() => {
		if (typeof document !== 'undefined') document.body.style.overflow = previousBodyOverflow;
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

{#if mounted}
	<div
		bind:this={scrim}
		class="dialog-scrim fixed inset-x-0 top-[46px] bottom-0 z-40"
		role="presentation"
		onclick={handleBackdropClick}
	></div>

	<div class="fixed inset-x-0 top-[46px] bottom-0 z-50 flex items-center justify-center p-4 sm:p-6">
		<div
			bind:this={panel}
			class="dialog-panel {widthClass} max-h-[min(84vh,760px)] w-full overflow-hidden"
			role="dialog"
			aria-modal="true"
			aria-labelledby={titleId}
			aria-describedby={description ? descriptionId : undefined}
			tabindex="-1"
			onkeydown={handleKeydown}
		>
			<div class="dialog-header">
				<div class="min-w-0 flex-1">
					<h2
						id={titleId}
						class="text-balance-safe text-[17px] leading-tight font-semibold tracking-[-0.018em]"
					>
						{title}
					</h2>
					{#if description}
						<p
							id={descriptionId}
							class="text-balance-safe mt-1 text-[13px] leading-5 text-muted-foreground"
						>
							{description}
						</p>
					{/if}
				</div>
				{#if showCloseButton}
					<button
						type="button"
						onclick={handleClose}
						class="icon-btn shrink-0"
						aria-label="Close dialog"
					>
						<X class="size-4" aria-hidden="true" />
					</button>
				{/if}
			</div>

			<div class="dialog-body max-h-[calc(min(84vh,760px)-74px)] overflow-y-auto">
				<div class="space-y-4">
					{@render children?.()}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.dialog-scrim {
		background: var(--scrim);
	}

	.dialog-panel {
		border: 1px solid var(--color-border);
		border-radius: var(--radius-2xl);
		background: var(--color-card);
		box-shadow: var(--shadow-lifted);
		transform-origin: center;
	}

	.dialog-header {
		display: flex;
		gap: 12px;
		align-items: flex-start;
		justify-content: space-between;
		padding: 18px 18px 14px;
	}

	.dialog-body {
		padding: 0 18px 18px;
	}
</style>
