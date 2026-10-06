<script lang="ts">
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';

	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	export function showToast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 4000);
	}
</script>

<FeedbackToast message={toastMessage} ok={toastOk} onClose={() => (toastMessage = null)} />
