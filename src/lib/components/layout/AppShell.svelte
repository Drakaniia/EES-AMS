<script lang="ts">
	import { page } from '$app/stores';

	const navItems = [
		{ href: '/', label: 'Dashboard', icon: 'dashboard' },
		{ href: '/students', label: 'Students', icon: 'users' },
		{ href: '/attendance', label: 'Tap Mode', icon: 'scan' },
		{ href: '/records', label: 'Records', icon: 'file-text' },
		{ href: '/settings', label: 'Settings', icon: 'settings' }
	] as const;

	type NavIconName = (typeof navItems)[number]['icon'];

	const iconPaths: Record<NavIconName, string> = {
		dashboard:
			'M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z',
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

<div class="min-h-screen flex flex-col md:flex-row bg-background text-foreground">
	<!-- Sidebar -->
	<aside
		class="md:w-72 border-b md:border-b-0 md:border-r border-border bg-background md:min-h-screen flex md:flex-col"
	>
		<!-- Desktop brand -->
		<div class="hidden md:block px-6 pt-8 pb-6">
			<div class="label-mono mb-3">Horizon · Step 01</div>
			<h1 class="display-lg leading-none">
				Attendance<br />Workspace
			</h1>
			<p class="mt-4 text-sm text-muted-foreground max-w-[14rem]">
				Tap-to-attend with NFC ID cards. Local-first, always offline-ready.
			</p>
		</div>

		<!-- Mobile brand bar -->
		<div class="md:hidden flex items-center gap-3 px-4 py-3 border-b border-border w-full">
			<div
				class="size-8 rounded-md bg-primary text-primary-foreground grid place-items-center font-mono text-xs font-bold"
			>
				H
			</div>
			<div class="font-medium">Horizon Attendance</div>
		</div>

		<!-- Nav -->
		<nav class="flex md:flex-col md:gap-1 md:px-3 md:pt-2 overflow-x-auto md:overflow-visible">
			{#each navItems as item}
				{@const active = isActive(item.href, $page.url.pathname)}
				<a
					href={item.href}
					class="flex items-center gap-3 px-4 py-3 md:rounded-md text-sm whitespace-nowrap transition-colors
						{active
						? 'bg-surface text-foreground border-b-2 md:border-b-0 border-primary md:border-l-2 md:border-l-primary'
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
		</nav>

		<div class="hidden md:block mt-auto px-6 py-6 label-mono">v1 · local-only</div>
	</aside>

	<!-- Main content -->
	<main class="flex-1 min-w-0">
		{@render children()}
	</main>
</div>
