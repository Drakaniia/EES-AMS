# Right-Click to Mark Absent — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a right-click shortcut on student boxes/rows in the Attendance page that marks a student absent directly, and make "Present all" skip absent-marked students.

**Architecture:** New `markAbsent()` method on `AttendancePageState` (deletes an existing `in` record via the existing `logForStudent` null path, or adds a session absent highlight for unrecorded students), threaded as a new `onMarkAbsent` prop through `AttendanceGrid` and `AttendanceManualLogDialog`, wired in `+page.svelte`. `presentAllStudents()` gains an absent-skip filter and keeps highlights.

**Tech Stack:** Svelte 5 (runes), SvelteKit, TypeScript, bun, Tailwind CSS, lucide-svelte. Frontend only — no Rust changes.

**Spec:** `docs/superpowers/specs/2026-07-31-attendance-right-click-absent-design.md`

## Global Constraints

- Indentation: **tabs** (repo style) in `.ts`/`.svelte` files.
- No new dependencies. No changes outside `src/routes/attendance/` and `src/routes/attendance/logs/` — actually only: `attendance-state.svelte.ts`, `attendance-page-state.svelte.ts`, `attendance-grid.svelte`, `attendance-manual-log-dialog.svelte`, `+page.svelte`.
- Right-click sites MUST call `e.preventDefault()` (suppresses the native browser context menu) before `onMarkAbsent(student)`.
- No unit tests for `AttendancePageState` (spec decision: it is coupled to Tauri commands and has no test harness). Verification is `bun run check` (must report "0 errors") + the manual QA checklist in Task 6.
- **Task 0 first:** the working tree contains uncommitted, coherent prior work ("present-by-default marking" — `absentStudentIds`, present/absent/pending counts, `getStudentStatus` tones) that this feature builds on. It must be committed as a baseline before feature work so feature commits stay reviewable.
- Behavior contract (from spec): right-click on Present → delete `in` record + absent highlight, toast `{name} marked absent`; on Pending → absent highlight only, toast `{name} marked absent`; on already-Absent → silent no-op. Left-click behavior unchanged. "Present all" skips absent-marked students and keeps their highlights.

---

### Task 0: Commit pre-existing present-by-default baseline

**Files:**
- Commit: `src/routes/attendance/+page.svelte`, `src/routes/attendance/attendance-grid.svelte`, `src/routes/attendance/attendance-manual-log-dialog.svelte`, `src/routes/attendance/attendance-page-state.svelte.ts`, `src/routes/attendance/attendance-state.svelte.ts`, `src/routes/attendance/day-overview/+page.svelte`

**Interfaces:**
- Consumes: the current (already modified) working tree.
- Produces: a clean baseline where `AttendancePageState` exposes `absentStudentIds: SvelteSet<string>`, `getStudentStatus()` returning `present | absent | pending` tones, `presentCount`/`absentCount`/`pendingCount`, `matchesSelectedClass(student)`, `getLastEventForSession(student)`, and `LogOptions.suppressLate`. Later tasks edit on top of these.

- [ ] **Step 1: Confirm baseline compiles**

Run: `bun run check`
Expected: `svelte-check found 0 errors and 0 warnings`

- [ ] **Step 2: Review what is being committed**

Run: `git status --short` and `git diff --stat`
Expected: only the 6 attendance files above (plus untracked `.omo/` and `docs/superpowers/` — those stay untracked).

- [ ] **Step 3: Commit the baseline**

```bash
git add src/routes/attendance/+page.svelte src/routes/attendance/attendance-grid.svelte src/routes/attendance/attendance-manual-log-dialog.svelte src/routes/attendance/attendance-page-state.svelte.ts src/routes/attendance/attendance-state.svelte.ts src/routes/attendance/day-overview/+page.svelte
git commit -m "feat(attendance): present-by-default marking baseline"
```

---

### Task 1: `LogOptions.message` — custom toast on attendance removal

**Files:**
- Modify: `src/routes/attendance/attendance-state.svelte.ts:16-20` (LogOptions type)
- Modify: `src/routes/attendance/attendance-page-state.svelte.ts` — `logForStudent` delete branch (the `if (type === null && last)` block)

**Interfaces:**
- Consumes: existing `LogOptions` type with `timestamp?` and `suppressLate?`.
- Produces: `LogOptions.message?: string` — when present, overrides the `{name} - Attendance removed` toast in the delete-record branch. Task 2 passes it.

- [ ] **Step 1: Extend `LogOptions`**

In `src/routes/attendance/attendance-state.svelte.ts`, change:

```typescript
export type LogOptions = {
	timestamp?: number;
	/** When true, late status is not surfaced in toasts/log pills for this record (manual grid flow). */
	suppressLate?: boolean;
};
```

to:

```typescript
export type LogOptions = {
	timestamp?: number;
	/** When true, late status is not surfaced in toasts/log pills for this record (manual grid flow). */
	suppressLate?: boolean;
	/** Overrides the toast shown when an existing 'in' record is removed (absent shortcut). */
	message?: string;
};
```

- [ ] **Step 2: Use the override in `logForStudent`**

In `src/routes/attendance/attendance-page-state.svelte.ts`, inside `logForStudent`, find the delete branch:

```typescript
		if (type === null && last) {
			try {
				await deleteEvent(last.id, 'Toggled off by user');
				this.events = this.events.filter((e) => e.id !== last.id);
				this.attendanceLog?.removeLogEntry(last.id);
				this.attendanceLog?.showToast(`${student.name} - Attendance removed`);
				this.attendanceLog?.resetUndo();
				this.absentStudentIds.add(student.id);
				return;
			} catch {
				this.attendanceLog?.showToast('Failed to remove attendance', false);
				return;
			}
		}
```

Replace the `showToast` line (keep everything else identical):

```typescript
				this.attendanceLog?.showToast(options.message ?? `${student.name} - Attendance removed`);
```

- [ ] **Step 3: Verify**

Run: `bun run check`
Expected: `svelte-check found 0 errors and 0 warnings`

- [ ] **Step 4: Commit**

```bash
git add src/routes/attendance/attendance-state.svelte.ts src/routes/attendance/attendance-page-state.svelte.ts
git commit -m "feat(attendance): support custom toast message on attendance removal"
```

---

### Task 2: `markAbsent()` method on `AttendancePageState`

**Files:**
- Modify: `src/routes/attendance/attendance-page-state.svelte.ts` — insert after `markStudent` (which ends around line 503)

**Interfaces:**
- Consumes: `LogOptions.message` (Task 1), `getLastEventForSession(student)`, `absentStudentIds`, `logForStudent`, `attendanceLog`, `isProcessing`, `dateLoading`.
- Produces: `async markAbsent(student: Student): Promise<void>` — public method Task 6 wires to the UI props.

- [ ] **Step 1: Add the method**

After the closing brace of `markStudent` (the method ending with `await this.logForStudent(student, action, { suppressLate: true });` … `finally { this.isProcessing = false; }`), insert:

```typescript
	async markAbsent(student: Student) {
		if (this.isProcessing || this.dateLoading) {
			this.attendanceLog?.showToast('Please wait - processing previous request', false);
			return;
		}

		const last = this.getLastEventForSession(student);
		if (!last) {
			if (this.absentStudentIds.has(student.id)) return;
			this.absentStudentIds.add(student.id);
			this.attendanceLog?.showToast(`${student.name} marked absent`);
			return;
		}

		this.isProcessing = true;
		try {
			// Manual grid/dialog marks never surface the late flag in this flow.
			await this.logForStudent(student, null, {
				suppressLate: true,
				message: `${student.name} marked absent`
			});
		} finally {
			this.isProcessing = false;
		}
	}
```

- [ ] **Step 2: Verify**

Run: `bun run check`
Expected: `svelte-check found 0 errors and 0 warnings`

- [ ] **Step 3: Commit**

```bash
git add src/routes/attendance/attendance-page-state.svelte.ts
git commit -m "feat(attendance): add markAbsent shortcut state method"
```

---

### Task 3: "Present all" skips absent-marked students

**Files:**
- Modify: `src/routes/attendance/attendance-page-state.svelte.ts` — `presentAllStudents()` (three edits)

**Interfaces:**
- Consumes: `absentStudentIds` (baseline), existing `studentsToMark` pipeline.
- Produces: `presentAllStudents()` behavior — records every unrecorded student EXCEPT `absentStudentIds` members; absent highlights are kept; toasts include the kept-absent count and the all-recorded-or-absent edge case. Task 6 manually verifies.

- [ ] **Step 1: Exclude absent-marked students from the bulk filter**

In `presentAllStudents()`, change:

```typescript
		const studentsToMark = this.students
			.filter((student) => this.matchesSelectedClass(student))
			.sort((a, b) => a.name.localeCompare(b.name))
			.filter((student) => this.getNextAttendanceType(student) === 'in');
```

to:

```typescript
		const studentsToMark = this.students
			.filter((student) => this.matchesSelectedClass(student))
			.sort((a, b) => a.name.localeCompare(b.name))
			.filter(
				(student) =>
					this.getNextAttendanceType(student) === 'in' &&
					!this.absentStudentIds.has(student.id)
			);
```

- [ ] **Step 2: Distinguish the empty-result toast**

Change:

```typescript
		if (studentsToMark.length === 0) {
			this.attendanceLog?.showToast('All students are already recorded');
			return;
		}
```

to:

```typescript
		if (studentsToMark.length === 0) {
			this.attendanceLog?.showToast(
				this.absentStudentIds.size > 0
					? 'All students are already recorded or marked absent'
					: 'All students are already recorded'
			);
			return;
		}
```

- [ ] **Step 3: Keep absent highlights and report them**

Change:

```typescript
			// Everyone is present again - the session absent highlight no longer applies.
			this.absentStudentIds.clear();

			this.attendanceLog?.showToast(
				`${createdEvents.length} ${createdEvents.length === 1 ? 'student' : 'students'} marked present`
			);
```

to (delete the `clear()` call entirely):

```typescript
			// Absent-marked students were skipped and keep their highlight.
			const absentKept =
				this.absentStudentIds.size > 0
					? ` · ${this.absentStudentIds.size} kept absent`
					: '';
			this.attendanceLog?.showToast(
				`${createdEvents.length} ${createdEvents.length === 1 ? 'student' : 'students'} marked present${absentKept}`
			);
```

- [ ] **Step 4: Verify**

Run: `bun run check`
Expected: `svelte-check found 0 errors and 0 warnings`

- [ ] **Step 5: Commit**

```bash
git add src/routes/attendance/attendance-page-state.svelte.ts
git commit -m "feat(attendance): present-all skips absent-marked students"
```

---

### Task 4: Right-click on student boxes and list rows (AttendanceGrid)

**Files:**
- Modify: `src/routes/attendance/attendance-grid.svelte` (props destructure + type, boxes view button, list view row)

**Interfaces:**
- Consumes: `markAbsent` behavior (Task 2, via the new prop).
- Produces: `AttendanceGrid` prop `onMarkAbsent: (student: Student) => void` — required (no default). Task 6 passes it.

- [ ] **Step 1: Add the prop to destructure and type**

In the props destructure, change:

```typescript
		onGetNextAttendanceType,
		onGetStudentStatus,
		dateNav
```

to:

```typescript
		onGetNextAttendanceType,
		onGetStudentStatus,
		onMarkAbsent,
		dateNav
```

In the props type block, change:

```typescript
		onGetStudentStatus: (student: Student) => { label: string; tone: string };
		dateNav?: Snippet;
```

to:

```typescript
		onGetStudentStatus: (student: Student) => { label: string; tone: string };
		onMarkAbsent: (student: Student) => void;
		dateNav?: Snippet;
```

- [ ] **Step 2: Boxes view — context menu + title hint**

In the boxes view, change the student button opening:

```svelte
					<button
						type="button"
						title={`${student.name} - ${status.label}`}
						disabled={isProcessing || dateLoading}
						onclick={() => onMarkStudent(student, action)}
```

to:

```svelte
					<button
						type="button"
						title={`${student.name} - ${status.label} · Right-click to mark absent`}
						disabled={isProcessing || dateLoading}
						onclick={() => onMarkStudent(student, action)}
						oncontextmenu={(e) => {
							e.preventDefault();
							onMarkAbsent(student);
						}}
```

- [ ] **Step 3: List view — context menu on the row**

In the list view, change the row opening:

```svelte
						<li
							class="flex flex-col gap-3 px-4 py-3 hover:bg-surface/50 sm:flex-row sm:items-center sm:justify-between"
						>
```

to:

```svelte
						<li
							class="flex flex-col gap-3 px-4 py-3 hover:bg-surface/50 sm:flex-row sm:items-center sm:justify-between"
							oncontextmenu={(e) => {
								e.preventDefault();
								onMarkAbsent(student);
							}}
						>
```

- [ ] **Step 4: Verify**

Run: `bun run check`
Expected: `svelte-check found 0 errors and 0 warnings`. NOTE: `AttendanceGrid` is used by `+page.svelte`, which does not pass `onMarkAbsent` yet — if svelte-check reports a missing-prop error here, that is expected; proceed to Task 6 to wire it. If it does NOT error (Svelte may not flag required props for components), continue regardless.

- [ ] **Step 5: Commit**

```bash
git add src/routes/attendance/attendance-grid.svelte
git commit -m "feat(attendance): right-click marks absent in grid views"
```

---

### Task 5: Right-click in the Manual log dialog

**Files:**
- Modify: `src/routes/attendance/attendance-manual-log-dialog.svelte` (props destructure + type, picker row)

**Interfaces:**
- Consumes: `markAbsent` behavior (Task 2, via the new prop).
- Produces: `AttendanceManualLogDialog` prop `onMarkAbsent: (student: Student) => void` — required. Task 6 passes it. Dialog stays open after right-click (no `closePicker`).

- [ ] **Step 1: Add the prop to destructure and type**

In the props destructure, change:

```typescript
		markStudent = async () => {}
```

to:

```typescript
		markStudent = async () => {},
		onMarkAbsent = () => {}
```

In the props type block, change:

```typescript
		markStudent: (
			student: Student,
			action: AttendanceType | null,
			closePicker: boolean
		) => Promise<void>;
	} = $props();
```

to:

```typescript
		markStudent: (
			student: Student,
			action: AttendanceType | null,
			closePicker: boolean
		) => Promise<void>;
		onMarkAbsent: (student: Student) => void;
	} = $props();
```

- [ ] **Step 2: Context menu on the picker row**

Change:

```svelte
				<li>
					<button
						disabled={isProcessing || dateLoading}
						onclick={() => markStudent(student, action, true)}
```

to:

```svelte
				<li
					oncontextmenu={(e) => {
						e.preventDefault();
						onMarkAbsent(student);
					}}
				>
					<button
						disabled={isProcessing || dateLoading}
						onclick={() => markStudent(student, action, true)}
```

- [ ] **Step 3: Verify**

Run: `bun run check`
Expected: `svelte-check found 0 errors and 0 warnings` (same note as Task 4 Step 4 — a missing-prop error until Task 6 wires `+page.svelte` is expected and OK).

- [ ] **Step 4: Commit**

```bash
git add src/routes/attendance/attendance-manual-log-dialog.svelte
git commit -m "feat(attendance): right-click marks absent in manual log dialog"
```

---

### Task 6: Wire `onMarkAbsent` in `+page.svelte` + full verification

**Files:**
- Modify: `src/routes/attendance/+page.svelte` (both component usages)

**Interfaces:**
- Consumes: `attendanceState.markAbsent` (Task 2), `onMarkAbsent` props (Tasks 4 & 5).
- Produces: the fully wired feature.

- [ ] **Step 1: Pass the prop to AttendanceGrid**

Change:

```svelte
			onGetNextAttendanceType={(student) => attendanceState.getNextAttendanceType(student)}
			onGetStudentStatus={(student) => attendanceState.getStudentStatus(student)}
		>
```

to:

```svelte
			onGetNextAttendanceType={(student) => attendanceState.getNextAttendanceType(student)}
			onGetStudentStatus={(student) => attendanceState.getStudentStatus(student)}
			onMarkAbsent={(student: Student) => void attendanceState.markAbsent(student)}
		>
```

- [ ] **Step 2: Pass the prop to AttendanceManualLogDialog**

Change:

```svelte
	markStudent={(student: Student, action: AttendanceType | null, closePicker = false) =>
		attendanceState.markStudent(student, action, closePicker)}
/>
```

to:

```svelte
	markStudent={(student: Student, action: AttendanceType | null, closePicker = false) =>
		attendanceState.markStudent(student, action, closePicker)}
	onMarkAbsent={(student: Student) => void attendanceState.markAbsent(student)}
/>
```

- [ ] **Step 3: Verify compile**

Run: `bun run check`
Expected: `svelte-check found 0 errors and 0 warnings`

- [ ] **Step 4: Run the full test suite (regression)**

Run: `bun test`
Expected: all existing tests pass.

- [ ] **Step 5: Manual QA — follow the spec's Verification checklist**

Run: `bun run tauri dev` (or `bun run dev` if only the web UI is being checked).

1. Attendance page, Boxes view: right-click a **pending** box → turns absent (red), Absent stat +1, toast `{name} marked absent`.
2. Right-click a **present** box → `in` record deleted, box turns absent, toast `{name} marked absent`.
3. Right-click an **absent** box → nothing changes, no toast.
4. Switch to List view: right-click a pending **row** → same as boxes.
5. Open **Manual log** dialog → right-click a pending student → marked absent, dialog **stays open**.
6. Right-click 3 students absent, then click **Present all** → the 3 stay absent, everyone else recorded, toast `{X} marked present · 3 kept absent`.
7. "Present all" when everyone is recorded or absent-marked → toast `All students are already recorded or marked absent`.
8. **Left-click** flows still work exactly as before (pending→present, present→absent, absent→present).
9. No native browser context menu appears at any right-click site.
10. Date/class change resets absent highlights; "Clear all" still resets them.

- [ ] **Step 6: Commit**

```bash
git add src/routes/attendance/+page.svelte
git commit -m "feat(attendance): wire right-click absent shortcut"
```

---

## Self-Review Notes

- **Spec coverage:** markAbsent (Task 2), LogOptions.message toast consistency (Task 1), present-all skip + kept-absent toast + edge message (Task 3), boxes/list/dialog contextmenu with preventDefault + dialog stays open (Tasks 4–5), page wiring + full QA (Task 6). All spec sections mapped.
- **Type consistency:** `markAbsent(student: Student): Promise<void>` is the single state entry point used by both props; `onMarkAbsent: (student: Student) => void` has the same name and shape in both components. `LogOptions.message?: string` used only in Task 1's delete branch.
- **No placeholders:** every step has exact paths, code, commands, and expected output.
