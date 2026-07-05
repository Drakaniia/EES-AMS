<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
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

	const navGroups = [
		{
			title: 'Main',
			items: [
				{ href: '/', label: 'Overview', icon: LayoutDashboard },
				{ href: '/attendance', label: 'Attendance', icon: ScanLine }
			]
		},
		{
			title: 'Management',
			items: [
				{ href: '/students', label: 'Class List', icon: UsersRound },
				{ href: '/records', label: 'Attendance Logs', icon: FileText }
			]
		},
		{
			title: 'System',
			items: [
				{ href: '/reports', label: 'SF2 Reports', icon: FileSpreadsheet },
				{ href: '/settings', label: 'Configuration', icon: Settings }
			]
		}
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

	function getSchoolYear() {
		const now = new Date();
		const year = now.getFullYear();
		const month = now.getMonth();
		return month < 7 ? `${year - 1}-${year}` : `${year}-${year + 1}`;
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
		class="flex shrink-0 flex-col border-b border-border bg-background md:min-h-0 md:w-72 md:border-r md:border-b-0"
		aria-label="Primary navigation"
	>
		<div class="flex items-center gap-3 px-4 py-3 md:px-5 md:pt-6 md:pb-5">
			<img
				src={logo}
				alt="Espiritu Elementary School seal"
				class="size-12 shrink-0 rounded-xl object-contain ring-1 ring-border md:size-14"
			/>
			<div class="min-w-0">
				<div class="truncate text-lg leading-none font-black tracking-normal uppercase md:text-2xl">
					EES AMS
				</div>
				<div
					class="mt-1 flex flex-wrap gap-x-2 gap-y-0.5 text-xs font-medium text-muted-foreground"
				>
					<span>{getSchoolYear()}</span>
					<span aria-hidden="true">/</span>
					<span>{settingsStore.settings?.quarter ?? '1st Quarter'}</span>
				</div>
			</div>
			<div class="ml-auto md:hidden">
				<span class="chip">
					<span class="status-dot {isOnline ? '' : 'status-dot-muted'}" aria-hidden="true"></span>
					{isOnline ? 'Local' : 'Offline'}
				</span>
			</div>
		</div>

		<nav
			class="min-w-0 touch-pan-x overflow-x-auto px-2 pb-2 md:flex-1 md:overflow-y-auto md:px-3 md:pb-5"
		>
			<div class="flex gap-2 md:flex-col md:gap-5">
				{#each navGroups as group (group.title)}
					<section class="flex shrink-0 md:block" aria-labelledby={`nav-${group.title}`}>
						<h2 id={`nav-${group.title}`} class="label-mono hidden px-3 pb-2 text-[10px] md:block">
							{group.title}
						</h2>
						<div class="flex gap-1 md:flex-col">
							{#each group.items as item (item.href)}
								{@const active = isActive(item.href, page.url.pathname)}
								{@const Icon = item.icon}
								<a
									href={resolve(item.href)}
									aria-current={active ? 'page' : undefined}
									class="control-ring group relative flex h-10 min-w-max items-center gap-2 rounded-xl border px-3 text-sm font-semibold whitespace-nowrap md:h-11 md:min-w-0 md:gap-3 md:px-3.5
										{active
										? 'border-primary/35 bg-primary/10 text-foreground shadow-sm'
										: 'border-transparent text-muted-foreground hover:bg-surface/80 hover:text-foreground'}"
								>
									{#if active}
										<span
											class="absolute top-1/2 left-1 hidden h-6 w-1 -translate-y-1/2 rounded-pill bg-primary md:block"
											aria-hidden="true"
										></span>
									{/if}
									<span
										class="grid size-7 shrink-0 place-items-center rounded-lg transition-colors {active
											? 'bg-primary text-primary-foreground'
											: 'bg-surface text-muted-foreground group-hover:text-foreground'}"
									>
										<Icon class="size-4" aria-hidden="true" />
									</span>
									<span class="truncate">
										{item.href === '/attendance' ? attendanceNavLabel : item.label}
									</span>
								</a>
							{/each}
						</div>
					</section>
				{/each}
			</div>
		</nav>

		<div class="hidden border-t border-border px-5 py-4 md:block">
			<div class="surface-panel p-3">
				<div class="flex items-center justify-between gap-3">
					<div class="min-w-0">
						<div class="truncate text-sm font-semibold">{todayLabel}</div>
						<div class="mt-1 text-xs text-muted-foreground">
							{settingsStore.loading ? 'Syncing settings' : 'Local attendance database'}
						</div>
					</div>
					<span class="chip shrink-0">
						<span class="status-dot {isOnline ? '' : 'status-dot-muted'}" aria-hidden="true"></span>
						{isOnline ? 'Online' : 'Offline'}
					</span>
				</div>
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
