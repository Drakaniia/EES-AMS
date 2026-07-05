<script lang="ts">
	import Dialog from '$lib/components/ui/Dialog.svelte';

	let {
		open = $bindable(false),
		quarter = $bindable('1st Quarter'),
		q1Start = $bindable(''),
		q1End = $bindable(''),
		q2Start = $bindable(''),
		q2End = $bindable(''),
		q3Start = $bindable(''),
		q3End = $bindable('')
	}: {
		open?: boolean;
		quarter?: string;
		q1Start?: string;
		q1End?: string;
		q2Start?: string;
		q2End?: string;
		q3Start?: string;
		q3End?: string;
	} = $props();
</script>

<Dialog
	{open}
	title="School Year Quarters"
	description="Set the current quarter and define the start/end dates for each period."
	onClose={() => (open = false)}
>
	<div class="space-y-6">
		<div class="space-y-2">
			<label for="currentQuarter" class="label-mono">Active Quarter</label>
			<select
				id="currentQuarter"
				bind:value={quarter}
				class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			>
				<option value="1st Quarter">1st Quarter</option>
				<option value="2nd Quarter">2nd Quarter</option>
				<option value="3rd Quarter">3rd Quarter</option>
			</select>
		</div>

		<div class="space-y-4">
			<h3 class="text-sm font-semibold tracking-wider text-muted-foreground uppercase">
				Quarter Dates
			</h3>

			<div class="grid grid-cols-2 gap-4">
				<div class="space-y-1">
					<label for="q1Start" class="text-xs font-medium text-muted-foreground">Q1 Start</label>
					<input
						id="q1Start"
						type="date"
						bind:value={q1Start}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
				<div class="space-y-1">
					<label for="q1End" class="text-xs font-medium text-muted-foreground">Q1 End</label>
					<input
						id="q1End"
						type="date"
						bind:value={q1End}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
			</div>

			<div class="grid grid-cols-2 gap-4">
				<div class="space-y-1">
					<label for="q2Start" class="text-xs font-medium text-muted-foreground">Q2 Start</label>
					<input
						id="q2Start"
						type="date"
						bind:value={q2Start}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
				<div class="space-y-1">
					<label for="q2End" class="text-xs font-medium text-muted-foreground">Q2 End</label>
					<input
						id="q2End"
						type="date"
						bind:value={q2End}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
			</div>

			<div class="grid grid-cols-2 gap-4">
				<div class="space-y-1">
					<label for="q3Start" class="text-xs font-medium text-muted-foreground">Q3 Start</label>
					<input
						id="q3Start"
						type="date"
						bind:value={q3Start}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
				<div class="space-y-1">
					<label for="q3End" class="text-xs font-medium text-muted-foreground">Q3 End</label>
					<input
						id="q3End"
						type="date"
						bind:value={q3End}
						class="h-9 w-full rounded-md border border-border bg-background px-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
			</div>
		</div>

		<button
			onclick={() => (open = false)}
			class="w-full rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
		>
			Done
		</button>
	</div>
</Dialog>
