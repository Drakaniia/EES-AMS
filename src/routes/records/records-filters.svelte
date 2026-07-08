<script lang="ts">
	import DateRangePicker from '$lib/components/ui/DateRangePicker.svelte';
	import StudentPicker from '$lib/components/ui/StudentPicker.svelte';
	import type { Student, Class } from '$lib/db-rust';

	let {
		from,
		to,
		classId,
		studentId,
		classes,
		students,
		recordCount,
		dateRangePickerOpen,
		onDateRangeChange,
		onClassChange,
		onStudentChange,
		onDateRangePickerOpen,
		onDateRangePickerClose
	}: {
		from: string;
		to: string;
		classId: string;
		studentId: string;
		classes: Class[];
		students: Student[];
		recordCount: number;
		dateRangePickerOpen: boolean;
		onDateRangeChange: (range: { from: string; to: string }) => void;
		onClassChange: (classId: string) => void;
		onStudentChange: (id: string) => void;
		onDateRangePickerOpen: () => void;
		onDateRangePickerClose: () => void;
	} = $props();
</script>

<section class="grid gap-4 px-4 py-5 sm:grid-cols-2 md:px-8 lg:grid-cols-4 lg:px-10">
	<!-- Date Range -->
	<div class="space-y-2">
		<div class="label-mono">Date Range</div>
		<button
			onclick={onDateRangePickerOpen}
			class="flex h-10 w-full items-center justify-between rounded-md border border-border bg-background px-3 text-left text-sm transition-colors hover:bg-surface focus:ring-2 focus:ring-primary focus:outline-none"
		>
			<span class={from || to ? '' : 'text-muted-foreground'}>
				{from && to
					? `${new Date(from).toLocaleDateString()} - ${new Date(to).toLocaleDateString()}`
					: from
						? `From ${new Date(from).toLocaleDateString()}`
						: to
							? `To ${new Date(to).toLocaleDateString()}`
							: 'Select date range'}
			</span>
			<svg
				class="size-4 text-muted-foreground"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
				<line x1="16" y1="2" x2="16" y2="6"></line>
				<line x1="8" y1="2" x2="8" y2="6"></line>
				<line x1="3" y1="10" x2="21" y2="10"></line>
			</svg>
		</button>
	</div>

	<!-- Class -->
	<div class="space-y-2">
		<div class="label-mono">Class</div>
		{#if classes.length <= 1}
			<div
				class="flex h-10 items-center rounded-md border border-border bg-surface px-3 text-sm font-medium"
			>
				{classes[0]?.name ?? 'No class configured'}
			</div>
		{:else}
			<div class="relative">
				<select
					value={classId}
					onchange={(e) => {
						onClassChange((e.currentTarget as HTMLSelectElement).value);
						onStudentChange('');
					}}
					class="h-10 w-full appearance-none rounded-md border border-border bg-background px-3 pr-10 text-sm transition-colors hover:bg-surface focus:ring-2 focus:ring-primary focus:outline-none"
				>
					<option value="">All classes</option>
					{#each classes as c (c.id)}
						<option value={c.id}>{c.name}</option>
					{/each}
				</select>
				<div
					class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-muted-foreground"
				>
					<svg
						class="size-4"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<path d="m6 9 6 6 6-6" />
					</svg>
				</div>
			</div>
		{/if}
	</div>

	<!-- Student -->
	<div class="space-y-2">
		<div class="label-mono">Student</div>
		<StudentPicker
			{students}
			selectedId={studentId}
			{classId}
			placeholder="All students"
			onSelect={({ id }) => onStudentChange(id)}
		/>
	</div>

	<!-- Total -->
	<div class="space-y-2">
		<div class="label-mono">Total attendance records</div>
		<div class="flex h-10 items-center font-mono text-sm">{recordCount}</div>
	</div>
</section>

<DateRangePicker
	open={dateRangePickerOpen}
	fromValue={from}
	toValue={to}
	onClose={onDateRangePickerClose}
	onSelect={onDateRangeChange}
/>
