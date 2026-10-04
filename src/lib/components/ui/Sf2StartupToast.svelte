<script lang="ts">
	/**
	 * The one app-wide surface for what the app has to say at startup.
	 *
	 * Mounted once, in the app shell, because it does not belong to a route: the
	 * E3 prompt is a question about the whole install, asked until it is answered.
	 *
	 * `ok` is hard-coded true because nothing here is ever an error the teacher did
	 * something wrong about. `FeedbackToast` renders `ok: false` as an assertive,
	 * destructive-coloured alert, which is the wrong register for "tell me the date
	 * classes started".
	 */
	import { onMount } from 'svelte';
	import FeedbackToast from './FeedbackToast.svelte';
	import { sf2StartupStore } from '$lib/stores/sf2-startup.svelte';

	onMount(() => {
		// Not awaited: a launch that produces no message is a normal launch.
		void sf2StartupStore.start();
	});
</script>

<FeedbackToast
	message={sf2StartupStore.message}
	ok={true}
	actionLabel={sf2StartupStore.actionLabel ?? undefined}
	onAction={sf2StartupStore.onAction ?? undefined}
	onClose={() => sf2StartupStore.dismiss()}
/>
