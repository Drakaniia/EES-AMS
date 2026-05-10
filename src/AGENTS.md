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
- **File Organization**: Routes in `src/routes/`, components in `src/lib/components/`, utilities in `src/lib/`
- **Naming**: kebab-case for files, PascalCase for components
- **TypeScript**: Strict typing enabled, use interfaces for data models
- **Styling**: TailwindCSS utility classes, follow DESIGN.md tokens
- **Tauri Integration**: Import from `@tauri-apps/api` for native functionality

Examples with actual file paths:
- DO: Use runes pattern from `src/lib/components/AttendanceCard.svelte`
- DO: Follow route structure in `src/routes/dashboard/+page.svelte`
- DO: API calls pattern from `src/lib/api.ts`
- DON'T: Use Svelte 4 stores like `src/lib/legacy/store.ts`

## Key Files
- **Main Layout**: `src/routes/+layout.svelte`
- **API Client**: `src/lib/api.ts`
- **Database Interface**: `src/lib/db-rust.ts`
- **NFC Handling**: `src/lib/nfc.ts`
- **Type Definitions**: `src/lib/types.ts`
- **CSV Export**: `src/lib/csv.ts`
- **UI Components**: `src/lib/components/`

## JIT Index Hints
- Find component: `rg -n "export.*ComponentName" src/lib/components`
- Find page: `rg -n "export.*Page" src/routes`
- Find API calls: `rg -n "tauri\.invoke" src/lib`
- Find types: `rg -n "interface|type" src/lib/types.ts`
- Find Svelte files: `find src/ -name "*.svelte"`

## Common Gotchas
- Always use `@tauri-apps/api` for native calls, never direct DOM manipulation
- Use TailwindCSS classes from DESIGN.md tokens for consistent styling
- SvelteKit routes use `+page.svelte` for pages, `+layout.svelte` for layouts
- Tauri commands are async, always await the result

## Pre-PR Checks
```bash
bun run check && bun run lint && bun run typecheck
```
