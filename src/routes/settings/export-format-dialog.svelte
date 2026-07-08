<script lang="ts">
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
</script>

{#if open}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => onclose?.()}
		onkeydown={(e) => e.key === 'Escape' && onclose?.()}
	></div>

	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="export-dialog-title"
	>
		<div class="w-full max-w-md space-y-5 rounded-2xl border border-border bg-background p-6">
			<div>
				<h2 id="export-dialog-title" class="text-lg font-semibold">Export Data</h2>
				<p class="mt-1 text-sm text-muted-foreground">
					Choose the format for your data export. You'll be able to select the save location.
				</p>
			</div>

			<div class="space-y-3">
				<label class="flex cursor-pointer items-center gap-3">
					<input
						type="radio"
						bind:group={format}
						value="json"
						class="text-primary focus:ring-primary"
					/>
					<div>
						<div class="font-medium">JSON Format</div>
						<div class="text-sm text-muted-foreground">
							Includes students, attendance records, classes, and system configuration
						</div>
					</div>
				</label>

				<label class="flex cursor-pointer items-center gap-3">
					<input
						type="radio"
						bind:group={format}
						value="database"
						class="text-primary focus:ring-primary"
					/>
					<div>
						<div class="font-medium">SQLite Database (.db)</div>
						<div class="text-sm text-muted-foreground">
							Complete database file, can be opened with SQLite tools
						</div>
					</div>
				</label>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => onclose?.()}
					class="flex-1 rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
				>
					Cancel
				</button>
				<button
					onclick={onexport}
					class="flex-1 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-white hover:opacity-90"
				>
					Export
				</button>
			</div>
		</div>
	</div>
{/if}
