<script lang="ts">
	import { page } from '$app/state';
	import { base } from '$app/paths';
	import { updateStore } from '$lib/stores/update.svelte';
	import {
		FileSpreadsheet,
		ScanLine,
		LayoutDashboard,
		FileText,
		UsersRound,
		Search,
		Settings
	} from 'lucide-svelte';
	import { commandPaletteStore } from '$lib/stores/command-palette.svelte';

	const navItems = [
		{ href: '/reports', label: 'SF2 Reports', icon: FileSpreadsheet },
		{ href: '/attendance', label: 'Attendance', icon: ScanLine },
		{ href: '/attendance/overview', label: 'Daily Overview', icon: LayoutDashboard },
		{ href: '/attendance/logs', label: 'Attendance Logs', icon: FileText },
		{ href: '/students', label: 'Class List', icon: UsersRound },
		{ href: '/settings', label: 'Settings', icon: Settings }
	] as const;

	function isActive(href: string, pathname: string) {
		// '/attendance' is exact — its sub-routes have their own tabs.
		if (href === '/attendance') return pathname === href || pathname === '/attendance/';
		return pathname === href || pathname.startsWith(`${href}/`);
	}
</script>

<div class="title-bar" data-tauri-drag-region>
	<nav class="title-nav" aria-label="Primary navigation" data-tauri-drag-region>
		{#each navItems as item (item.href)}
			{@const active = isActive(item.href, page.url.pathname)}
			{@const Icon = item.icon}
			<a
				href={`${base}${item.href}`}
				aria-current={active ? 'page' : undefined}
				class="title-nav-link"
				class:active
			>
				<Icon class="size-[15px] shrink-0" strokeWidth={1.9} aria-hidden="true" />
				<span class="title-nav-label">{item.label}</span>
				{#if item.href === '/settings' && updateStore.badgeVisible}
					<span class="title-badge" title="Update available" aria-hidden="true"></span>
				{/if}
			</a>
		{/each}
	</nav>

	<div class="title-spacer" data-tauri-drag-region></div>

	<button
		type="button"
		class="title-search"
		title="Search commands (Ctrl+K)"
		aria-label="Search commands (Ctrl+K)"
		onclick={() => commandPaletteStore.openPalette()}
	>
		<Search class="size-3.5" strokeWidth={2} aria-hidden="true" />
		<span class="title-search-label">Search</span>
		<kbd class="title-search-kbd">Ctrl K</kbd>
	</button>
</div>

<style>
	/* The only translucent material in the product. Content scrolls beneath it;
	   nothing else earns a blur. */
	.title-bar {
		display: flex;
		align-items: center;
		height: 46px;
		padding: 0 10px;
		background: var(--chrome);
		backdrop-filter: blur(24px) saturate(180%);
		-webkit-backdrop-filter: blur(24px) saturate(180%);
		border-bottom: 1px solid var(--chrome-border);
		user-select: none;
		-webkit-app-region: drag;
		position: relative;
		z-index: 50;
		flex-shrink: 0;
	}

	@media (prefers-reduced-transparency: reduce) {
		.title-bar {
			background: var(--color-background);
			backdrop-filter: none;
			-webkit-backdrop-filter: none;
		}
	}

	/* ── Nav ──────────────────────────────────────── */
	.title-nav {
		display: flex;
		align-items: center;
		gap: 1px;
		-webkit-app-region: no-drag;
	}

	.title-nav-link {
		position: relative;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 30px;
		padding: 0 10px;
		border-radius: 7px;
		font-size: 12.5px;
		font-weight: 500;
		letter-spacing: -0.005em;
		color: var(--color-muted-foreground);
		text-decoration: none;
		white-space: nowrap;
		transition:
			background-color 140ms ease,
			color 140ms ease;
	}

	.title-nav-link:hover {
		background: color-mix(in oklab, var(--color-foreground) 5%, transparent);
		color: var(--color-foreground);
	}

	.title-nav-link.active {
		background: color-mix(in oklab, var(--color-foreground) 8%, transparent);
		color: var(--color-foreground);
		font-weight: 600;
	}

	/* ── Spacer ───────────────────────────────────── */
	.title-spacer {
		flex: 1;
		-webkit-app-region: drag;
	}

	/* ── Search ───────────────────────────────────── */
	.title-search {
		display: flex;
		align-items: center;
		gap: 7px;
		width: 196px;
		height: 30px;
		margin-right: 2px;
		border: 1px solid transparent;
		border-radius: 7px;
		background: color-mix(in oklab, var(--color-foreground) 4%, transparent);
		padding: 0 8px;
		color: var(--color-muted-foreground);
		-webkit-app-region: no-drag;
		transition:
			background-color 140ms ease,
			border-color 140ms ease,
			color 140ms ease;
	}

	.title-search:hover {
		background: color-mix(in oklab, var(--color-foreground) 7%, transparent);
		color: var(--color-foreground);
	}

	.title-search:active {
		transform: scale(0.99);
	}

	.title-search-label {
		flex: 1;
		text-align: left;
		font-size: 12.5px;
		white-space: nowrap;
	}

	.title-search-kbd {
		flex-shrink: 0;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-card);
		padding: 1px 5px;
		font-family: var(--font-sans);
		font-size: 10px;
		font-weight: 600;
		letter-spacing: 0.01em;
		color: var(--color-muted-foreground);
	}

	/* ── Update badge on nav item ─────────────── */
	.title-badge {
		position: absolute;
		top: 5px;
		right: 5px;
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--color-primary);
	}

	@media (max-width: 900px) {
		.title-nav-label {
			display: none;
		}

		.title-nav-link {
			padding: 0 8px;
		}

		.title-search {
			width: 124px;
		}
	}
</style>
