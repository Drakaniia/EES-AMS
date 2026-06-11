<script lang="ts">
	type Props = {
		class?: string;
	};

	let { class: className = '' }: Props = $props();
</script>

<span
	class={`spinner inline-block size-4 shrink-0 rounded-full border-2 border-current border-t-transparent ${className}`}
	aria-hidden="true"
></span>

<style>
	.spinner {
		animation: button-spinner-rotate 0.75s linear infinite;
	}

	@keyframes button-spinner-rotate {
		to {
			transform: rotate(360deg);
		}
	}
</style>
