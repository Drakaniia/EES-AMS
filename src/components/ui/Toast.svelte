<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { onMount } from 'svelte';
	import { X, CheckCircle, AlertCircle, Info, Download } from 'lucide-svelte';

	interface Props {
		type?: 'info' | 'success' | 'warning' | 'error' | 'update';
		title?: string;
		message?: string;
		duration?: number;
		closable?: boolean;
		show?: boolean;
		actionText?: string;
		action?: () => void;
	}

	let {
		type = 'info',
		title = '',
		message = '',
		duration = 5000,
		closable = true,
		show = true,
		actionText = '',
		action = $bindable(() => {})
	}: Props = $props();

	const dispatch = createEventDispatcher();
	let timeoutId: ReturnType<typeof setTimeout>;

	const icons = {
		info: Info,
		success: CheckCircle,
		warning: AlertCircle,
		error: AlertCircle,
		update: Download
	};

	const colors = {
		info: 'bg-blue-50 border-blue-200 text-blue-800',
		success: 'bg-green-50 border-green-200 text-green-800',
		warning: 'bg-yellow-50 border-yellow-200 text-yellow-800',
		error: 'bg-red-50 border-red-200 text-red-800',
		update: 'bg-purple-50 border-purple-200 text-purple-800'
	};

	const iconColors = {
		info: 'text-blue-500',
		success: 'text-green-500',
		warning: 'text-yellow-500',
		error: 'text-red-500',
		update: 'text-purple-500'
	};

	function close() {
		show = false;
		dispatch('close');
	}

	function handleAction() {
		if (action) {
			action();
		}
		dispatch('action');
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

	$effect(() => {
		return cleanup;
	});

	const IconComponent = $derived(icons[type]);
	const colorClass = $derived(colors[type]);
	const iconColorClass = $derived(iconColors[type]);
</script>

{#if show}
	<div
		class="fixed top-4 right-4 z-50 max-w-sm w-full animate-in slide-in-from-top-full duration-300"
		role="alert"
		aria-live="polite"
	>
		<div class="flex items-start p-4 border rounded-lg shadow-lg {colorClass}">
			<div class="shrink-0">
				<IconComponent class="w-6 h-6 {iconColorClass}" />
			</div>
			
			<div class="ml-3 flex-1">
				{#if title}
					<h3 class="text-sm font-semibold mb-1">{title}</h3>
				{/if}
				{#if message}
					<p class="text-sm">{message}</p>
				{/if}
				
				{#if actionText}
					<button
						onclick={handleAction}
						class="mt-2 px-3 py-1 text-xs font-medium rounded bg-white/20 hover:bg-white/30 transition-colors"
					>
						{actionText}
					</button>
				{/if}
			</div>
			
			{#if closable}
				<div class="ml-4 shrink-0">
					<button
						onclick={close}
						class="inline-flex rounded-md p-1.5 hover:bg-white/20 transition-colors"
						aria-label="Close notification"
					>
						<X class="w-4 h-4" />
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	@keyframes slide-in-from-top-full {
		from {
			transform: translateY(-100%);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	.animate-in {
		animation: slide-in-from-top-full 0.3s ease-out;
	}
</style>
