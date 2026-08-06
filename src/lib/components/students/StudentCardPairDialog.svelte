<script lang="ts">
	import type { Student } from '$lib/db-rust';

	type Props = {
		open: boolean;
		student: Student | null;
		cardSerial?: string;
		onSave?: () => void;
		onClose?: () => void;
	};

	let { open, student, cardSerial = $bindable(''), onSave, onClose }: Props = $props();

	let inputElement: HTMLInputElement | null = $state(null);

	$effect(() => {
		if (open && inputElement) {
			inputElement.focus();
		}
	});
</script>

{#if open}
	<div
		class="fixed inset-x-0 top-8 bottom-0 z-40 bg-black/50"
		role="presentation"
		onclick={onClose}
		onkeydown={(e) => e.key === 'Escape' && onClose?.()}
	></div>

	<div
		class="fixed inset-x-0 top-8 bottom-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="card-dialog-title"
	>
		<div class="w-full max-w-md space-y-5 rounded-2xl border border-border bg-background p-6">
			<div>
				<h2 id="card-dialog-title" class="text-lg font-semibold">Pair card</h2>
				<p class="mt-1 text-sm text-muted-foreground">
					Enter the card serial for {student?.name}.
				</p>
			</div>

			<div class="space-y-4">
				<div class="space-y-1.5">
					<label for="manual-serial" class="label-mono">Card serial</label>
					<input
						id="manual-serial"
						bind:this={inputElement}
						bind:value={cardSerial}
						placeholder="Tap card on reader or type serial…"
						autocomplete="off"
						spellcheck="false"
						class="control-ring w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm"
					/>
				</div>
			</div>

			<div class="flex justify-end gap-2">
				<button
					onclick={onClose}
					class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
				>
					Cancel
				</button>
				<button
					onclick={onSave}
					disabled={!cardSerial}
					class="rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
				>
					Save
				</button>
			</div>
		</div>
	</div>
{/if}
