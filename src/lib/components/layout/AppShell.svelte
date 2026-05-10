<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/stores';
	import logo from '$lib/assets/logo-seal.png';

	const navGroups = [
		{
			title: 'Main',
			items: [
				{ href: '/', label: 'Overview', icon: 'dashboard' },
				{ href: '/attendance', label: 'Live Session', icon: 'scan' }
			]
		},
		{
			title: 'Management',
			items: [
				{ href: '/students', label: 'Student Roster', icon: 'users' },
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

	function isActive(href: string, pathname: string) {
		return href === '/' ? pathname === '/' : pathname.startsWith(href);
	}
</script>

<div class="bg-background text-foreground flex min-h-screen flex-col md:flex-row">
	<!-- Sidebar -->
	<aside
		class="border-border bg-background flex border-b md:min-h-screen md:w-72 md:flex-col md:border-r md:border-b-0"
	>
		<!-- Desktop brand -->
		<div class="hidden px-6 pt-8 pb-6 md:block">
			<div class="flex items-center gap-3">
				<img src={logo} alt="School Logo" class="size-10 object-contain" />
				<h1 class="text-xl font-bold tracking-tight">
					Attendance<br />System
				</h1>
			</div>
			<p class="text-muted-foreground mt-4 max-w-[14rem] text-sm">
				Manage student attendance with ease.
			</p>
		</div>

		<!-- Mobile brand bar -->
		<div class="border-border flex w-full items-center gap-3 border-b px-4 py-3 md:hidden">
			<img src={logo} alt="School Logo" class="size-8 object-contain" />
			<div class="font-medium">Attendance System</div>
		</div>

		<!-- Nav -->
		<nav class="flex overflow-x-auto md:flex-col md:gap-6 md:overflow-visible md:px-3 md:pt-4">
			{#each navGroups as group}
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
									? 'bg-surface text-foreground border-primary md:border-l-primary border-b-2 md:border-b-0 md:border-l-2'
									: 'text-muted-foreground hover:text-foreground hover:bg-surface/60'}"
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
								<span>{item.label}</span>
							</a>
						{/each}
					</div>
				</div>
			{/each}
		</nav>
	</aside>

	<!-- Main content -->
	<main class="min-w-0 flex-1">
		{@render children()}
	</main>
</div>
