<script lang="ts">
	type Props = {
		active?: boolean;
		title: string;
		description?: string;
		simple?: boolean;
	};

	let { active = false, title, description, simple = false }: Props = $props();
</script>

{#if active}
	<div
		class="rounded-xl border border-primary/25 bg-primary/10 p-4"
		role="status"
		aria-live="polite"
	>
		<div class="flex items-start gap-3">
			<span
				class="mt-0.5 grid size-9 shrink-0 place-items-center rounded-lg border border-primary/30 bg-background text-primary"
				aria-hidden="true"
			>
				{#if simple}
					<span class="size-2 rounded-full bg-primary"></span>
				{:else}
					<span class="size-4 animate-spin rounded-full border-2 border-primary/20 border-t-primary"
					></span>
				{/if}
			</span>

			<div class="min-w-0 flex-1">
				<div class="flex items-center justify-between gap-3">
					<div class="text-sm font-semibold text-foreground">{title}</div>
					<div class="label-mono text-[10px] text-primary">
						{simple ? 'In progress' : 'Working'}
					</div>
				</div>

				{#if description}
					<p class="mt-1 text-xs leading-5 text-muted-foreground">{description}</p>
				{/if}

				{#if simple}
					<div
						class="mt-3 h-2 rounded-pill border border-primary/20 bg-background"
						role="progressbar"
						aria-valuetext={title}
					>
						<div class="h-full w-2/5 rounded-pill bg-primary"></div>
					</div>
				{:else}
					<div
						class="mt-3 h-2 overflow-hidden rounded-pill border border-primary/20 bg-background"
						role="progressbar"
						aria-valuemin="0"
						aria-valuemax="100"
						aria-valuetext={title}
					>
						<div class="task-progress-fill h-full w-1/3 rounded-pill bg-primary"></div>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.task-progress-fill {
		animation: progress-slide 1.2s ease-in-out infinite;
	}

	@keyframes progress-slide {
		0% {
			transform: translateX(-120%);
		}
		50% {
			transform: translateX(110%);
		}
		100% {
			transform: translateX(330%);
		}
	}
</style>
