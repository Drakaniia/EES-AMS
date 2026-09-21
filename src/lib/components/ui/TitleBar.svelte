<script lang="ts">
	import { page } from '$app/state';
	import { base } from '$app/paths';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { updateStore } from '$lib/stores/update.svelte';
	import defaultLogo from '$lib/assets/logo-seal.png';
	import { FileSpreadsheet, ScanLine, Search, Settings } from 'lucide-svelte';
	import { commandPaletteStore } from '$lib/stores/command-palette.svelte';

	const activeLogo = $derived.by(() => {
		const path = settingsStore.settings?.brandingLogoPath;
		if (path) return convertFileSrc(path);
		return defaultLogo;
	});

	const brandingTitle = $derived(settingsStore.settings?.brandingTitle || 'EES AMS');

	const attendanceNavLabel = $derived(
		settingsStore.settings?.attendanceMode === 'card_reader' ? 'Live Session' : 'Attendance'
	);

	const navItems = [
		{ href: '/reports', label: 'SF2 Reports', icon: FileSpreadsheet },
		{ href: '/attendance', label: 'Attendance', icon: ScanLine }
	] as const;

	function isActive(href: string, pathname: string) {
		return href === '/' ? pathname === '/' : pathname.startsWith(href);
	}
</script>

<div class="title-bar" data-tauri-drag-region>
	<!-- Logo -->
	<a
		href={`${base}/reports`}
		class="title-logo"
		title={brandingTitle}
		aria-label="Navigate to Reports"
		data-tauri-drag-region
	>
		<img src={activeLogo} alt={brandingTitle} class="title-logo-img" />
	</a>

	<!-- Nav tabs -->
	<nav class="title-nav" aria-label="Primary navigation" data-tauri-drag-region>
		{#each navItems as item (item.href)}
			{@const active = isActive(item.href, page.url.pathname)}
			{@const Icon = item.icon}
			{@const label = item.href === '/attendance' ? attendanceNavLabel : item.label}
			<a
				href={`${base}${item.href}`}
				aria-current={active ? 'page' : undefined}
				class="title-nav-link"
				class:active
			>
				<Icon class="size-[15px] shrink-0" aria-hidden="true" />
				<span class="title-nav-label">{label}</span>
				{#if active}
					<span class="title-nav-indicator" aria-hidden="true"></span>
				{/if}
			</a>
		{/each}
	</nav>

	<!-- Spacer -->
	<div class="title-spacer" data-tauri-drag-region></div>

	<!-- Command palette trigger -->
	<button
		type="button"
		class="title-search"
		title="Search commands (Ctrl+K)"
		aria-label="Search commands (Ctrl+K)"
		onclick={() => commandPaletteStore.openPalette()}
	>
		<Search class="size-3.5" aria-hidden="true" />
		<span class="title-search-label">Search</span>
		<kbd class="title-search-kbd">Ctrl K</kbd>
	</button>

	<!-- Settings gear -->
	<a
		href={`${base}/settings`}
		aria-current={page.url.pathname.startsWith('/settings') ? 'page' : undefined}
		class="title-settings"
		title="Settings"
	>
		<Settings class="size-4" aria-hidden="true" />
		{#if updateStore.badgeVisible}
			<span class="title-badge" title="Update available" aria-hidden="true"></span>
		{/if}
	</a>
</div>

<style>
	.title-bar {
		display: flex;
		align-items: center;
		height: 48px;
		padding: 0 8px 0 12px;
		background: rgba(253, 251, 249, 0.72);
		backdrop-filter: blur(20px) saturate(180%);
		-webkit-backdrop-filter: blur(20px) saturate(180%);
		border-bottom: 1px solid rgba(0, 0, 0, 0.06);
		user-select: none;
		-webkit-app-region: drag;
		position: relative;
		z-index: 50;
	}

	:global(.dark) .title-bar {
		background: rgba(18, 18, 20, 0.72);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	@media (prefers-reduced-transparency: reduce) {
		.title-bar {
			background: var(--color-background);
			backdrop-filter: none;
			-webkit-backdrop-filter: none;
		}
	}

	/* ── Logo ─────────────────────────────────────── */
	.title-logo {
		display: flex;
		align-items: center;
		flex-shrink: 0;
		-webkit-app-region: no-drag;
		border-radius: 8px;
		padding: 4px;
		transition: background-color 150ms ease;
	}
	.title-logo:hover {
		background: color-mix(in oklab, var(--color-surface) 70%, transparent);
	}
	.title-logo-img {
		width: 24px;
		height: 24px;
		border-radius: 6px;
		object-fit: contain;
		outline: 1px solid var(--color-border);
		outline-offset: 0px;
	}

	/* ── Nav ──────────────────────────────────────── */
	.title-nav {
		display: flex;
		align-items: center;
		gap: 2px;
		margin-left: 16px;
		-webkit-app-region: no-drag;
	}
	.title-nav-link {
		position: relative;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 32px;
		padding: 0 12px;
		border-radius: 8px;
		font-size: 13px;
		font-weight: 500;
		color: var(--color-muted-foreground);
		text-decoration: none;
		transition:
			background-color 150ms ease,
			color 150ms ease;
		white-space: nowrap;
	}
	.title-nav-link:hover {
		background: color-mix(in oklab, var(--color-surface) 70%, transparent);
		color: var(--color-foreground);
	}
	.title-nav-link.active {
		color: var(--color-foreground);
		font-weight: 600;
	}
	.title-nav-indicator {
		position: absolute;
		bottom: -1px;
		left: 50%;
		transform: translateX(-50%);
		width: 16px;
		height: 2px;
		border-radius: 1px;
		background: var(--color-primary);
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
		gap: 6px;
		width: 220px;
		height: 32px;
		margin-right: 4px;
		border: 1px solid var(--color-border);
		border-radius: 8px;
		background: transparent;
		padding: 0 10px;
		color: var(--color-muted-foreground);
		-webkit-app-region: no-drag;
		transition:
			background-color 150ms ease,
			border-color 150ms ease,
			color 150ms ease;
	}
	.title-search:hover {
		background: color-mix(in oklab, var(--color-surface) 70%, transparent);
		border-color: color-mix(in oklab, var(--color-primary) 38%, var(--color-border));
		color: var(--color-foreground);
	}
	.title-search-label {
		flex: 1;
		text-align: left;
		font-size: 13px;
		white-space: nowrap;
	}
	.title-search-kbd {
		flex-shrink: 0;
		border: 1px solid var(--color-border);
		border-radius: 5px;
		background: var(--surface-soft);
		padding: 1px 5px;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		color: var(--color-muted-foreground);
	}

	/* ── Settings ─────────────────────────────────── */
	.title-settings {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border-radius: 8px;
		color: var(--color-muted-foreground);
		text-decoration: none;
		-webkit-app-region: no-drag;
		transition:
			background-color 150ms ease,
			color 150ms ease;
	}
	.title-settings:hover {
		background: color-mix(in oklab, var(--color-surface) 70%, transparent);
		color: var(--color-foreground);
	}
	.title-badge {
		position: absolute;
		top: 5px;
		right: 5px;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: oklch(0.6 0.22 27);
	}
</style>
