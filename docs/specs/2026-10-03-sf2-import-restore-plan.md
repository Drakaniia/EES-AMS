# SF2 Import-Restore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the three Settings → SF2 workflows (Create From Template, Import SF2, Sync Roster) as TypeScript UI over the existing TS backend, in the working tree on `main`.

**Architecture:** One new backend composition module (`template/import.ts`: stage → analyze → validate → upsert) reusing `readWorkbookAnalysis`, `importValidationFromAnalysis`, `syncWorkbookLearnerMappings`, `upsertTemplateWithMappings`; thin wrappers in `$lib/api/sf2.ts`; three handlers plus a create form and a validation dialog in Settings. No Rust, no new Excel logic.

**Tech Stack:** SvelteKit 5 (runes), TypeScript (strict, no `any`), ExcelJS via existing `openWorkbook`/`saveWorkbookAtomic`, Vitest (`bun run test` only).

**Spec:** `docs/specs/2026-10-03-sf2-import-restore-design.md`

## Global Constraints

- Route code imports from `$lib/api` only — never `$lib/db/repos` or `@tauri-apps/*` (pickers via lazy `import('./pickers')` inside `$lib/api` functions).
- All file I/O through `getFileSystem()`; workbook writes via `saveWorkbookAtomic`/`writeFileAtomic` only.
- No `as any`, no `@ts-ignore`; Prettier (`useTabs: true`) clean.
- `errorMessage()` strings are a UI contract — reuse existing messages byte-for-byte, never reword.
- Tests: Vitest only, `bun run test` / `bunx vitest run <path>` (never `bun test`); real SQL via `NodeSqlDriver` (`useTestDb`), fake disk via `MemoryFileSystem`, real template bytes via `loadTemplate()`.
- Route/state files stay under 400 lines; one store per domain; Svelte 5 runes only.
- No commits in this plan — changes stay in the working tree on `main` until the user asks.
- New button labels must never read "Import X from Workbook" (retired control).

---

## File Structure

| File                                                         | Responsibility                                                                                                                                  |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib/features/sf2/template/import.ts` (create)           | Stage picked file in-scope, analyze, validate-gate, adopt roster, upsert template row. Pure composition over existing modules.                  |
| `src/lib/features/sf2/__tests__/import.test.ts` (create)     | Backend tests: happy path, `.xls` refusal, mismatch gate, duplicate refusal, empty-analysis refusal.                                            |
| `src/lib/api/sf2.ts` (modify)                                | Add `createSf2WorkbookFromTemplate`, `pickImportWorkbookFile`, `stageImportWorkbook`, `validateSf2WorkbookImport`, `importSf2WorkbookFromFile`. |
| `src/routes/settings/sf2-state.svelte.ts` (modify)           | Create-draft fields, import-validation state, class list, four handlers.                                                                        |
| `src/routes/settings/sf2-section.svelte` (modify)            | Buttons + create form; renders the dialog.                                                                                                      |
| `src/routes/settings/sf2-import-dialog.svelte` (create)      | Validation report dialog (counts, mismatches, Proceed anyway / Cancel).                                                                         |
| `src/routes/reports/import-control-removed.test.ts` (delete) | Premise void — the control is intentionally back.                                                                                               |

`template/import.ts` depends on: `platform/fs` (`getFileSystem`), `db` (`invalidInput`), `db/repos/settings` (`getSettings`), `features/excel/workbook` (`openWorkbook`), `sf2/roster/analysis` (`readWorkbookAnalysis`), `sf2/metadata` (`dateMappingsFromAnalysis`, `metadataFromImportAnalysis`), `sf2/validation-service` (`importValidationFromAnalysis`), `sf2/validation` (`ensureImportValidationAllows`, type `Sf2ImportValidation`), `sf2/naming` (`className`), `sf2/roster/helpers` (`findOrCreateClass`), `sf2/roster/learner-sync` (`syncWorkbookLearnerMappings`), `sf2/workbook-files` (`getSf2WorkbookDir`, `singleWorkbookPath`, `legacyWorkbookFileName`, `layoutFingerprint`, `hashBytes`), `sf2/repository` (`upsertTemplateWithMappings`, `latestTemplateForClass`, `EMPTY_DATE_ANALYSIS_MESSAGE`, `EMPTY_ROSTER_ANALYSIS_MESSAGE`), `domain/models` (`nowEpochSeconds`), `features/backup/paths` (`baseName`).

---

### Task 1: Backend — `importSf2WorkbookFromFile` plus validation entry

**Files:**

- Create: `src/lib/features/sf2/template/import.ts`
- Test: `src/lib/features/sf2/__tests__/import.test.ts`

**Interfaces:**

- Consumes: everything in the File Structure table above, with the exact signatures already in the codebase (`readWorkbookAnalysis(workbook: Workbook): Sf2WorkbookAnalysis`; `importValidationFromAnalysis(sourcePath, analysis): Promise<Sf2ImportValidation>`; `ensureImportValidationAllows(validation, proceedAnyway): void`; `syncWorkbookLearnerMappings(classId, templateId, learners): Promise<WorkbookLearnerSync>`; `dateMappingsFromAnalysis(templateId, analysis): Sf2DateMapping[]`; `findOrCreateClass(name, settings?): Promise<Class>`; `upsertTemplateWithMappings(template, students, dates): Promise<void>`; `latestTemplateForClass(classId)`; `singleWorkbookPath(dir, {gradeLevel, section, templateId})`; `getSf2WorkbookDir()`).
- Produces: `stageImportSource(pickedPath: string): Promise<string>`; `validateSf2WorkbookImport(stagedPath: string): Promise<Sf2ImportValidation>`; `importSf2WorkbookFromFile(stagedPath: string, proceedAnyway: boolean): Promise<Sf2ImportSummary>`.

- [ ] **Step 1: Write the failing test** — `src/lib/features/sf2/__tests__/import.test.ts`. Bind the world exactly like `template.test.ts`: `useTestDb()`, the same `SF2_SCHEMA` script (settings + sf2_student_mappings + sf2_templates + sf2_date_mappings DDL copied verbatim), `useSf2WorkbookDir('/workbooks')` in `beforeEach` (reset to `null` in `afterEach`), `MemoryFileSystem` bound via the template fixture path. Fixture bytes for the "external" file: `loadTemplate()` returns a fixture whose `open()` gives a `Workbook`; serialize with `workbookToBytes` and write to `/picked/school-file.xlsx` on the memory fs. First test only:

```ts
test('imports a staged workbook: adopts its roster and records the template', async () => {
	const staged = await stageImportSource('/picked/school-file.xlsx');
	const summary = await importSf2WorkbookFromFile(staged, false);
	expect(summary.learnersFound).toBeGreaterThan(0);
	expect(summary.datesMapped).toBeGreaterThan(0);
	expect(summary.sourcePath).toContain('/workbooks/SF2-');
	const latest = await latestTemplateForClass(summary.classId);
	expect(latest?.id).toBe(summary.templateId);
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `bunx vitest run src/lib/features/sf2/__tests__/import.test.ts`
Expected: FAIL with "Cannot find module" / "stageImportSource is not defined" (feature missing, not a typo).

- [ ] **Step 3: Write minimal implementation** — `src/lib/features/sf2/template/import.ts`:

```ts
import { getSettings } from '$lib/db/repos/settings';
import { invalidInput } from '$lib/db';
import { nowEpochSeconds } from '$lib/domain/models';
import { baseName } from '$lib/features/backup/paths';
import { openWorkbook } from '$lib/features/excel/workbook';
import { getFileSystem } from '$lib/platform/fs';
import type { Sf2ImportSummary } from '$lib/types';
import { readWorkbookAnalysis } from '../roster/analysis';
import { findOrCreateClass } from '../roster/helpers';
import { syncWorkbookLearnerMappings } from '../roster/learner-sync';
import { className } from '../naming';
import { dateMappingsFromAnalysis, metadataFromImportAnalysis } from '../metadata';
import {
	EMPTY_DATE_ANALYSIS_MESSAGE,
	EMPTY_ROSTER_ANALYSIS_MESSAGE,
	latestTemplateForClass,
	upsertTemplateWithMappings
} from '../repository';
import { ensureImportValidationAllows, type Sf2ImportValidation } from '../validation';
import { importValidationFromAnalysis } from '../validation-service';
import {
	getSf2WorkbookDir,
	hashBytes,
	layoutFingerprint,
	singleWorkbookPath
} from '../workbook-files';

const STAGING_FOLDER = 'import-staging';

/** Copy a teacher-picked file somewhere the fs allow-list covers (mirrors `restore-staging.ts`: sweep-first, one slot). */
export async function stageImportSource(pickedPath: string): Promise<string> {
	const lower = pickedPath.toLowerCase();
	if (lower.endsWith('.xls') && !lower.endsWith('.xlsx'))
		throw invalidInput(
			'This is an old .xls workbook. Open it once in Excel, Save As .xlsx, then import from that file.'
		);
	const fs = getFileSystem();
	const bytes = await fs.readFile(pickedPath);
	const dir = `${await getSf2WorkbookDir()}/${STAGING_FOLDER}`;
	await fs.mkdirp(dir);
	for (const name of await fs.readDir(dir)) {
		if (name.endsWith('/')) continue;
		await fs.remove(`${dir}/${name}`);
	}
	const path = `${dir}/import-${baseName(pickedPath)}`;
	await fs.writeFileAtomic(path, bytes);
	return path;
}

/** Analyze a staged file and report roster disagreements without writing anything. */
export async function validateSf2WorkbookImport(stagedPath: string): Promise<Sf2ImportValidation> {
	return importValidationFromAnalysis(
		stagedPath,
		readWorkbookAnalysis(await openWorkbook(stagedPath))
	);
}

export async function importSf2WorkbookFromFile(
	stagedPath: string,
	proceedAnyway: boolean
): Promise<Sf2ImportSummary> {
	const fs = getFileSystem();
	const bytes = await fs.readFile(stagedPath);
	const workbook = await openWorkbook(stagedPath);
	const analysis = readWorkbookAnalysis(workbook);
	const validation = await importValidationFromAnalysis(stagedPath, analysis);
	ensureImportValidationAllows(validation, proceedAnyway);
	if (analysis.dates.length === 0) throw invalidInput(EMPTY_DATE_ANALYSIS_MESSAGE);
	const metadata = metadataFromImportAnalysis(analysis);
	const classRecord = await findOrCreateClass(
		className(metadata.gradeLevel, metadata.section),
		await getSettings()
	);
	const existing = await latestTemplateForClass(classRecord.id);
	if (existing !== undefined)
		throw invalidInput(
			`An SF2 workbook already exists for ${classRecord.name}. ` +
				'Update the existing workbook settings instead of creating a new one'
		);
	const templateId = crypto.randomUUID();
	const sync = await syncWorkbookLearnerMappings(classRecord.id, templateId, analysis.learners);
	if (sync.studentMappings.length === 0) throw invalidInput(EMPTY_ROSTER_ANALYSIS_MESSAGE);
	const dateMappings = dateMappingsFromAnalysis(templateId, analysis);
	const sourcePath = singleWorkbookPath(await getSf2WorkbookDir(), {
		templateId,
		gradeLevel: metadata.gradeLevel,
		section: metadata.section
	});
	await fs.rename(stagedPath, sourcePath);
	const template = {
		id: templateId,
		sourcePath,
		sourceHash: `imported-${hashBytes(bytes)}-${classRecord.id}`,
		schoolId: metadata.schoolId,
		schoolName: metadata.schoolName,
		schoolYear: metadata.schoolYear,
		reportMonth: metadata.reportMonth,
		gradeLevel: metadata.gradeLevel,
		section: metadata.section,
		adviserName: metadata.adviserName,
		schoolHeadName: metadata.schoolHeadName,
		layoutFingerprint: layoutFingerprint(analysis),
		activeClassId: classRecord.id,
		importedAt: nowEpochSeconds(),
		lastSyncedAt: undefined
	};
	await upsertTemplateWithMappings(template, sync.studentMappings, dateMappings);
	return {
		templateId,
		classId: classRecord.id,
		className: classRecord.name,
		sourcePath,
		schoolYear: template.schoolYear,
		gradeLevel: template.gradeLevel,
		section: template.section,
		learnersFound: sync.studentMappings.length,
		studentsCreated: sync.studentsCreated,
		studentsReused: sync.studentsReused,
		datesMapped: dateMappings.length
	};
}
```

Notes for the implementer: `sourceHash` deliberately lacks the `bundled-` prefix, so `templateOwnsRoster` treats this as the school's workbook (adopt branch). `getSettings()` mirrors `create.ts:65`. If `FileSystem` has no `rename`, use read-then-`writeFileAtomic`-then-`remove` and note the deviation in the final report.

- [ ] **Step 4: Run the test to verify it passes**

Run: `bunx vitest run src/lib/features/sf2/__tests__/import.test.ts`
Expected: PASS.

- [ ] **Step 5: Add the four refusal tests** (same file, each RED first if the behavior is not yet in the implementation — they are, so they pass immediately as characterization):

```ts
test('refuses an old .xls file with the Save-As message', async () => {
	expect(await failure(stageImportSource('/picked/old.xls'))).toMatch(/Save As \.xlsx/);
});

test('refuses a roster mismatch unless the teacher proceeds', async () => {
	await seedClassWithStudents(); // class 'Grade 3 - MATAPAT' with students the fixture roster disagrees with
	const staged = await stageImportSource('/picked/school-file.xlsx');
	expect(await failure(importSf2WorkbookFromFile(staged, false))).toMatch(
		/Student List Mismatch Detected/
	);
	const summary = await importSf2WorkbookFromFile(
		await stageImportSource('/picked/school-file.xlsx'),
		true
	);
	expect(summary.learnersFound).toBeGreaterThan(0);
});

test('refuses a second import for a class that has a workbook', async () => {
	await importSf2WorkbookFromFile(await stageImportSource('/picked/school-file.xlsx'), true);
	expect(
		await failure(
			importSf2WorkbookFromFile(await stageImportSource('/picked/school-file.xlsx'), true)
		)
	).toMatch(/already exists for/);
});

test('refuses a workbook with no learners', async () => {
	const empty = await workbookToBytes(newWorkbook('JUNE 2025'));
	await fs().writeFileAtomic('/picked/empty.xlsx', empty);
	expect(
		await failure(importSf2WorkbookFromFile(await stageImportSource('/picked/empty.xlsx'), true))
	).toMatch(/no learners|no calendar dates/i);
});
```

Helpers to define in the test file (copy the `failure` helper verbatim from `template.test.ts:144-152`): `seedClassWithStudents` (same `SF2_SCHEMA` script + inserts whose names disagree with the fixture roster — reuse the three seed names; the fixture roster differs), `fs()` returning `getFileSystem()`. `newWorkbook` from `template-fixture.ts:75`, `workbookToBytes` from `features/excel/workbook`. Note: re-staging before the second call because a successful import renames the staged file away.

- [ ] **Step 6: Run the full backend-related suites**

Run: `bunx vitest run src/lib/features/sf2/__tests__/import.test.ts`, then `bunx vitest run src/lib/features/sf2/__tests__/template.test.ts`
Expected: both PASS (no regressions in create/update).

### Task 2: API surface in `$lib/api/sf2.ts`

**Files:**

- Modify: `src/lib/api/sf2.ts`
- Test: existing `src/lib/api/__tests__/barrel.test.ts` (run; extend only if it enumerates exports)

**Interfaces:**

- Consumes: `createWorkbookFromTemplate` (`template/create.ts:89`), `stageImportSource` / `validateSf2WorkbookImport` / `importSf2WorkbookFromFile` (Task 1), `syncWorkbookRosterForClass` (already imported), `pickWorkbookFile` (lazy `./pickers`).
- Produces: `createSf2WorkbookFromTemplate(draft: Sf2TemplateDraft): Promise<Sf2ImportSummary>`; `pickImportWorkbookFile(): Promise<string | null>`; `stageImportWorkbook(pickedPath: string): Promise<string>`; `validateSf2WorkbookImportFile(stagedPath: string): Promise<Sf2ImportValidation>`; `importSf2Workbook(stagedPath: string, proceedAnyway: boolean): Promise<Sf2ImportSummary>`.

- [ ] **Step 1: Write the failing test** — no new test file (thin wrappers; covered by Task 1 + Task 4 flows). Instead RED is: `bun run check` fails on the not-yet-existing imports once Task 3 references them — skip to implementation, with the Task 3 UI test as the proving test. (Justification: wrappers are one-line pass-throughs over Task-1-tested functions; testing them twice adds no signal.)
- [ ] **Step 2: Implement** — append to `src/lib/api/sf2.ts`:

```ts
import { createWorkbookFromTemplate } from '$lib/features/sf2/template/create';
import {
	importSf2WorkbookFromFile,
	stageImportSource,
	validateSf2WorkbookImport
} from '$lib/features/sf2/template/import';
import type { Sf2ImportValidation } from '$lib/features/sf2/validation';

/** Create a class workbook from the bundled DepEd template. */
export async function createSf2WorkbookFromTemplate(
	draft: Sf2TemplateDraft
): Promise<Sf2ImportSummary> {
	return await createWorkbookFromTemplate(draft);
}

/** One workbook — null when the teacher dismissed the picker, never an error. */
export async function pickImportWorkbookFile(): Promise<string | null> {
	const { pickWorkbookFile } = await import('./pickers');
	return await pickWorkbookFile();
}

/** Copy a picked file in-scope (see `stageImportSource`). */
export async function stageImportWorkbook(pickedPath: string): Promise<string> {
	return await stageImportSource(pickedPath);
}

/** The validation report for the import dialog; writes nothing. */
export async function validateSf2WorkbookImportFile(
	stagedPath: string
): Promise<Sf2ImportValidation> {
	return await validateSf2WorkbookImport(stagedPath);
}

/** Adopt a staged workbook after (explicit-proceed) validation. */
export async function importSf2Workbook(
	stagedPath: string,
	proceedAnyway: boolean
): Promise<Sf2ImportSummary> {
	return await importSf2WorkbookFromFile(stagedPath, proceedAnyway);
}
```

`Sf2TemplateDraft`, `Sf2ImportSummary` types are already imported in `sf2.ts:49-56`.

- [ ] **Step 3: Run checks**

Run: `bun run check`
Expected: 0 errors. Then run `bunx vitest run src/lib/api/__tests__/barrel.test.ts` — if it enumerates the export list, add the five new names to its expectation (that edit is the "test" for this task).

### Task 3: Settings state — draft, validation state, handlers

**Files:**

- Modify: `src/routes/settings/sf2-state.svelte.ts` (233 lines — after this task re-read the file; if it approaches 400, move the create-draft helpers to `sf2-create-draft.svelte.ts` in the same folder)

**Interfaces:**

- Consumes: Task 2 API (`createSf2WorkbookFromTemplate`, `pickImportWorkbookFile`, `stageImportWorkbook`, `validateSf2WorkbookImportFile`, `importSf2Workbook`, `syncSf2Roster`, `getSf2LaunchMonth` unaffected); `listClasses` from `$lib/features/settings/native` (already used by `settings-state.svelte.ts:2` — same import path works here); type `Sf2ImportValidation` from `$lib/features/sf2/validation`; types `Sf2TemplateDraft`, `Sf2ImportSummary` from `$lib/types`.
- Produces (fields/methods on `Sf2State` consumed by Task 4): `availableClasses: {id,name}[]`; `createDraft` (all `Sf2TemplateDraft` string fields as `$state`, `learnerNamesText` textarea backing); `importReview: { stagedPath, validation } | null`; `importDialogOpen`; busy flags `creating`, `staging`, `importing`, `syncing`; `selectedClassId`; methods `onCreateFromTemplate()`, `onPickImportFile()`, `onConfirmImport(proceed: boolean)`, `onCancelImport()`, `onSyncRoster()`.

- [ ] **Step 1: Write the failing test** — UI state is exercised through the dialog/section render; the proving test lands in Task 4. For this task, RED is a `svelte-check` error in `sf2-section.svelte` referencing the not-yet-existing fields — proceed to implementation (same one-line-wrapper reasoning as Task 2 does not apply here; the real coverage is Task 4's render test plus `bun run check`).
- [ ] **Step 2: Implement** — add to `Sf2State`:
  - `availableClasses = $state<{ id: string; name: string }[]>([])`, loaded in `load()` via `Promise.all` addition: `listClasses()` (failure → toast, empty list; never blocks the other reads).
  - `createDraft` fields initialized `''` (and `learnerNamesText = $state('')`); `selectedClassId = $state('')`.
  - `onCreateFromTemplate()`: guard `creating`; build `Sf2TemplateDraft` from fields (`learnerNames: learnerNamesText.split('\n')`, `classId: selectedClassId || undefined`); `await createSf2WorkbookFromTemplate(draft)`; toast `Workbook created for ${summary.className}: ${summary.learnersFound} learners, ${summary.datesMapped} days mapped.`; clear `learnerNamesText`; `await load()`.
  - `onPickImportFile()`: guard `staging`; `picked = await pickImportWorkbookFile()`; `if (picked === null) return`; `staged = await stageImportWorkbook(picked)`; `validation = await validateSf2WorkbookImportFile(staged)`; set `importReview = { stagedPath: staged, validation }`, `importDialogOpen = true`. Any throw → `ctx.toast(msg, false)`.
  - `onConfirmImport(proceed)`: guard `importing`; `if (!importReview) return`; `summary = await importSf2Workbook(importReview.stagedPath, proceed)`; close dialog, null the review; toast created-summary; `await load()`. Throw → toast, keep dialog open so the teacher can cancel instead.
  - `onCancelImport()`: close dialog, null the review (staged file stays; next stage sweeps it).
  - `onSyncRoster()`: guard `syncing`; resolve class: `selectedClassId` else the single available class else toast `'Choose a class first.'` and return; `await syncSf2Roster(classId)`; toast `'Roster synced.'`; `await loadMonths()`.
  - All catches use the existing private `errorMessage(error, fallback)` helper.
- [ ] **Step 3: Run checks**

Run: `bun run check`
Expected: 0 errors.

### Task 4: Settings UI — buttons, create form, validation dialog

**Files:**

- Create: `src/routes/settings/sf2-import-dialog.svelte`
- Modify: `src/routes/settings/sf2-section.svelte`
- Test: `src/routes/settings/sf2-import-dialog.test.ts` (render test asserting counts/mismatches render and both buttons fire)

**Interfaces:**

- Consumes: `sf2State` singleton fields/methods from Task 3; `Dialog` primitive (`open`, `title`, `maxWidth`, `onClose`, `children` snippet); `Sf2ImportValidation` shape (`validation.ts:50-63`).
- Produces: rendered entry points with labels `Create from template`, `Import workbook`, `Sync roster` (never the retired phrase).

- [ ] **Step 1: Write the failing test** — `src/routes/settings/sf2-import-dialog.test.ts`. Follow the existing `branding-section.test.ts` render pattern in the same folder (read it first for the exact render harness). Minimal:

```ts
test('shows mismatch counts and offers explicit proceed', () => {
	const { getByText, getByRole } = render(Sf2ImportDialog, {
		props: {
			open: true,
			validation: sampleValidation(),
			busy: false,
			onProceed: () => {},
			onCancel: () => {}
		}
	});
	expect(getByText(/2 missing from workbook/)).toBeTruthy();
	expect(getByRole('button', { name: /import anyway/i })).toBeTruthy();
	expect(getByRole('button', { name: /cancel/i })).toBeTruthy();
});
```

`sampleValidation()`: a literal `Sf2ImportValidation` with `missingFromSf2` of length 2, everything else empty, `hasDiscrepancies: true`. Note: rendering a `.svelte` component in Vitest needs the jsdom setup already in `vitest.config.ts` — mirror `branding-section.test.ts` exactly.

- [ ] **Step 2: Run it to verify it fails**

Run: `bunx vitest run src/routes/settings/sf2-import-dialog.test.ts`
Expected: FAIL with "Cannot find module './sf2-import-dialog.svelte'".

- [ ] **Step 3: Write minimal implementation** — `sf2-import-dialog.svelte`:

```svelte
<script lang="ts">
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import type { Sf2ImportValidation } from '$lib/features/sf2/validation';
	import Spinner from '$lib/components/ui/Spinner.svelte';

	interface Props {
		open: boolean;
		validation: Sf2ImportValidation | null;
		busy: boolean;
		onProceed: () => void;
		onCancel: () => void;
	}
	let { open, validation, busy, onProceed, onCancel }: Props = $props();
</script>

<Dialog
	{open}
	title="Import workbook"
	description="Review disagreements before the workbook becomes the class record."
	maxWidth="lg"
	onClose={onCancel}
>
	{#if validation}
		<div class="space-y-3 text-sm">
			<p>
				{validation.currentStudentCount} students on record, {validation.sf2LearnerCount} learners in
				the workbook.
			</p>
			{#if validation.missingFromSf2.length > 0}
				<p>
					{validation.missingFromSf2.length} missing from workbook: {validation.missingFromSf2
						.slice(0, 5)
						.map((s) => s.name)
						.join('; ')}{validation.missingFromSf2.length > 5 ? '…' : ''}
				</p>
			{/if}
			{#if validation.missingFromCurrent.length > 0}
				<p>
					{validation.missingFromCurrent.length} new in workbook: {validation.missingFromCurrent
						.slice(0, 5)
						.map((l) => l.name)
						.join('; ')}{validation.missingFromCurrent.length > 5 ? '…' : ''}
				</p>
			{/if}
			{#if validation.possibleNameMismatches.length > 0}
				<p>
					{validation.possibleNameMismatches.length} possible name mismatches — check spelling in Excel
					before proceeding.
				</p>
			{/if}
			{#if !validation.hasDiscrepancies}
				<p>No disagreements. Importing adopts the workbook's roster as-is.</p>
			{/if}
			<div class="flex justify-end gap-3">
				<button type="button" onclick={onCancel} disabled={busy} class="...">Cancel</button>
				<button type="button" onclick={onProceed} disabled={busy} class="..."
					>{#if busy}<Spinner />{/if}
					{busy ? 'Importing…' : 'Import anyway'}</button
				>
			</div>
		</div>
	{/if}
</Dialog>
```

(Use the same button classes as `sf2-section.svelte:47-52`; `...` above means copy those exact class strings.)

- [ ] **Step 4: Wire the section** — in `sf2-section.svelte`, above the month-workbooks block, add the entry block: three buttons (`Create from template` toggles the inline form; `Import workbook` → `sf2State.onPickImportFile()`; `Sync roster` → `sf2State.onSyncRoster()`), the inline create form (text inputs bound to `sf2State.createDraft.*`, class `<select>` bound to `sf2State.selectedClassId` over `sf2State.availableClasses`, learners `<textarea>` bound to `sf2State.learnerNamesText`, submit → `sf2State.onCreateFromTemplate()`), and `<Sf2ImportDialog open={sf2State.importDialogOpen} validation={sf2State.importReview?.validation ?? null} busy={sf2State.importing} onProceed={() => sf2State.onConfirmImport(true)} onCancel={() => sf2State.onCancelImport()} />`. After editing, check line counts of both `sf2-section.svelte` and `sf2-state.svelte.ts`: if either exceeds 400 lines, split before proceeding (form → `sf2-create-form.svelte`, no new state file).
- [ ] **Step 5: Run tests and checks**

Run: `bunx vitest run src/routes/settings/sf2-import-dialog.test.ts`, then `bun run check`, then `bun run lint`
Expected: PASS, 0 errors, lint clean (fix with `bun run format` on the touched files if needed).

### Task 5: Delete the void guard test, full verification

**Files:**

- Delete: `src/routes/reports/import-control-removed.test.ts`

**Interfaces:** none (removal only).

- [ ] **Step 1: Delete the file** — its premise ("the recovery control does not exist") is void by design; the Reports route itself is untouched, so no replacement coverage is needed there. New labels were chosen to never contain the retired phrase.
- [ ] **Step 2: Run the whole suite**

Run: `bun run test`
Expected: green (excluding the wall-clock perf test, which is not part of `bun run test` by design).

- [ ] **Step 3: Run the pre-PR checks**

Run: `bun run check`, `bun run lint`
Expected: 0 errors, clean. Confirm diff contains no `as any` / `@ts-ignore` and no touched route/state file exceeds 400 lines.

## Self-Review

- Spec coverage: §3 API+import module → Tasks 1–2; §3 state/UI/dialog → Tasks 3–4; §5 flows → Tasks 1 (import) + reuse (create/sync already exist); §6 errors → Task 1 messages + Task 3 toasts; §7 testing → Task 1/4 tests + Task 5 suite; §8 out-of-scope respected (no `.xls` parsing — refusal only; no month writes; Reports Import-X stays removed — reports route untouched; guard test deleted per spec §4 table).
- Placeholder scan: no TBD/TODO; every step names exact files, functions, commands, expected outputs.
- Type consistency: `Sf2ImportSummary` (create/update/import return), `Sf2ImportValidation` (validate → dialog → confirm), `WorkbookLearnerSync.studentMappings` count as `learnersFound` — matches existing shapes.
