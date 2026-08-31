<script lang="ts">
	import { AlertCircle, CheckCircle2, X } from 'lucide-svelte';

	type Props = {
		message: string | null;
		ok?: boolean;
		actionLabel?: string;
		onAction?: () => void;
		onClose?: () => void;
	};

	let { message, ok = true, actionLabel, onAction, onClose }: Props = $props();
	const Icon = $derived(ok ? CheckCircle2 : AlertCircle);

	let timer: ReturnType<typeof setTimeout> | null = null;
	let remaining = $state(3000);
	let startTime = $state(0);

	function startTimer() {
		startTime = Date.now();
		timer = setTimeout(() => {
			timer = null;
			onClose?.();
		}, remaining);
	}

	function pauseTimer() {
		if (timer) {
			clearTimeout(timer);
			timer = null;
			remaining -= Date.now() - startTime;
		}
	}

	$effect(() => {
		if (message) {
			remaining = 3000;
			startTimer();
		}
		return () => {
			if (timer) clearTimeout(timer);
			timer = null;
		};
	});

	function handleMouseEnter() {
		pauseTimer();
	}

	function handleMouseLeave() {
		if (message && remaining > 0) {
			startTimer();
		}
	}
</script>

{#if message}
	<div
		class="fixed right-4 bottom-4 z-[70] flex max-w-[min(28rem,calc(100vw-2rem))] items-start gap-3 rounded-2xl border px-4 py-3 text-sm font-medium md:top-12 md:right-6 md:bottom-auto
			{ok
			? 'border-primary/25 bg-background/96 text-foreground'
			: 'border-destructive/40 bg-background/96 text-destructive'}"
		role={ok ? 'status' : 'alert'}
		aria-live={ok ? 'polite' : 'assertive'}
		onmouseenter={handleMouseEnter}
		onmouseleave={handleMouseLeave}
	>
		<Icon class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
		<span class="text-balance-safe min-w-0 flex-1">{message}</span>
		{#if actionLabel && onAction}
			<button
				type="button"
				onclick={onAction}
				class="control-ring shrink-0 rounded-md border border-border bg-surface px-2 py-1 text-xs font-semibold text-primary hover:bg-primary/10"
			>
				{actionLabel}
			</button>
		{/if}
		{#if onClose}
			<button
				type="button"
				onclick={onClose}
				class="control-ring -mr-1 grid size-7 shrink-0 place-items-center rounded-md border border-transparent text-muted-foreground hover:border-border hover:bg-surface hover:text-foreground"
				aria-label="Dismiss message"
			>
				<X class="size-3.5" aria-hidden="true" />
			</button>
		{/if}
	</div>
{/if}
