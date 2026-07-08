<script lang="ts">
	import { ScanLine } from 'lucide-svelte';

	import { fmtTime } from '$lib/csv';
	import type { AttendanceType } from '$lib/db-rust';
	import type { LogLine } from './attendance-state.svelte';

	let {
		classes,
		sessionClass,
		isProcessing,
		dateLoading,
		cardInput,
		cardInputElement = $bindable(),
		log,
		onCardInputChange,
		onCardSubmit,
		pickerOpen,
		onPickerOpen
	}: {
		classes: { id: string; name: string }[];
		sessionClass: { id: string; name: string; dayStart: string; dayEnd: string } | undefined;
		isProcessing: boolean;
		dateLoading: boolean;
		cardInput: string;
		cardInputElement: HTMLInputElement | null;
		log: LogLine[];
		onCardInputChange: (value: string) => void;
		onCardSubmit: (serial: string) => void;
		pickerOpen: boolean;
		onPickerOpen: () => void;
	} = $props();
</script>

<div
	class="relative flex min-h-[30rem] items-center justify-center overflow-hidden rounded-2xl border border-border bg-surface p-6 md:p-8"
>
	{#if classes.length === 0}
		<div class="relative w-full max-w-md text-center">
			<h3 class="display-lg mb-2">No Classes</h3>
			<p class="mb-8 text-muted-foreground">
				Add a class in Settings before starting a live session.
			</p>
		</div>
	{:else if !sessionClass}
		<div class="relative w-full max-w-md text-center">
			<h3 class="display-lg mb-2">No active class</h3>
			<p class="mb-8 text-muted-foreground">
				Check the assigned class schedule in Settings before starting a live session.
			</p>
		</div>
	{:else}
		<div class="relative w-full max-w-md text-center" role="status" aria-live="polite">
			<div class="label-mono mb-4 text-primary">
				<span class="inline-block size-2 rounded-full bg-primary align-middle"></span> Ready for card
				taps
			</div>

			<div
				class="mx-auto grid size-36 place-items-center rounded-full border-2 border-primary bg-background"
			>
				<ScanLine class="size-16 text-primary" strokeWidth={1.5} />
			</div>

			<h3 class="mt-8 text-4xl font-semibold tracking-normal">Tap a card</h3>
			<p class="mx-auto mt-2 max-w-sm text-sm text-muted-foreground">
				The reader field stays focused for card serials and typed fallback entries.
			</p>
			{#if isProcessing}
				<p class="mt-3 text-sm font-medium text-primary" aria-live="assertive">
					Processing card tap...
				</p>
			{/if}

			<form
				onsubmit={(e) => {
					e.preventDefault();
					onCardSubmit(cardInput);
				}}
				class="mx-auto mt-6 max-w-sm"
			>
				<label for="card-reader-serial" class="sr-only">Card serial</label>
				<input
					id="card-reader-serial"
					bind:this={cardInputElement}
					type="text"
					value={cardInput}
					oninput={(e) => onCardInputChange((e.currentTarget as HTMLInputElement).value)}
					placeholder="Tap card or enter serial..."
					autocomplete="off"
					spellcheck="false"
					aria-describedby="card-reader-help"
					disabled={isProcessing || dateLoading}
					class="control-ring h-12 w-full rounded-md border border-border bg-background px-4 text-center font-mono text-sm disabled:cursor-wait disabled:opacity-70"
				/>
				<p id="card-reader-help" class="mt-2 text-xs text-muted-foreground">
					Press Enter after typing a serial manually.
				</p>
			</form>
		</div>
	{/if}
</div>

<div class="flex min-h-[30rem] flex-col rounded-2xl border border-border bg-card p-5">
	<div class="mb-4 flex shrink-0 items-start justify-between gap-3">
		<div class="flex flex-col">
			<h3 class="text-lg font-medium">Session log</h3>
			<span class="label-mono text-xs opacity-60">Latest card or manual actions</span>
		</div>
		<span class="label-mono rounded-pill border border-border bg-surface px-2 py-1 text-[10px]">
			{log.length} entries
		</span>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto">
		{#if log.length === 0}
			<div
				class="flex h-full w-full flex-col items-center justify-center rounded-xl border border-dashed border-border p-4 text-center text-sm text-muted-foreground"
			>
				No activity recorded in this session.
			</div>
		{:else}
			<ul class="divide-y divide-border">
				{#each log as line (line.id)}
					<li class="flex items-center justify-between gap-3 py-3">
						<div class="min-w-0 flex-1">
							<div class="leading-snug font-medium break-words">{line.studentName}</div>
							<div class="label-mono">{fmtTime(line.timestamp)}</div>
						</div>
						<div class="flex items-center gap-2">
							{#if line.isLate}
								<span
									class="rounded-pill border border-destructive/20 bg-destructive/10 px-2 py-0.5 font-mono text-[10px] font-bold text-destructive"
								>
									LATE
								</span>
							{/if}
							{@render pill(line.type)}
						</div>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>

{#snippet pill(type: AttendanceType | 'error')}
	<span
		class="shrink-0 rounded-pill px-2 py-1 font-mono text-[10px] font-bold
			{type === 'in'
			? 'bg-primary text-primary-foreground'
			: 'bg-destructive text-destructive-foreground'}"
	>
		{type === 'in' ? 'IN' : 'ERROR'}
	</span>
{/snippet}
