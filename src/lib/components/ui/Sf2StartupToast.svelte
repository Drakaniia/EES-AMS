<script lang="ts">
	/**
	 * The one app-wide surface for what the app has to say at startup.
	 *
	 * Mounted once, in the app shell, because neither thing it carries belongs to
	 * a route: the self-heal runs on a Rust thread that started before the webview
	 * existed and reports whenever it finishes, and the E3 prompt is a question
	 * about the whole install until it is answered.
	 *
	 * A quiet notice renders through the same toast but announced politely:
	 * `FeedbackToast` treats `ok: false` as an assertive, destructive-coloured
	 * alert, and "Excel was not available so nothing was checked" is not an error
	 * the teacher did anything wrong about.
	 */
	import { onMount } from 'svelte';
	import FeedbackToast from './FeedbackToast.svelte';
	import { sf2StartupStore } from '$lib/stores/sf2-startup.svelte';

	onMount(() => {
		// Not awaited: the heal may not finish for seconds, and it may never
		// finish at all. Neither is a reason to hold up the shell.
		void sf2StartupStore.start();
	});
</script>

<FeedbackToast
	message={sf2StartupStore.message}
	ok={sf2StartupStore.ok}
	actionLabel={sf2StartupStore.actionLabel ?? undefined}
	onAction={sf2StartupStore.onAction ?? undefined}
	onClose={() => sf2StartupStore.dismiss()}
/>
