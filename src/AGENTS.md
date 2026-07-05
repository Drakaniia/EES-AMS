# Frontend - SvelteKit 5

## Package Identity

SvelteKit 5 frontend application with TypeScript, TailwindCSS 4, and Tauri v2 integration for desktop attendance management system.

## Setup & Run

```bash
# Install dependencies
bun install

# Development mode
bun run dev

# Type checking
bun run check

# Linting and formatting
bun run lint
bun run format

# Build for production
bun run build
```

## Patterns & Conventions

- **Svelte 5 Runes**: Use `$state`, `$derived`, `$effect` instead of Svelte 4 stores
- **File Organization**: Routes in `src/routes/`, shared components in `src/lib/components/`, feature modules in `src/lib/features/`, and cross-feature utilities in `src/lib/`
- **Shared UI Contract**: `src/lib/components/ui/` is the only shared UI root. Import shared UI with `$lib/components/ui/...`; never use `src/components/ui/...` or relative `../components/ui/...` paths.
- **Feature Modules**: Put feature-specific workflow logic under `src/lib/features/<feature>/`. Settings workflows live in `src/lib/features/settings/`.
- **Naming**: kebab-case for files, PascalCase for components
- **TypeScript**: Strict typing enabled, use interfaces for data models
- **Styling**: TailwindCSS utility classes, follow docs/DESIGN.md tokens
- **Tauri Integration**: Route code should call feature native adapters such as `$lib/features/settings/native`; low-level command wrappers stay in `src/lib/db-rust.ts` unless a component has a narrow native concern.

Examples with actual file paths:

- DO: Import shared UI as `$lib/components/ui/Dialog.svelte`
- DO: Keep settings workflow helpers in `src/lib/features/settings/global-settings.ts`, `backup.ts`, `sf2-workbook.ts`, or `sf2-validation.ts`
- DO: Use the settings native adapter from `src/lib/features/settings/native.ts` in settings-focused code
- DO: Follow Svelte 5 runes patterns in `src/lib/stores/settings.svelte.ts` and `src/routes/settings/+page.svelte`
- DON'T: Add or import shared UI from `src/components/ui/`
- DON'T: Import shared UI with route-relative paths like `../components/ui/Dialog.svelte`

## Key Files

- **Main Layout**: `src/routes/+layout.svelte`
- **Database Interface**: `src/lib/db-rust.ts`
- **Settings Feature**: `src/lib/features/settings/`
- **Type Definitions**: `src/lib/types.ts`
- **CSV Export**: `src/lib/csv.ts`
- **UI Components**: `src/lib/components/`

## JIT Index Hints

- Find component: `rg -n "export.*ComponentName" src/lib/components`
- Find page: `rg -n "export.*Page" src/routes`
- Find feature modules: `rg -n "export .*" src/lib/features`
- Find native adapters: `rg -n "from '\\$lib/features/.*/native'" src/`
- Find Tauri command wrappers: `rg -n "invoke\\(" src/lib`
- Find types: `rg -n "interface|type" src/lib/types.ts`
- Find Svelte files: `find src/ -name "*.svelte"`

## Common Gotchas

- Prefer feature native adapters from route code; keep direct `@tauri-apps/api` imports inside low-level wrappers or narrowly scoped native UI components.
- Use TailwindCSS classes from docs/DESIGN.md tokens for consistent styling
- SvelteKit routes use `+page.svelte` for pages, `+layout.svelte` for layouts
- Tauri commands are async, always await the result

## Pre-PR Checks

```bash
bun run check && bun run lint && bun run typecheck
```
