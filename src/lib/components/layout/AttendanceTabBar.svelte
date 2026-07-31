<script lang="ts">
	import { page } from '$app/state';
	import { base } from '$app/paths';
	import { ScanLine, LayoutDashboard, FileText, UsersRound } from 'lucide-svelte';

	const tabs = [
		{ href: '/attendance', label: 'Attendance', icon: ScanLine },
		{ href: '/attendance/overview', label: 'Daily Overview', icon: LayoutDashboard },
		{ href: '/attendance/logs', label: 'Attendance Logs', icon: FileText },
		{ href: '/students', label: 'Class List', icon: UsersRound }
	] as const;

	function isActive(href: string, pathname: string) {
		return href === '/attendance'
			? pathname === href || pathname === '/attendance/'
			: pathname.startsWith(href);
	}
</script>

<nav
	class="tab-bar flex items-center gap-0.5 overflow-x-auto border-b border-border bg-background px-4 py-0 md:px-8 lg:px-10"
	aria-label="Attendance navigation"
>
	{#each tabs as tab (tab.href)}
		{@const active = isActive(tab.href, page.url.pathname)}
		{@const Icon = tab.icon}
		<a
			href={`${base}${tab.href}`}
			aria-current={active ? 'page' : undefined}
			class="tab-link group relative inline-flex h-10 items-center gap-2 border-b-2 px-3 text-sm font-medium whitespace-nowrap transition-all
				{active
				? 'border-primary text-foreground'
				: 'border-transparent text-muted-foreground hover:border-border hover:text-foreground'}"
		>
			<span
				class="grid size-6 shrink-0 place-items-center rounded-md transition-all
					{active ? 'bg-primary/10 text-primary' : 'text-muted-foreground group-hover:text-foreground'}"
			>
				<Icon class="size-3.5" aria-hidden="true" />
			</span>
			<span class="truncate">{tab.label}</span>
		</a>
	{/each}
</nav>

<style>
	.tab-bar {
		scrollbar-width: none;
		-ms-overflow-style: none;
	}
	.tab-bar::-webkit-scrollbar {
		display: none;
	}
	.tab-link {
		transition:
			border-color 150ms ease,
			color 150ms ease,
			background-color 150ms ease;
	}
</style>
