<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/stores';
	import logo from '$lib/assets/logo-seal.png';
	import { settingsStore } from '$lib/stores/settings.svelte';

	const navGroups = [
		{
			title: 'Main',
			items: [
				{ href: '/', label: 'Overview', icon: 'dashboard' },
				{ href: '/attendance', label: 'Attendance', icon: 'scan' }
			]
		},
		{
			title: 'Management',
			items: [
				{ href: '/students', label: 'Class List', icon: 'users' },
				{ href: '/records', label: 'Attendance Logs', icon: 'file-text' }
			]
		},
		{
			title: 'System',
			items: [{ href: '/settings', label: 'Configuration', icon: 'settings' }]
		}
	] as const;

	type NavIconName = (typeof navGroups)[number]['items'][number]['icon'];

	const iconPaths: Record<NavIconName, string> = {
		dashboard: 'M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z',
		users:
			'M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75',
		scan: 'M3 7V5a2 2 0 0 1 2-2h2M17 3h2a2 2 0 0 1 2 2v2M21 17v2a2 2 0 0 1-2 2h-2M7 21H5a2 2 0 0 1-2-2v-2M7 12h10',
		'file-text':
			'M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zM14 2v6h6M16 13H8M16 17H8M10 9H9H8',
		settings:
			'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z'
	};

	let { children } = $props();
	const attendanceNavLabel = $derived(
		settingsStore.settings?.attendanceMode === 'card_reader' ? 'Live Session' : 'Attendance'
	);

	function isActive(href: string, pathname: string) {
		return href === '/' ? pathname === '/' : pathname.startsWith(href);
	}

	function getSchoolYear() {
		const now = new Date();
		const year = now.getFullYear();
		const month = now.getMonth(); // 0-indexed, 0 = Jan, 7 = Aug
		if (month < 7) {
			// Before August, we are in the second half of the school year
			return `${year - 1}–${year}`;
		} else {
			// From August onwards, we are in the first half of a new school year
			return `${year}–${year + 1}`;
		}
	}
</script>

<div class="flex h-screen flex-col overflow-hidden bg-background text-foreground md:flex-row">
	<!-- Sidebar -->
	<aside
		class="flex border-b border-border bg-background md:min-h-screen md:w-72 md:flex-col md:border-r md:border-b-0"
	>
		<!-- Desktop brand -->
		<div class="hidden px-5 pt-6 pb-6 md:block">
			<div class="flex items-center gap-3">
				<img src={logo} alt="School Logo" class="size-16 shrink-0 object-contain" />
				<div class="flex flex-col justify-center gap-0.5">
					<h1 class="text-2xl leading-none font-black tracking-tight whitespace-nowrap uppercase">
						EES AMS
					</h1>
					<div class="flex flex-col text-xs leading-snug font-medium text-muted-foreground">
						<span>{getSchoolYear()}</span>
						<span>{settingsStore.settings?.quarter ?? '1st Quarter'}</span>
					</div>
				</div>
			</div>
		</div>

		<!-- Mobile brand bar -->
		<div class="flex w-full items-center gap-3 border-b border-border px-4 py-3 md:hidden">
			<img src={logo} alt="School Logo" class="size-16 object-contain" />
			<div class="flex flex-col justify-center leading-tight">
				<div class="text-lg font-black tracking-tight uppercase">EES AMS</div>
				<div class="flex flex-col text-xs font-medium text-muted-foreground">
					<span>{getSchoolYear()}</span>
					<span>{settingsStore.settings?.quarter ?? '1st Quarter'}</span>
				</div>
			</div>
		</div>

		<!-- Nav -->
		<nav class="flex overflow-x-auto md:flex-col md:gap-6 md:overflow-visible md:px-3 md:pt-4">
			{#each navGroups as group (group.title)}
				<div class="flex flex-row md:flex-col md:gap-1">
					<div class="label-mono hidden px-3 pb-2 text-[10px] md:block">{group.title}</div>
					<div class="flex flex-row md:flex-col md:gap-1">
						{#each group.items as item (item.href)}
							{@const active = isActive(item.href, $page.url.pathname)}
							<a
								href={resolve(item.href)}
								onclick={(e) => {
									e.preventDefault();
									goto(resolve(item.href));
								}}
								class="flex items-center gap-3 px-4 py-3 text-sm whitespace-nowrap transition-colors md:rounded-md
									{active
									? 'border-b-2 border-primary bg-surface text-foreground md:border-b-0 md:border-l-2 md:border-l-primary'
									: 'text-muted-foreground hover:bg-surface/60 hover:text-foreground'}"
							>
								<svg
									class="size-4 shrink-0"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									aria-hidden="true"
								>
									<path d={iconPaths[item.icon]} />
								</svg>
								<span>{item.href === '/attendance' ? attendanceNavLabel : item.label}</span>
							</a>
						{/each}
					</div>
				</div>
			{/each}
		</nav>
	</aside>

	<!-- Main content -->
	<main class="min-w-0 flex-1 overflow-auto">
		{@render children()}
	</main>
</div>
