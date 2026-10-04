# SF2 Import-Restore Design — 2026-10-03

## Status

Approved design (Approach A). No implementation until this spec is reviewed.

## 1. Problem

The three Settings → SF2 workflows (Create From Template, Import SF2,
Sync Roster) plus the Import X recovery button were removed per D18 and the
month-split spec §12. Users report the import workbook feature is missing:
a teacher with a school workbook from outside the app, or a fresh class
with no workbook, has no entry point. The per-month model writes month
files itself but never ingests an external file.

## 2. Decision

Restore all three Settings → SF2 flows as thin TypeScript UI over the
already-ported TS backend (Approach A). No Rust, no new Excel engine.
`createWorkbookFromTemplate` (`src/lib/features/sf2/template/create.ts:89`),
`updateWorkbookSettings` (`template/update.ts:80`),
`syncWorkbookRosterForClass` (`roster/sync.ts:19`),
`importValidationFromAnalysis` (`validation-service.ts:34`), and
`importAbsentMarks` (`attendance/attendance-import.ts:110`) are reused
unchanged. The only new backend is one `importSf2WorkbookFromFile`
composition function.

## 3. Architecture

- `src/lib/api/sf2.ts` gains two exports: `createSf2WorkbookFromTemplate`
  and `importSf2WorkbookFromFile`. `syncSf2Roster` already exists and is
  reused. Route code imports only from `$lib/api`.
- `src/lib/features/sf2/template/import.ts` (new, one file): copy picked
  file → `openWorkbook` → `readWorkbookAnalysis` → `importValidationFromAnalysis`
  → `upsertTemplateWithMappings`. No Excel process; one in-memory workbook,
  one `saveWorkbookAtomic`.
- `src/routes/settings/sf2-state.svelte.ts`: `draft` fields plus
  `onCreateFromTemplate()`, `onPickImportFile()` / `onConfirmImport(proceed)`,
  `onSyncRoster()`. Errors surface via `ctx.toast(msg, false)`; nothing swallowed.
- `src/routes/settings/sf2-section.svelte`: one "Start / Import / Sync" block
  above the month table; restored `Sf2ImportValidationDialog` (mismatch list,
  Proceed / Cancel).
- Picker: `pickWorkbookFile` (`src/lib/api/pickers.ts:40`) with dynamic
  `plugin-dialog` import; filters `['xlsx', 'xlsm']` only. `.xls` is refused
  with "Open once in Excel, Save As .xlsx, then import" (AGENTS §9).

## 4. Components

| Piece              | File                                                | Change                                                                   |
| ------------------ | --------------------------------------------------- | ------------------------------------------------------------------------ |
| API surface        | `src/lib/api/sf2.ts`                                | Add create + import-from-file wrappers                                   |
| Import composition | `src/lib/features/sf2/template/import.ts`           | New; copy → analyze → validate → upsert                                  |
| Settings state     | `src/routes/settings/sf2-state.svelte.ts`           | Draft + three handlers + validation state                                |
| Settings UI        | `src/routes/settings/sf2-section.svelte`            | Buttons + validation dialog                                              |
| Removal-guard test | `src/routes/reports/import-control-removed.test.ts` | Rewrite to assert the three controls exist (invert of current assertion) |

## 5. Data flow

- Create: form draft → `metadataFromDraft` → `writeBundledTemplateToDir` →
  in-memory writes (metadata, calendar, roster, formulas with cached values) →
  single `saveWorkbookAtomic` → `upsertTemplateWithMappings`.
- Import: picker → atomic copy into `workbooks/_legacy/` → analysis →
  validation report → `ensureImportValidationAllows` (`validation.ts:159`)
  throws on mismatch → dialog; explicit Proceed writes mappings.
- Sync: `latestTemplateForClass` → `templateOwnsRoster` branch
  (`syncBundledTemplateRoster` re-assigns and grows; `syncImportedWorkbookRoster`
  only fills free rows, refuses when outgrown) → single `saveWorkbookAtomic`.
- Month files are never written by import. Import lands in the pre-split store;
  `runSf2WorkbookSplit` stays the only writer of month files.

## 6. Error handling

- Picker dismiss → null, not an error; silent return.
- Missing class → `invalidInput('Selected class was not found')`.
- Duplicate create → existing message kept byte-for-byte (`errorMessage` contract
  in `$lib/db/error`).
- Missing source file → "Import the SF2 workbook again".
- Outgrown imported rows → existing "add rows in Excel, then import again" text.
- Legacy `.xls` picked → refusal message directing Save As `.xlsx`.
- All workbook writes temp + rename (`saveWorkbookAtomic` / `writeFileAtomic`).

## 7. Testing

Vitest only (`bun run test`; never `bun test`). Real SQL via `NodeSqlDriver`,
fake disk via `MemoryFileSystem`, real template bytes via `template-fixture`
`loadTemplate()`. One check per rule: create writes verifiable workbook;
mismatch import blocked without explicit proceed; sync re-run is a no-op;
dismissed picker changes nothing. `bun run check` zero errors, no `as any`.

## 8. Out of scope

- No `.xls` parsing. No COM / Excel automation revival. No cloud sync.
- No writes to month worksheets from import. No rewording of `errorMessage()`.
- No second data store; OPFS SQLite only.
- The Reports "Import X from Workbook" button stays removed: startup self-heal
  (§8.2 of the month-split spec) already recovers those marks automatically.
