# Feature Routes - SvelteKit Pages

## Package Identity

SvelteKit route modules organized by feature: attendance, dashboard, records, reports, settings, and students for the attendance management system.

## Setup & Run

```bash
# Development (from project root)
bun run tauri dev

# Type checking routes
bun run check

# Find specific route
find src/routes -name "+page.svelte" -o -name "+layout.svelte"
```

## Patterns & Conventions

- **File-Based Routing**: Use SvelteKit's file-based routing system
- **Route Structure**: `+layout.svelte` for shared layouts, `+page.svelte` for pages
- **Load Functions**: `+page.ts` or `+layout.ts` for data loading
- **Feature Organization**: Group related routes in subdirectories
- **Data Flow**: Use Tauri commands via `src/lib/api.ts` for backend communication
- **Form Handling**: Use SvelteKit form actions for POST requests

Examples with actual file paths:

- DO: Follow page structure from `src/routes/dashboard/+page.svelte`
- DO: Use load functions like `src/routes/records/+page.ts`
- DO: Layout patterns from `src/routes/+layout.svelte`
- DON'T: Hardcode API calls directly in components
- DON'T: Mix business logic with presentation

## Key Routes

- **Dashboard**: `src/routes/dashboard/+page.svelte` - Main overview
- **Attendance**: `src/routes/attendance/` - Real-time attendance tracking
- **Records**: `src/routes/records/` - Historical attendance data
- **Reports**: `src/routes/reports/` - Analytics and reporting
- **Settings**: `src/routes/settings/` - System configuration
- **Students**: `src/routes/students/` - Student management

## Route Patterns

### Dashboard Routes

- Overview page with attendance statistics
- Quick actions for common tasks
- Real-time status updates

### Attendance Routes

- Live attendance tracking interface
- Card reader input integration
- Manual attendance entry
- Attendance validation

### Records Routes

- Searchable attendance history
- Date range filtering
- Export functionality (CSV)
- Pagination for large datasets

### Reports Routes

- Attendance analytics
- Chart visualizations
- Report generation
- Export capabilities

### Settings Routes

- System configuration
- User preferences
- Database management

## JIT Index Hints

- Find page: `rg -n "export.*Page" src/routes`
- Find load function: `rg -n "export.*load" src/routes`
- Find form action: `rg -n "export.*actions" src/routes`
- Find layout: `rg -n "+layout\.svelte" src/routes`
- Find route by feature: `find src/routes/attendance -name "*.svelte"`

## Common Gotchas

- Always use `+layout.svelte` for shared UI components
- Load functions should handle errors gracefully
- Use Tauri commands via `src/lib/api.ts`, not direct imports
- Form actions should return proper redirects or responses
- Route parameters are accessed via `$params` rune in Svelte 5

## Data Loading Patterns

- Use `+page.ts` for page-specific data
- Use `+layout.ts` for shared data across routes
- Handle loading states and errors appropriately
- Cache frequently accessed data when possible

## Pre-PR Checks

```bash
# Check routes specifically
bun run check -- src/routes
bun run lint -- src/routes
```
