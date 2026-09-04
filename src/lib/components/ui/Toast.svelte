<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { AlertCircle, CheckCircle, Download, Info, X } from 'lucide-svelte';

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

	const icons = {
		info: Info,
		success: CheckCircle,
		warning: AlertCircle,
		error: AlertCircle,
		update: Download
	};

	const colors = {
		info: 'border-border bg-background text-foreground',
		success: 'border-primary/25 bg-primary/10 text-foreground',
		warning: 'border-accent/25 bg-accent/10 text-foreground',
		error: 'border-destructive/25 bg-destructive/10 text-destructive',
		update: 'border-primary/40 bg-background text-foreground shadow-soft'
	};

	const iconColors = {
		info: 'text-muted-foreground',
		success: 'text-primary',
		warning: 'text-accent',
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
	const colorClass = $derived(colors[type]);
	const iconColorClass = $derived(iconColors[type]);
</script>

{#if show}
	<div
		class="fixed top-4 right-4 z-50 w-full max-w-sm"
		role={type === 'error' ? 'alert' : 'status'}
		aria-live={type === 'error' ? 'assertive' : 'polite'}
	>
		<div class="flex items-start rounded-lg border p-4 {colorClass}">
			<div class="shrink-0">
				{#if actionDisabled && type === 'update'}
					<span class="mt-2 block size-2 rounded-full bg-orange-600" aria-hidden="true"></span>
				{:else}
					<IconComponent class="h-6 w-6 {iconColorClass}" />
				{/if}
			</div>

			<div class="ml-3 flex-1">
				{#if title}
					<h3 class="mb-1 text-sm font-semibold">{title}</h3>
				{/if}
				{#if message}
					<p class="text-sm leading-5 whitespace-pre-line">{message}</p>
				{/if}

				{#if actionText}
					<button
						onclick={handleAction}
						disabled={actionDisabled}
						class="mt-3 rounded-md border border-border bg-surface px-3 py-1.5 text-xs font-semibold transition-colors hover:bg-surface/80 disabled:cursor-not-allowed disabled:opacity-70"
					>
						{actionText}
					</button>
				{/if}
			</div>

			{#if closable}
				<div class="ml-4 shrink-0">
					<button
						onclick={close}
						class="inline-flex rounded-md p-1.5 transition-colors hover:bg-surface"
						aria-label="Close notification"
					>
						<X class="h-4 w-4" />
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}
