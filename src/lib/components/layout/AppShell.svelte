<script lang="ts">
	import { page } from '$app/state';
	import { base } from '$app/paths';
	import logo from '$lib/assets/logo-seal.png';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { onMount } from 'svelte';
	import {
		FileSpreadsheet,
		FileText,
		LayoutDashboard,
		ScanLine,
		Settings,
		UsersRound
	} from 'lucide-svelte';

	let { children } = $props();
	let isOnline = $state(true);

	const navItems = [
		{ href: '/', label: 'Overview', icon: LayoutDashboard },
		{ href: '/attendance', label: 'Attendance', icon: ScanLine },
		{ type: 'divider' as const },
		{ href: '/students', label: 'Class List', icon: UsersRound },
		{ href: '/records', label: 'Attendance Logs', icon: FileText },
		{ type: 'divider' as const },
		{ href: '/reports', label: 'SF2 Reports', icon: FileSpreadsheet },
		{ href: '/settings', label: 'Configuration', icon: Settings }
	] as const;

	const attendanceNavLabel = $derived(
		settingsStore.settings?.attendanceMode === 'card_reader' ? 'Live Session' : 'Attendance'
	);
	const todayLabel = $derived(
		new Intl.DateTimeFormat(undefined, {
			weekday: 'short',
			month: 'short',
			day: 'numeric'
		}).format(new Date())
	);

	onMount(() => {
		isOnline = navigator.onLine;
		const updateOnlineStatus = () => {
			isOnline = navigator.onLine;
		};

		window.addEventListener('online', updateOnlineStatus);
		window.addEventListener('offline', updateOnlineStatus);

		return () => {
			window.removeEventListener('online', updateOnlineStatus);
			window.removeEventListener('offline', updateOnlineStatus);
		};
	});

	function isActive(href: string, pathname: string) {
		return href === '/' ? pathname === '/' : pathname.startsWith(href);
	}
</script>

<div class="app-surface flex h-full min-h-0 flex-col overflow-hidden text-foreground md:flex-row">
	<a
		href="#main-content"
		class="sr-only focus:not-sr-only focus:fixed focus:top-10 focus:left-4 focus:z-[80] focus:rounded-md focus:bg-primary focus:px-4 focus:py-2 focus:text-sm focus:font-semibold focus:text-primary-foreground"
	>
		Skip to content
	</a>

	<aside
		class="sidebar flex shrink-0 flex-col border-b border-border bg-background md:min-h-0 md:w-64 md:border-r md:border-b-0"
		aria-label="Primary navigation"
	>
		<!-- Header -->
		<div class="flex items-center gap-3 px-4 py-3 md:px-4 md:pt-5 md:pb-4">
			<img
				src={logo}
				alt="Espiritu Elementary School seal"
				class="size-10 shrink-0 rounded-xl object-contain ring-1 ring-border md:size-11"
			/>
			<div class="min-w-0">
				<div class="truncate text-base leading-tight font-bold tracking-tight md:text-lg">
					EES AMS
				</div>
				<div class="mt-0.5 text-[11px] font-medium text-muted-foreground">
					{settingsStore.settings?.quarter ?? '1st Quarter'}
				</div>
			</div>
		</div>

		<!-- Navigation -->
		<nav
			class="min-w-0 flex-1 touch-pan-x overflow-x-auto px-2 pb-2 md:overflow-y-auto md:px-2 md:pb-4"
		>
			<div class="flex gap-1 md:flex-col">
				{#each navItems as item ('type' in item && item.type === 'divider' ? 'div' : (item as { href: string }).href)}
					{#if 'type' in item && item.type === 'divider'}
						<div
							class="mx-3 my-1 hidden h-px bg-border md:block"
							role="separator"
							aria-hidden="true"
						></div>
						<div class="mx-1 w-px bg-border md:hidden" role="separator" aria-hidden="true"></div>
					{:else}
						{@const navItem = item as { href: string; label: string; icon: typeof LayoutDashboard }}
						{@const active = isActive(navItem.href, page.url.pathname)}
						{@const Icon = navItem.icon}
						<a
							href={`${base}${navItem.href}`}
							aria-current={active ? 'page' : undefined}
							class="nav-link group relative flex h-9 items-center gap-3 rounded-lg px-3 text-sm font-medium whitespace-nowrap transition-all md:h-10
								{active
								? 'bg-primary/10 text-foreground'
								: 'text-muted-foreground hover:bg-surface/70 hover:text-foreground'}"
						>
							<span
								class="grid size-7 shrink-0 place-items-center rounded-lg transition-all
									{active
									? 'bg-primary text-primary-foreground shadow-sm'
									: 'text-muted-foreground group-hover:text-foreground'}"
							>
								<Icon class="size-4" aria-hidden="true" />
							</span>
							<span class="truncate">
								{navItem.href === '/attendance' ? attendanceNavLabel : navItem.label}
							</span>
						</a>
					{/if}
				{/each}
			</div>
		</nav>

		<!-- Footer -->
		<div class="hidden border-t border-border px-4 py-3 md:block">
			<div class="flex items-center justify-between gap-2">
				<div class="min-w-0">
					<div class="truncate text-xs font-semibold text-muted-foreground">{todayLabel}</div>
				</div>
				<span class="status-indicator" title={isOnline ? 'Online' : 'Offline'}>
					<span class="status-dot {isOnline ? '' : 'status-dot-muted'}" aria-hidden="true"></span>
					<span class="sr-only">{isOnline ? 'Online' : 'Offline'}</span>
				</span>
			</div>
		</div>
	</aside>

	<main
		id="main-content"
		class="min-h-0 min-w-0 flex-1 overflow-auto focus:outline-none"
		tabindex="-1"
	>
		{@render children()}
	</main>
</div>
