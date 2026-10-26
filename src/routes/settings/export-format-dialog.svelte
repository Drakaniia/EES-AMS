<script lang="ts">
	import Dialog from '$lib/components/ui/Dialog.svelte';

	let {
		open = $bindable(false),
		format = $bindable<'json' | 'database'>('json'),
		onexport,
		onclose
	}: {
		open?: boolean;
		format?: 'json' | 'database';
		onexport?: () => void;
		onclose?: () => void;
	} = $props();

	const options = [
		{
			value: 'json' as const,
			label: 'JSON',
			detail: 'Students, attendance records, classes and configuration.'
		},
		{
			value: 'database' as const,
			label: 'SQLite database (.db)',
			detail: 'The complete database file, openable with any SQLite tool.'
		}
	];
</script>

<Dialog
	{open}
	title="Export data"
	description="Choose a format. You pick the save location next."
	onClose={onclose}
>
	<div class="space-y-2">
		{#each options as option (option.value)}
			<label
				class="flex cursor-pointer items-start gap-3 rounded-lg border p-3 transition-colors
					{format === option.value
					? 'border-primary/45 bg-primary/[0.045]'
					: 'border-border hover:bg-surface'}"
			>
				<input
					type="radio"
					bind:group={format}
					value={option.value}
					class="mt-0.5 accent-[var(--color-primary)]"
				/>
				<span class="min-w-0">
					<span class="block text-[13px] font-medium">{option.label}</span>
					<span class="mt-0.5 block text-[11.5px] leading-4 text-muted-foreground">
						{option.detail}
					</span>
				</span>
			</label>
		{/each}
	</div>

	<div class="flex justify-end gap-2 border-t border-border pt-3">
		<button onclick={() => onclose?.()} class="btn btn-secondary">Cancel</button>
		<button onclick={onexport} class="btn btn-primary">Export</button>
	</div>
</Dialog>
