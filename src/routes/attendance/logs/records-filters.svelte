<script lang="ts">
	import { CalendarDays, ChevronDown } from 'lucide-svelte';
	import DateRangePicker from '$lib/components/ui/DateRangePicker.svelte';
	import StudentPicker from '$lib/components/ui/StudentPicker.svelte';
	import type { Student, Class } from '$lib/api';

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

<section
	class="flex flex-wrap items-end gap-2 px-5 pt-4 pb-3 md:px-8 lg:px-10"
	aria-label="Record filters"
>
	<div class="min-w-48 flex-1">
		<span class="mb-1 block text-[11px] font-medium text-muted-foreground">Date range</span>
		<button
			onclick={onDateRangePickerOpen}
			class="field flex items-center justify-between gap-2 text-left"
		>
			<span class={from || to ? '' : 'text-muted-foreground'}>
				{from && to
					? `${new Date(from).toLocaleDateString()} – ${new Date(to).toLocaleDateString()}`
					: from
						? `From ${new Date(from).toLocaleDateString()}`
						: to
							? `To ${new Date(to).toLocaleDateString()}`
							: 'Any date'}
			</span>
			<CalendarDays
				class="size-3.5 shrink-0 text-muted-foreground"
				aria-hidden="true"
				strokeWidth={1.9}
			/>
		</button>
	</div>

	<div class="min-w-40 flex-1">
		<span class="mb-1 block text-[11px] font-medium text-muted-foreground">Class</span>
		{#if classes.length <= 1}
			<div class="field flex items-center text-muted-foreground">
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
					class="select-field appearance-none"
				>
					<option value="">All classes</option>
					{#each classes as c (c.id)}
						<option value={c.id}>{c.name}</option>
					{/each}
				</select>
				<ChevronDown
					class="pointer-events-none absolute inset-y-0 right-2.5 my-auto size-3.5 text-muted-foreground"
					aria-hidden="true"
				/>
			</div>
		{/if}
	</div>

	<div class="min-w-44 flex-1">
		<span class="mb-1 block text-[11px] font-medium text-muted-foreground">Student</span>
		<StudentPicker
			{students}
			selectedId={studentId}
			{classId}
			placeholder="All students"
			onSelect={({ id }) => onStudentChange(id)}
		/>
	</div>

	<div class="ml-auto flex items-baseline gap-1.5 pb-2 text-[11.5px] text-muted-foreground">
		<span class="text-[13px] font-semibold text-foreground tabular-nums">{recordCount}</span>
		<span>{recordCount === 1 ? 'record' : 'records'}</span>
	</div>
</section>

<DateRangePicker
	open={dateRangePickerOpen}
	fromValue={from}
	toValue={to}
	onClose={onDateRangePickerClose}
	onSelect={onDateRangeChange}
/>
