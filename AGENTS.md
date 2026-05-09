activate skills when needed
always follow @DESIGN.md and /reference when generating UI
we start by converrting typescript (/reference) into svelte
always run bun run check, lint, typecheck / cargo check, clippy every after implimentaition done to ensure

use this Svelte patterns:
$state / $derived / $derived.by instead of useState/useEffect
{#snippet} + {@render} for the StatCard and EmptyState sub-components (snippets with args can't be used as components in Svelte 5 — they must be called with {@render})
$app/stores's page store for active nav detection instead of useLocation
onMount for the async data load
