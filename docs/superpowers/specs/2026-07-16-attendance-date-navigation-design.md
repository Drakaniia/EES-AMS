# Design Spec: Attendance Page Date Indicator & Navigation Controls

**Date:** 2026-07-16  
**Status:** Approved

## Overview

This design outlines the addition of a clear, human-readable relative date indicator and seamless day-by-day date navigation (previous/next arrows) on the attendance page. This navigation serves as a direct, fast alternative to the existing calendar date picker dialog.

## Motivation

Currently, navigating to a different day requires clicking the date button, opening the full `DatePickerDialog` modal, finding the target date, and clicking it. For day-to-day work, teachers need a rapid, seamless way to switch back and forth by one day (e.g. to check yesterday's logs or log attendance for an adjacent day) without opening a modal dialog. Additionally, the date button only displays the raw ISO date format (e.g., `2026-07-16`), which is not user-friendly.

---

## Detailed Design

### 1. Pure Helper Additions

We will add utility functions in [attendance-state.svelte.ts](file:///C:/Users/Qwenzy/Desktop/ees_ams/src/routes/attendance/attendance-state.svelte.ts) for date arithmetic.

```typescript
/**
 * Offsets a date key string (YYYY-MM-DD) by a given number of days.
 * Returns the new date key formatted as YYYY-MM-DD.
 */
export function adjustDate(dateKey: string, offsetDays: number): string {
	const parts = parseDateKey(dateKey);
	if (!parts) return dateKey;

	const date = new Date(parts.year, parts.monthIndex, parts.day);
	date.setDate(date.getDate() + offsetDays);
	return fmtDate(date.getTime());
}
```

### 2. UI Component Modifications

In [attendance/+page.svelte](file:///C:/Users/Qwenzy/Desktop/ees_ams/src/routes/attendance/+page.svelte), we will:

1. Import `ChevronLeft` and `ChevronRight` from `lucide-svelte`.
2. Define a derived reactive state `displayDateLabel` that uses the chosen relative date labels (`Today • ...`, `Yesterday • ...`, `Tomorrow • ...`):

   ```typescript
   const displayDateLabel = $derived.by(() => {
   	const today = fmtDate(Date.now());
   	const yesterday = adjustDate(today, -1);
   	const tomorrow = adjustDate(today, 1);

   	const formatted = formatAttendanceDate(selectedDate);
   	if (selectedDate === today) {
   		return `Today • ${formatted}`;
   	} else if (selectedDate === yesterday) {
   		return `Yesterday • ${formatted}`;
   	} else if (selectedDate === tomorrow) {
   		return `Tomorrow • ${formatted}`;
   	}
   	return formatted;
   });
   ```

3. Implement `handleDateOffset(offset: number)` to trigger date updates:
   ```typescript
   function handleDateOffset(offset: number) {
   	const nextDate = adjustDate(selectedDate, offset);
   	void selectAttendanceDate(nextDate);
   }
   ```
4. Replace the old date button in the header actions block with the unified navigation control:

   ```html
   <div
   	class="inline-flex items-center rounded-full border border-border bg-background p-0.5 shadow-sm"
   >
   	<button type="button" onclick="{()" ="">
   		handleDateOffset(-1)} disabled={dateLoading || isProcessing} class="flex size-9 items-center
   		justify-center rounded-full text-muted-foreground hover:bg-surface hover:text-foreground
   		disabled:opacity-40 transition-colors cursor-pointer" aria-label="Previous day" >
   		<ChevronLeft class="size-4" />
   	</button>

   	<button type="button" onclick="{()" ="">
   		(datePickerOpen = true)} disabled={dateLoading || isProcessing} class="inline-flex h-9
   		items-center gap-2 rounded-full px-3 text-sm font-semibold hover:bg-surface transition-colors
   		cursor-pointer disabled:opacity-60" aria-haspopup="dialog" aria-expanded={datePickerOpen} >
   		{#if dateLoading}
   		<span class="size-2 animate-pulse rounded-full bg-primary" aria-hidden="true"></span>
   		{:else}
   		<CalendarDays class="size-4 text-primary" aria-hidden="true" />
   		{/if}
   		<span class="font-mono text-xs md:text-sm">{displayDateLabel}</span>
   	</button>

   	<button type="button" onclick="{()" ="">
   		handleDateOffset(1)} disabled={dateLoading || isProcessing} class="flex size-9 items-center
   		justify-center rounded-full text-muted-foreground hover:bg-surface hover:text-foreground
   		disabled:opacity-40 transition-colors cursor-pointer" aria-label="Next day" >
   		<ChevronRight class="size-4" />
   	</button>
   </div>
   ```

---

## Verification Plan

### Manual Verification

1. **Initial Load**: Open the attendance page. The date indicator should display `Today • [DayOfWeek, Month Date, Year]` (e.g., `Today • Thu, Jul 16, 2026`).
2. **Backward Navigation**: Click the left arrow. The indicator should update to `Yesterday • [DayOfWeek, Month Date, Year]` (e.g., `Yesterday • Wed, Jul 15, 2026`), and events for that day should load correctly.
3. **Forward Navigation**: Click the right arrow. The indicator should update back to `Today ...`. Click again to go to `Tomorrow • ...` to verify the "Tomorrow" relative label works correctly.
4. **Subsequent Days**: Navigate further back or forward. The relative label prefix should disappear, leaving just the clean formatted date (e.g., `Tue, Jul 14, 2026`).
5. **Modal Interaction**: Click the middle date label. The datepicker dialog should open. Pick a date (e.g., 5 days ago) and ensure it loads. Verify the navigation arrows update relative to this newly selected date (i.e. clicking left goes to 6 days ago).
6. **Loading & Processing States**: Ensure that clicking the navigation arrows disables all controls and displays the loading/processing indicator correctly while data is fetched.
7. **Accessibility Verification**: Ensure the screen reader labels are descriptive and elements are keyboard focusable.
