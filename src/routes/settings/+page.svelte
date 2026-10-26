<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import {
		BookOpen,
		DatabaseBackup,
		GraduationCap,
		RefreshCw,
		SlidersHorizontal
	} from 'lucide-svelte';
	import { settingsState } from './settings-state.svelte';

	import ClassesSection from './classes-section.svelte';
	import BackupSection from './backup-section.svelte';
	import Sf2Section from './sf2-section.svelte';
	import GlobalConfigForm from './global-config-form.svelte';
	import UpdateSection from './update-section.svelte';
	import RestoreBackupDialog from './restore-backup-dialog.svelte';

	/** The pane ids are also the deep-link hashes the command palette uses. */
	type SectionId =
		| 'settings-general'
		| 'settings-classes'
		| 'settings-sf2'
		| 'settings-backup'
		| 'settings-update';

	const sections: { id: SectionId; label: string; icon: typeof SlidersHorizontal }[] = [
		{ id: 'settings-general', label: 'General', icon: SlidersHorizontal },
		{ id: 'settings-classes', label: 'Class', icon: GraduationCap },
		{ id: 'settings-sf2', label: 'SF2 Workbook', icon: BookOpen },
		{ id: 'settings-backup', label: 'Data & Backup', icon: DatabaseBackup },
		{ id: 'settings-update', label: 'Software Update', icon: RefreshCw }
	];

	let activeId = $state<SectionId>('settings-general');

	onMount(() => {
		settingsState.init();
	});

	// Deep links from the command palette (#settings-<section>). Selecting a
	// pane in the sidebar does not touch the URL, so this only reacts to a real
	// navigation.
	$effect(() => {
		const id = decodeURIComponent(page.url.hash.slice(1));
		if (sections.some((section) => section.id === id)) activeId = id as SectionId;
	});
</script>

<svelte:head>
	<title>Settings — Attendance System</title>
	<meta name="description" content="Manage your classes and system configuration." />
</svelte:head>

<div class="settings-shell">
	<aside class="settings-sidebar">
		<h1 class="settings-sidebar-title">Settings</h1>
		<nav class="settings-nav" aria-label="Settings sections">
			{#each sections as section (section.id)}
				{@const Icon = section.icon}
				<button
					type="button"
					class="settings-nav-item"
					class:active={activeId === section.id}
					aria-current={activeId === section.id ? 'page' : undefined}
					onclick={() => (activeId = section.id)}
				>
					<Icon class="size-[15px] shrink-0" strokeWidth={1.9} aria-hidden="true" />
					<span class="truncate">{section.label}</span>
				</button>
			{/each}
		</nav>
	</aside>

	<div class="settings-body">
		<div class="settings-content">
			{#if settingsStore.loading}
				<div class="space-y-3" role="status" aria-live="polite">
					<div class="skeleton h-5 w-36 rounded"></div>
					<div class="skeleton h-3.5 w-72 rounded"></div>
					<div class="skeleton mt-5 h-40 w-full rounded-xl"></div>
				</div>
			{:else if settingsStore.error}
				<div
					class="rounded-xl border border-destructive/25 bg-destructive/[0.04] p-4 text-[13px] text-destructive"
				>
					{settingsStore.error}
					<button onclick={() => settingsState.reload()} class="btn btn-secondary btn-sm ml-2">
						Retry
					</button>
				</div>
			{:else if activeId === 'settings-classes'}
				<ClassesSection />
			{:else if activeId === 'settings-sf2'}
				<Sf2Section onOpenData={() => (activeId = 'settings-backup')} />
			{:else if activeId === 'settings-backup'}
				<BackupSection />
			{:else if activeId === 'settings-update'}
				<UpdateSection />
			{:else}
				<GlobalConfigForm />
			{/if}
		</div>
	</div>
</div>

<RestoreBackupDialog />
<FeedbackToast
	message={settingsState.toastMessage}
	ok={settingsState.toastOk}
	onClose={() => (settingsState.toastMessage = null)}
/>

<style>
	.settings-shell {
		display: flex;
		height: 100%;
		min-height: 0;
		align-items: stretch;
	}

	/* ── Sidebar ──────────────────────────────────── */
	.settings-sidebar {
		display: flex;
		width: 218px;
		flex-shrink: 0;
		flex-direction: column;
		gap: 2px;
		overflow-y: auto;
		border-right: 1px solid var(--color-border);
		background: var(--color-background);
		padding: 16px 10px;
	}

	.settings-sidebar-title {
		padding: 2px 8px 12px;
		font-size: 17px;
		font-weight: 600;
		letter-spacing: -0.02em;
	}

	.settings-nav {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.settings-nav-item {
		display: flex;
		width: 100%;
		height: 30px;
		align-items: center;
		gap: 8px;
		border: none;
		border-radius: 7px;
		background: transparent;
		padding: 0 8px;
		font-size: 12.5px;
		font-weight: 500;
		text-align: left;
		color: var(--color-foreground);
		transition:
			background-color 120ms ease,
			color 120ms ease;
	}

	.settings-nav-item :global(svg) {
		color: var(--color-muted-foreground);
	}

	.settings-nav-item:hover:not(.active) {
		background: var(--color-surface);
	}

	.settings-nav-item.active {
		background: color-mix(in oklab, var(--color-primary) 10%, transparent);
		color: color-mix(in oklab, var(--color-primary) 78%, black 12%);
		font-weight: 600;
	}

	:global(.dark) .settings-nav-item.active {
		color: var(--color-primary);
	}

	.settings-nav-item.active :global(svg) {
		color: var(--color-primary);
	}

	.settings-nav-item:focus-visible {
		outline: none;
		box-shadow:
			0 0 0 2px var(--color-background),
			0 0 0 4px color-mix(in oklab, var(--color-ring) 52%, transparent);
	}

	/* ── Pane ─────────────────────────────────────── */
	.settings-body {
		min-width: 0;
		min-height: 0;
		flex: 1;
		overflow-y: auto;
	}

	.settings-content {
		max-width: 720px;
		margin-inline: auto;
		padding: 28px 24px 72px;
	}

	@media (min-width: 1024px) {
		.settings-content {
			padding: 36px 32px 88px;
		}
	}

	/* Narrow windows: the sidebar folds into a scrolling tab strip. */
	@media (max-width: 767px) {
		.settings-shell {
			flex-direction: column;
		}

		.settings-sidebar {
			width: auto;
			flex-direction: row;
			align-items: center;
			gap: 4px;
			overflow-x: auto;
			overflow-y: hidden;
			border-right: none;
			border-bottom: 1px solid var(--color-border);
			padding: 8px;
		}

		.settings-sidebar-title {
			display: none;
		}

		.settings-nav {
			flex-direction: row;
		}

		.settings-nav-item {
			width: auto;
			white-space: nowrap;
		}
	}
</style>
