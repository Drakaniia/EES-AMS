<script lang="ts">
	import { AlertCircle, CheckCircle2, X } from 'lucide-svelte';
	import { tick } from 'svelte';
	import { enterPanel } from '$lib/motion';

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
	let panelEl = $state<HTMLDivElement | null>(null);

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
			tick().then(() => panelEl && enterPanel(panelEl, { from: { opacity: 0, y: 8 } }));
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
		bind:this={panelEl}
		class="fixed right-4 bottom-4 z-[70] flex max-w-[min(26rem,calc(100vw-2rem))] items-start gap-2.5 rounded-xl border border-border bg-card px-3.5 py-3 text-[12.5px] font-medium shadow-[var(--shadow-lifted)] md:right-6"
		role={ok ? 'status' : 'alert'}
		aria-live={ok ? 'polite' : 'assertive'}
		onmouseenter={handleMouseEnter}
		onmouseleave={handleMouseLeave}
	>
		<Icon
			class="mt-px size-4 shrink-0 {ok ? 'text-primary' : 'text-destructive'}"
			strokeWidth={1.9}
			aria-hidden="true"
		/>
		<span class="text-balance-safe min-w-0 flex-1 leading-5">{message}</span>
		{#if actionLabel && onAction}
			<button type="button" onclick={onAction} class="btn btn-secondary btn-sm shrink-0">
				{actionLabel}
			</button>
		{/if}
		{#if onClose}
			<button
				type="button"
				onclick={onClose}
				class="icon-btn -mt-1 -mr-1.5 size-6 shrink-0"
				aria-label="Dismiss message"
			>
				<X class="size-3.5" aria-hidden="true" />
			</button>
		{/if}
	</div>
{/if}
