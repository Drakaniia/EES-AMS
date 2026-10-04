# OPFS-Unavailable SF2 Reports — Spec

**Date:** 2026-10-04
**Source:** teacher screenshot on packaged Tauri build (`SF2 reports are unavailable` empty state + `Retry`)
**Scope:** global startup / every route that touches the DB, not Reports-only
**Goal (user-confirmed):** both correct diagnosis AND resilient fallback
**Status:** spec only — no code changes

## 1. Problem statement

Packaged app shows on Reports:

> `SF2 reports are unavailable`
> `database error: the opfs VFS is unavailable (OPFS storage is blocked on this PC); check disk space and site-storage permissions, then reopen the app [protocol=https:isolated=true sab=yes; OPFS storage probe failed: Failed to execute 'getDirectory' on 'StorageManager': Illegal invocation]`
> `[Retry]`

Evidence from the repo:

- `src/routes/reports/ReportLoadingStates.svelte:35` renders title `SF2 reports are unavailable` with `description={loadError}` and a single `Retry` button.
- `src/lib/db/worker.ts:80-107` (`describeMissingOpfs`) picks the static reason; `:106` is the `OPFS storage is blocked` branch — the fallthrough when SAB exists, FileSystem sync-handle APIs exist, and `crossOriginIsolated` is true.
- `src/lib/db/worker.ts:147-167` (`probeOpfsStorage`) appends the live probe result via `diagnoseOpfsFailure` (`:170-177`).
- `src/lib/db/error.ts:26` prefixes everything with `database error: {detail}`; `AGENTS.md §6.1/§6.3` calls these strings a byte-for-byte UI contract — **user explicitly waives this for this fix: rewording is allowed.**
- Release headers are already set: `src-tauri/tauri.conf.json:29-32` (`COOP: same-origin`, `COEP: require-corp`) + `useHttpsScheme: true`, matching the screenshot's `protocol=https:isolated=true sab=yes`.

### 1.1 Root-cause hypothesis (to confirm in implementation)

`probeOpfsStorage` detaches the method then calls it unbound:

```ts
const getDirectory = nav?.storage?.getDirectory; // unbound
const root = (await (getDirectory as () => Promise<ProbeRoot>)()) as ProbeRoot;
```

`navigator.storage.getDirectory` requires its `navigator.storage` receiver; calling it detached throws exactly `Failed to execute 'getDirectory' on 'StorageManager': Illegal invocation` **even when OPFS storage is healthy**. Consequences:

1. The probe always fails in the worker, so the message always blames "blocked storage" even when the real cause is (a) outdated WebView2 without sync-access handles, (b) SQLite proxy/VFS install failure, or (c) genuinely blocked storage.
2. The static branch (`describeMissingOpfs` fallthrough) and the probe failure compound: two guesses presented as one fact.
3. `Retry` reuses the `WorkerSqlDriver` singleton (`src/lib/db/client.ts:47-59` drops the cached rejection so retry re-opens, good) but the UI only retries the same failing open with the same misleading text — no fallback, no recovery steps, no auto-retry.

Fixing the binding (`nav.storage.getDirectory()` with receiver, or `.call(nav.storage)`) is **in scope (user-confirmed)** and is expected to change the diagnosis on most PCs from "blocked" to either "probe passed → proxy/headers" or a true storage error.

## 2. Goals / non-goals

### Goals

1. Correct diagnosis: bound probe + accurate cause taxonomy (WebView2 too old vs. headers/proxy vs. truly blocked storage vs. quota).
2. Resilient startup: when OPFS open fails in the packaged app, offer and enter **temporary in-memory DB mode** (user-chosen fallback) with a **global banner** on every page.
3. Friendly + technical error layers: plain sentence by default, expandable technical details (`protocol / isolated / sab / probe / WebView2 hint`).
4. Auto-retry: silent re-probe on window focus/reopen in addition to manual Retry.
5. In-app recovery steps for WebView2 update and site-storage permission/disk-space checks (no bare external link).
6. Allow zip backup restore **into** memory mode so a teacher is not stuck view-only.
7. Rewrite the `database error:` detail text for this path (contract waiver applies to this path only).

### Non-goals

- No Tauri-filesystem persistent SQLite fallback in this spec (user chose memory fallback).
- No cloud sync, no background service, no keyring/OAuth (per AGENTS.md §6.5).
- No change to the v1→v25 migration chain semantics, formula text, or workbook layout.
- No Excel COM automation (gone per AGENTS.md §10).

## 3. UX spec

### 3.1 Where it applies

Global startup gate, not Reports-only. Every route that calls `getDriver()` (`students`, `attendance`, `reports`, `settings`, `records`, backup scheduling in `bootstrapApp`) funnels through the same `WorkerSqlDriver.open()` failure. The Reports empty state keeps its title for deep-links, but the canonical recovery UI lives at the app shell level so a teacher blocked on launch sees it regardless of route.

### 3.2 New unavailable-DB screen (replaces bare Retry)

Keep `ReportLoadingStates.svelte` title `SF2 reports are unavailable` for the Reports route, but the global gate shows:

- **Headline (friendly):** e.g. `Attendance data can't be opened yet` / Reports keeps its existing title.
- **Plain sentence:** what happened + what it means for data, e.g. `The app couldn't open its on-device database, so reports aren't available. Your workbooks and backups on disk are untouched.`
- **Cause-specific next step (one of):**
  - WebView2 too old → `Update Microsoft Edge WebView2 Runtime to the latest version, then reopen the app.` + in-app steps (see §3.4).
  - Headers/proxy (probe passed, `OpfsDb` missing) → `This looks like an app setup issue, not your PC. Reopen the app; if it persists, report the details below.`
  - Blocked storage / quota → `Check disk space and site-storage permissions, then reopen.`
- **Actions row:**
  - `Retry` (manual re-open, same as today).
  - `Continue without saving (temporary)` — enters memory mode. Must confirm data-loss implication.
  - `Restore from backup…` — enabled in and out of memory mode; in memory mode it imports the zip's `db.sqlite` image into the memory DB.
- **Expandable `Details`:** the full technical string (rewritten detail + `[protocol=… isolated=… sab=…; probe=…; cause=…]`). Copyable.

### 3.3 Memory mode

- Entered only by explicit teacher action (`Continue without saving`), never silently.
- Banner: **global, on every page** (user-confirmed) — e.g. `Temporary mode — changes won't persist after close. [Retry database] [Learn more]`.
- `Retry database` in the banner re-attempts the OPFS open; on success, offer to export/snapshot memory contents first if non-empty (CSV/SQL readable export — never silently discard).
- Writes work normally against the memory DB so attendance/SF2 preview logic can be exercised; exports (CSV/JSON/workbook) work; scheduled backup zips are disabled or clearly labeled `temporary session — not backed up` (decide in implementation; default: disable schedule, keep manual export).
- Restore-from-backup into memory mode: reuse existing `import` path semantics (`OpfsDb.importDb` for OPFS; memory driver seed for memory mode) + post-import `user_version` forward-migration (same rule as `client.ts:171-173`).

### 3.4 In-app recovery steps (WebView2 / storage)

Spec requires inline steps, not just a link:

- WebView2: how to check installed version, where the evergreen installer lives, that a reopen is required after update.
- Site storage: how to confirm disk space, that private/cleared-site-data modes block OPFS, how to allow site data for the app origin (packaged origin is the Tauri asset protocol — steps must be written for the Tauri window, not Chrome settings verbatim).
- Exact copy TBD in implementation; spec requires the steps be tested on a real blocked PC (see §6).

### 3.5 Auto-retry

- **User-confirmed:** automatic silent retry on window focus / app reopen, in addition to manual `Retry`.
- Constraints: debounce (e.g. at most once per focus, backoff on repeated failure), no retry storm while a transaction gate is held, no masking of memory-mode banner. Log each attempt to the existing `postMessage` error path, not `console` only.

## 4. Functional requirements

1. **Probe binding fix** (`src/lib/db/worker.ts:149-153`): call `getDirectory` bound to `navigator.storage`; same for `getFileHandle`/`createSyncAccessHandle` if detached anywhere. Probe must return `null` (pass) on a healthy packaged build.
2. **Cause taxonomy:** at minimum distinguish (a) SAB missing, (b) sync-access APIs missing (WebView2 old), (c) not isolated, (d) probe-passed-but-`OpfsDb`-missing (proxy/headers), (e) probe-failed (true storage error incl. quota). Each maps to a distinct detail string + next-step block. No fallthrough may claim "blocked" when the probe passed.
3. **Error text rewrite allowed** for this path: new `detail` strings; keep `errorMessage()` template shape (`database error: {detail}`) unless the implementation justifies changing the prefix too — waiver covers both.
4. **Driver seam:** `getDriver()/useDriver()` (`src/lib/db/index.ts`) must support injecting or switching to the memory driver at runtime without touching repos (repos take no driver parameter per AGENTS.md §6.3). `NodeSqlDriver` is tests-only and never bundled — memory fallback needs a runtime-safe equivalent (new module or WASM-backed memory DB, to be designed in implementation; do not bundle `node:sqlite`).
5. **Retry semantics preserved:** failed open/migration must not poison the singleton (`client.ts:48-59`, `:82-87` behavior kept).
6. **Restore into memory:** backup `export`/`import` contract (`sqlite3_serialize` image) works for both OPFS and memory drivers; restore re-runs migrations forward.
7. **No silent data loss:** entering/exiting memory mode requires explicit confirm when the memory DB is non-empty; exiting via successful OPFS re-open offers export first.
8. **Telemetry/logging:** every OPFS failure logs `envFacts()` + probe result + chosen cause through the worker `postMessage` error path so `describeError` surfaces it; keep `crossOriginIsolated`/`protocol`/`sab` facts.

## 5. Edge cases

- OPFS blocked on first-ever launch (no DB, no workbooks): memory mode + `Create from template` must still work; banner explains nothing will persist.
- OPFS fails after migrations partially ran: `user_version` stamp replay rule unchanged; memory DB starts from `migrate()` on empty.
- Quota exceeded mid-session (open OK, write fails later): same unavailable-DB screen on next failing op, not just at open; memory-mode offer must not destroy the OPFS file.
- WebView2 updated while app open: focus-triggered auto-retry should recover without restart where possible; otherwise prompt reopen.
- Backup zip pairs DB with workbook future (`x-count.ts` guard): restore preview in memory mode must show the same pairing warning.
- `node:sqlite` unavailable in the bundled app: memory driver must not depend on it.

## 6. Verification (user-confirmed: both automated + manual)

1. `bun run check` (svelte-check, 0 errors), `bun run lint` clean, `bun run test` green (`bunx vitest run <path>` for focused files — never `bun test`).
2. New Vitest coverage: probe binding (unbound-call regression test with a strict mock receiver), cause-taxonomy mapping table, memory-mode enter/exit + restore-import + non-poisoned retry.
3. Manual packaged-app checklist on a PC that reproduces the screenshot: (a) blocked OPFS → new screen shows correct cause + in-app steps; (b) `Retry` + focus auto-retry recover after unblock; (c) `Continue without saving` → global banner on every page; (d) restore zip into memory works; (e) details block is copyable.
4. Confirm `protocol=https isolated=true sab=yes` packaged baseline still reports probe **passed** after the binding fix.

## 7. Files likely touched (implementation phase only)

- `src/lib/db/worker.ts` — probe binding, taxonomy, `diagnoseOpfsFailure` text.
- `src/lib/db/error.ts` — rewritten detail strings for this path (waiver).
- `src/lib/db/client.ts`, `src/lib/db/index.ts` — runtime memory-driver switch, retry/auto-retry hooks.
- New runtime memory driver (not `node-driver.ts`).
- `src/lib/bootstrap.ts` — global gate wiring, focus auto-retry, quit-hook interplay with memory mode.
- `src/routes/reports/ReportLoadingStates.svelte` + app shell — two-layer message, actions, global banner.
- Backup restore preview (`x-count.ts` path) — memory-mode support.

## 8. Follow-ups left for implementation

- Exact friendly copy + in-app WebView2/storage step text (needs a real blocked-PC pass).
- Whether scheduled backups pause or label-and-continue in memory mode.
- Export-before-exit format default (SQL readable vs. CSV vs. zip snapshot).
- Auto-retry debounce/backoff constants.
