<script lang="ts">
	import { onMount, tick } from 'svelte';
	import {
		BookOpen,
		CornerDownLeft,
		DatabaseBackup,
		FileSpreadsheet,
		FileText,
		GraduationCap,
		LayoutDashboard,
		RefreshCw,
		ScanLine,
		Search,
		Settings,
		SlidersHorizontal,
		UserRound,
		UsersRound,
		Zap
	} from 'lucide-svelte';
	import { commandPaletteStore as store } from '$lib/stores/command-palette.svelte';
	import { filterPaletteItems, type PaletteIcon, type PaletteItem } from '$lib/command-palette';
	import { enterPanel } from '$lib/motion';

	const ICONS: Record<PaletteIcon, typeof FileText> = {
		reports: FileSpreadsheet,
		attendance: ScanLine,
		overview: LayoutDashboard,
		logs: FileText,
		students: UsersRound,
		settings: Settings,
		classes: GraduationCap,
		workbook: BookOpen,
		data: DatabaseBackup,
		general: SlidersHorizontal,
		update: RefreshCw
	};

	/** The item's own icon, else one that names its group. */
	function iconFor(item: PaletteItem) {
		if (item.icon) return ICONS[item.icon];
		if (item.group === 'Students') return UserRound;
		if (item.group === 'Actions') return Zap;
		return FileText;
	}

	let inputEl = $state<HTMLInputElement | null>(null);
	let resultsEl = $state<HTMLDivElement | null>(null);
	let panelEl = $state<HTMLDivElement | null>(null);
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
	// whatever the user was using before.
	$effect(() => {
		if (store.open) {
			previouslyFocused = document.activeElement as HTMLElement | null;
			tick().then(() => {
				inputEl?.focus();
				if (panelEl) enterPanel(panelEl, { from: { opacity: 0, scale: 0.985, y: -10 } });
			});
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
				// Never steal keys from an open modal dialog.
				const modalOpen = document.querySelector('[role="dialog"][aria-modal="true"]');
				if (store.open || !modalOpen) {
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
				bind:this={panelEl}
				class="palette-panel"
				role="dialog"
				aria-modal="true"
				aria-label="Command palette"
			>
				<div class="palette-input-row">
					<Search
						class="size-[17px] shrink-0 text-muted-foreground"
						strokeWidth={2}
						aria-hidden="true"
					/>
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
								{@const Icon = iconFor(row.item)}
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
										<Icon class="size-3.5" strokeWidth={1.9} />
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

	/* Dim the page; no blur on the scrim. One translucent material (the title
	   bar) is enough for the whole product. */
	.palette-backdrop {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		border: none;
		padding: 0;
		background: var(--scrim);
	}

	.palette-position {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding: clamp(3rem, 11vh, 6.5rem) 1rem 1rem;
		pointer-events: none;
	}

	.palette-panel {
		display: flex;
		width: min(620px, 100%);
		flex-direction: column;
		overflow: hidden;
		pointer-events: auto;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-2xl);
		background: var(--color-card);
		box-shadow: var(--shadow-lifted);
		transform-origin: top center;
	}

	/* ── Input ─────────────────────────────────────── */
	.palette-input-row {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 52px;
		padding: 0 14px;
		border-bottom: 1px solid var(--color-border);
	}

	.palette-input {
		flex: 1;
		min-width: 0;
		border: none;
		outline: none;
		background: transparent;
		font-size: 15px;
		letter-spacing: -0.01em;
		color: var(--color-foreground);
	}

	.palette-input::placeholder {
		color: var(--color-muted-foreground);
	}

	.palette-kbd {
		flex-shrink: 0;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-surface);
		padding: 1px 6px;
		font-family: var(--font-sans);
		font-size: 10px;
		font-weight: 600;
		color: var(--color-muted-foreground);
	}

	/* ── Results ───────────────────────────────────── */
	.palette-results {
		max-height: min(50vh, 400px);
		overflow-y: auto;
		padding: 6px;
	}

	.palette-group-label {
		padding: 9px 8px 4px;
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.005em;
		color: var(--color-muted-foreground);
	}

	.palette-item {
		display: flex;
		width: 100%;
		align-items: center;
		gap: 9px;
		height: 36px;
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		padding: 0 8px;
		font-size: 13.5px;
		text-align: left;
		color: var(--color-foreground);
		transition: background-color 110ms ease;
	}

	.palette-item:hover:not(.selected) {
		background: var(--color-surface);
	}

	.palette-item.selected {
		background: var(--color-fill);
	}

	.palette-item-icon {
		display: grid;
		width: 24px;
		height: 24px;
		flex-shrink: 0;
		place-items: center;
		border-radius: 6px;
		background: var(--color-surface);
		color: var(--color-muted-foreground);
	}

	.palette-item.selected .palette-item-icon {
		background: var(--color-card);
		color: var(--color-primary);
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
		font-size: 11px;
		color: var(--color-muted-foreground);
	}

	.palette-empty {
		padding: 28px 12px;
		font-size: 13px;
		text-align: center;
		color: var(--color-muted-foreground);
	}

	/* ── Footer ────────────────────────────────────── */
	.palette-footer {
		display: flex;
		gap: 16px;
		border-top: 1px solid var(--color-border);
		background: color-mix(in oklab, var(--color-surface) 55%, var(--color-card));
		padding: 8px 14px;
	}

	.palette-footer-hint {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 11px;
		color: var(--color-muted-foreground);
	}
</style>
