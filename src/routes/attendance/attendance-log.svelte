<script lang="ts">
	import { onDestroy } from 'svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import { fmtTime } from '$lib/csv';
	import type { LogLine, LastResult } from './attendance-state.svelte';

	let {
		log = $bindable([]),
		onUndo
	}: {
		log: LogLine[];
		onUndo: (eventId: string) => Promise<boolean>;
	} = $props();

	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = $state(null);
	let lastResult = $state<LastResult | null>(null);
	let lastEventId = $state<string | null>(null);
	let undoTimer: ReturnType<typeof setTimeout> | null = $state(null);

	onDestroy(() => {
		if (toastTimer) clearTimeout(toastTimer);
		if (undoTimer) clearTimeout(undoTimer);
	});

	export function showToast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	export function addLogEntry(line: LogLine) {
		log = [line, ...log].slice(0, 30);
	}

	export function addLogEntries(lines: LogLine[]) {
		log = [...lines, ...log].slice(0, 30);
	}

	export function removeLogEntry(id: string) {
		log = log.filter((l) => l.id !== id);
	}

	export function setUndo(eventId: string, result: LastResult) {
		lastEventId = eventId;
		lastResult = result;
		if (undoTimer) clearTimeout(undoTimer);
		undoTimer = setTimeout(() => {
			lastResult = null;
			lastEventId = null;
		}, 5000);
	}

	export function resetUndo() {
		lastResult = null;
		lastEventId = null;
		if (undoTimer) {
			clearTimeout(undoTimer);
			undoTimer = null;
		}
	}

	export function resetState() {
		resetUndo();
		log = [];
	}

	async function handleUndo() {
		if (!lastEventId || !lastResult) return;
		const eventIdToRemove = lastEventId;
		const resultName = lastResult.name;
		try {
			const success = await onUndo(lastEventId);
			if (success) {
				log = log.filter((line) => line.id !== eventIdToRemove);
				showToast(`Undid ${resultName} attendance`);
			} else {
				showToast('Failed to undo last action', false);
			}
		} catch {
			showToast('Failed to undo last action', false);
		} finally {
			lastEventId = null;
			lastResult = null;
			if (undoTimer) clearTimeout(undoTimer);
		}
	}
</script>

{#if lastResult}
	<div class="pointer-events-none fixed inset-x-0 bottom-10 z-50 flex justify-center px-4">
		<div
			class="pointer-events-auto flex max-w-[min(34rem,calc(100vw-2rem))] items-center gap-4 rounded-2xl border px-5 py-4 md:px-8 md:py-5
				{lastResult.ok
				? 'border-border bg-background text-foreground'
				: 'border-destructive bg-destructive text-destructive-foreground'}"
			role="status"
			aria-live="assertive"
		>
			<div
				class="grid size-12 place-items-center rounded-full
				{lastResult.isLate ? 'bg-destructive/20 text-destructive' : 'bg-primary/20 text-primary'}"
			>
				<svg
					class="size-6"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2.5"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<polyline points="20 6 9 17 4 12" />
				</svg>
			</div>
			<div class="min-w-0">
				<div class="text-balance-safe text-lg leading-tight font-bold md:text-xl">
					{lastResult.name}
				</div>
				<div class="label-mono flex gap-2">
					<span class={lastResult.isLate ? 'font-bold text-destructive' : ''}>
						{lastResult.isLate ? 'LATE' : 'IN'}
					</span>
					<span class="text-muted-foreground">-</span>
					<span class="text-muted-foreground">{fmtTime(lastResult.time)}</span>
				</div>
			</div>
		</div>
	</div>
{/if}

<FeedbackToast
	message={toastMessage}
	ok={toastOk}
	actionLabel={lastEventId && toastOk ? 'Undo' : undefined}
	onAction={handleUndo}
	onClose={() => (toastMessage = null)}
/>
