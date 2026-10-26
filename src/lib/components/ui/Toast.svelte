<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte';
	import { AlertCircle, CheckCircle2, Download, Info, X } from 'lucide-svelte';
	import { enterPanel } from '$lib/motion';

	interface Props {
		type?: 'info' | 'success' | 'warning' | 'error' | 'update';
		title?: string;
		message?: string;
		duration?: number;
		closable?: boolean;
		show?: boolean;
		actionText?: string;
		actionDisabled?: boolean;
		action?: () => void;
		onClose?: () => void;
		onAction?: () => void;
	}

	let {
		type = 'info',
		title = '',
		message = '',
		duration = 5000,
		closable = true,
		show = true,
		actionText = '',
		actionDisabled = false,
		action,
		onClose,
		onAction
	}: Props = $props();

	let timeoutId: ReturnType<typeof setTimeout>;
	let panelEl = $state<HTMLDivElement | null>(null);

	const icons = {
		info: Info,
		success: CheckCircle2,
		warning: AlertCircle,
		error: AlertCircle,
		update: Download
	};

	const iconTone = {
		info: 'text-muted-foreground',
		success: 'text-primary',
		warning: 'text-amber-600 dark:text-amber-400',
		error: 'text-destructive',
		update: 'text-primary'
	};

	function close() {
		show = false;
		onClose?.();
	}

	function handleAction() {
		if (actionDisabled) return;
		action?.();
		onAction?.();
	}

	$effect(() => {
		if (show)
			tick().then(() => panelEl && enterPanel(panelEl, { from: { opacity: 0, scale: 1, y: 10 } }));
	});

	onMount(() => {
		if (duration > 0) {
			timeoutId = setTimeout(() => {
				close();
			}, duration);
		}
	});

	function cleanup() {
		if (timeoutId) {
			clearTimeout(timeoutId);
		}
	}

	onDestroy(cleanup);

	const IconComponent = $derived(icons[type]);
	const iconColorClass = $derived(iconTone[type]);
</script>

{#if show}
	<div
		class="fixed right-4 bottom-4 z-[70] w-full max-w-[22rem]"
		role={type === 'error' ? 'alert' : 'status'}
		aria-live={type === 'error' ? 'assertive' : 'polite'}
	>
		<div bind:this={panelEl} class="toast-panel flex items-start gap-3 p-3.5">
			{#if actionDisabled && type === 'update'}
				<span class="mt-1.5 block size-1.5 shrink-0 rounded-full bg-primary" aria-hidden="true"
				></span>
			{:else}
				<IconComponent class="mt-px size-[18px] shrink-0 {iconColorClass}" strokeWidth={1.9} />
			{/if}

			<div class="min-w-0 flex-1">
				{#if title}
					<h3 class="text-[13px] leading-5 font-semibold tracking-[-0.008em]">{title}</h3>
				{/if}
				{#if message}
					<p class="text-[12.5px] leading-[1.5] whitespace-pre-line text-muted-foreground">
						{message}
					</p>
				{/if}

				{#if actionText}
					<button
						onclick={handleAction}
						disabled={actionDisabled}
						class="btn btn-secondary btn-sm mt-2.5"
					>
						{actionText}
					</button>
				{/if}
			</div>

			{#if closable}
				<button
					onclick={close}
					class="icon-btn -mt-1 -mr-1 size-6 shrink-0"
					aria-label="Close notification"
				>
					<X class="size-3.5" aria-hidden="true" />
				</button>
			{/if}
		</div>
	</div>
{/if}

<style>
	.toast-panel {
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background-color: var(--color-card);
		box-shadow: var(--shadow-lifted);
	}
</style>
