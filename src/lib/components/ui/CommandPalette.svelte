<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { CornerDownLeft, FileText, Search, UserRound, Zap } from 'lucide-svelte';
	import { commandPaletteStore as store } from '$lib/stores/command-palette.svelte';
	import { filterPaletteItems, type PaletteItem } from '$lib/command-palette';

	let inputEl = $state<HTMLInputElement | null>(null);
	let resultsEl = $state<HTMLDivElement | null>(null);
	let previouslyFocused: HTMLElement | null = null;

	type Row =
		| { kind: 'group'; label: string; key: string }
		| { kind: 'item'; item: PaletteItem; key: string; index: number };

	const groups = $derived(filterPaletteItems(store.allItems(), store.query));

	const rows = $derived.by(() => {
		const out: Row[] = [];
		let index = 0;
		for (const group of groups) {
			out.push({ kind: 'group', label: group.group, key: `group-${group.group}` });
			for (const item of group.items) {
				out.push({ kind: 'item', item, key: item.id, index });
				index++;
			}
		}
		return out;
	});

	const itemRows = $derived(
		rows.filter((row): row is Extract<Row, { kind: 'item' }> => row.kind === 'item')
	);
	const selectedItem = $derived(itemRows[store.selectedIndex]?.item);

	// Keep the cursor in range and reset it whenever the query or result set
	// changes.
	$effect(() => {
		void store.query;
		void rows.length;
		store.selectedIndex = 0;
	});

	// Focus management: the input takes focus on open, and focus returns to
	// whatever the user was using before (e.g. the card reader input).
	$effect(() => {
		if (store.open) {
			previouslyFocused = document.activeElement as HTMLElement | null;
			tick().then(() => inputEl?.focus());
		} else {
			if (previouslyFocused && previouslyFocused.isConnected) previouslyFocused.focus();
			previouslyFocused = null;
		}
	});

	function scrollSelectedIntoView() {
		tick().then(() => {
			resultsEl?.querySelector('[data-selected="true"]')?.scrollIntoView({ block: 'nearest' });
		});
	}

	function handleInputKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			store.selectedIndex = Math.min(store.selectedIndex + 1, itemRows.length - 1);
			scrollSelectedIntoView();
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			store.selectedIndex = Math.max(store.selectedIndex - 1, 0);
			scrollSelectedIntoView();
		} else if (event.key === 'Enter') {
			event.preventDefault();
			if (selectedItem) store.run(selectedItem);
		} else if (event.key === 'Escape') {
			event.preventDefault();
			store.closePalette();
		}
	}

	onMount(() => {
		const onWindowKeydown = (event: KeyboardEvent) => {
			if (event.key === 'Escape' && store.open) {
				event.preventDefault();
				store.closePalette();
				return;
			}
			if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
				// Never steal keys from an open modal dialog, and never open while
				// the card-reader wedge is armed (raw scans would hit the query box).
				const modalOpen = document.querySelector('[role="dialog"][aria-modal="true"]');
				if (store.open || (!modalOpen && !store.cardReaderArmed)) {
					event.preventDefault();
					if (store.open) {
						store.closePalette();
					} else {
						store.openPalette();
					}
				}
			}
		};
		window.addEventListener('keydown', onWindowKeydown);
		return () => window.removeEventListener('keydown', onWindowKeydown);
	});
</script>

{#if store.open}
	<div class="palette-layer" role="presentation">
		<button
			type="button"
			class="palette-backdrop"
			aria-label="Close command palette"
			onclick={() => store.closePalette()}
		></button>
		<div class="palette-position">
			<div
				class="surface-panel palette-panel"
				role="dialog"
				aria-modal="true"
				aria-label="Command palette"
			>
				<div class="palette-input-row">
					<Search class="size-[18px] shrink-0 text-muted-foreground" aria-hidden="true" />
					<input
						bind:this={inputEl}
						bind:value={store.query}
						class="palette-input"
						placeholder="Search pages, actions, students…"
						aria-label="Search commands"
						autocomplete="off"
						spellcheck="false"
						onkeydown={handleInputKeydown}
					/>
					<kbd class="palette-kbd">esc</kbd>
				</div>

				<div bind:this={resultsEl} class="palette-results" role="listbox" aria-label="Results">
					{#if rows.length === 0}
						<div class="palette-empty">No results for “{store.query}”</div>
					{:else}
						{#each rows as row (row.key)}
							{#if row.kind === 'group'}
								<div class="palette-group-label">{row.label}</div>
							{:else}
								<button
									type="button"
									role="option"
									aria-selected={row.index === store.selectedIndex}
									class="palette-item"
									class:selected={row.index === store.selectedIndex}
									data-selected={row.index === store.selectedIndex}
									onmouseenter={() => (store.selectedIndex = row.index)}
									onclick={() => store.run(row.item)}
								>
									<span class="palette-item-icon" aria-hidden="true">
										{#if row.item.group === 'Students'}
											<UserRound class="size-3.5" />
										{:else if row.item.group === 'Actions'}
											<Zap class="size-3.5" />
										{:else}
											<FileText class="size-3.5" />
										{/if}
									</span>
									<span class="palette-item-label">{row.item.label}</span>
									<span class="palette-item-hint">{row.item.hint}</span>
								</button>
							{/if}
						{/each}
					{/if}
				</div>

				<div class="palette-footer">
					<span class="palette-footer-hint">
						<CornerDownLeft class="size-3" aria-hidden="true" />
						to open
					</span>
					<span class="palette-footer-hint">↑↓ navigate</span>
					<span class="palette-footer-hint">esc close</span>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.palette-layer {
		position: fixed;
		inset: 0;
		z-index: 80;
	}

	.palette-backdrop {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		border: none;
		padding: 0;
		background: color-mix(in oklab, var(--color-foreground) 38%, transparent);
		backdrop-filter: blur(2px);
		-webkit-backdrop-filter: blur(2px);
	}

	.palette-position {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding: clamp(3.5rem, 12vh, 7rem) 1rem 1rem;
		pointer-events: none;
	}

	.palette-panel {
		display: flex;
		width: min(480px, 100%);
		flex-direction: column;
		overflow: hidden;
		pointer-events: auto;
		box-shadow: var(--shadow-soft);
	}

	/* ── Input ─────────────────────────────────────── */
	.palette-input-row {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 0.875rem 1rem;
		border-bottom: 1px solid var(--color-border);
	}

	.palette-input {
		flex: 1;
		min-width: 0;
		border: none;
		outline: none;
		background: transparent;
		font-size: 15px;
		color: var(--color-foreground);
	}

	.palette-input::placeholder {
		color: var(--color-muted-foreground);
	}

	.palette-kbd {
		flex-shrink: 0;
		border: 1px solid var(--color-border);
		border-radius: 6px;
		background: var(--surface-soft);
		padding: 2px 7px;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		color: var(--color-muted-foreground);
		text-transform: uppercase;
	}

	/* ── Results ───────────────────────────────────── */
	.palette-results {
		max-height: min(52vh, 420px);
		overflow-y: auto;
		padding: 0.5rem;
	}

	.palette-group-label {
		padding: 0.5rem 0.625rem 0.3rem;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 700;
		letter-spacing: 0.06em;
		color: var(--color-muted-foreground);
		text-transform: uppercase;
	}

	.palette-item {
		display: flex;
		width: 100%;
		align-items: center;
		gap: 0.625rem;
		border: none;
		border-radius: var(--radius-lg);
		background: transparent;
		padding: 0.5rem 0.625rem;
		font-size: 14px;
		text-align: left;
		color: var(--color-foreground);
	}

	.palette-item:hover:not(.selected) {
		background: var(--surface-soft);
	}

	.palette-item.selected {
		background: color-mix(in oklab, var(--color-primary) 13%, transparent);
	}

	.palette-item-icon {
		display: grid;
		width: 26px;
		height: 26px;
		flex-shrink: 0;
		place-items: center;
		border-radius: 8px;
		background: var(--surface-soft);
		color: var(--color-muted-foreground);
	}

	.palette-item-label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.palette-item-hint {
		flex-shrink: 0;
		font-family: var(--font-mono);
		font-size: 11px;
		color: var(--color-muted-foreground);
	}

	.palette-empty {
		padding: 1.5rem 0.75rem;
		font-size: 14px;
		text-align: center;
		color: var(--color-muted-foreground);
	}

	/* ── Footer ────────────────────────────────────── */
	.palette-footer {
		display: flex;
		gap: 1.1rem;
		border-top: 1px solid var(--color-border);
		padding: 0.625rem 1rem;
	}

	.palette-footer-hint {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-family: var(--font-mono);
		font-size: 11px;
		color: var(--color-muted-foreground);
	}
</style>
