<script lang="ts">
	/**
	 * macOS-style overlay scrollbars for the SF2 grid, both axes.
	 *
	 * The grid scroller hides its native bars on purpose: native bars belong
	 * to the OS, and under overlay-scrollbar settings (or the old -12px clip
	 * trick misfiring) the vertical bar can vanish with no recourse. These
	 * thumbs are driven by the scroller's own live metrics, so they render
	 * exactly when there is overflow - and they take no layout space.
	 *
	 * Perf contract (same as the grid's): reads happen in the ResizeObserver
	 * / initial layout only. Scroll events write `transform` straight onto
	 * the thumbs - no reactive re-render of the 1500-cell table per frame.
	 * Dragging tracks the pointer 1:1 from the grab point (no jumping), with
	 * window-level move/up listeners so a fast drag never loses the thumb.
	 */
	const HIDE_AFTER_MS = 1200;
	const MIN_THUMB_PX = 28;
	/** Inset the thumb travels in on each side (matches the CSS offsets). */
	const TRACK_INSET_PX = 6;

	let { target }: { target: HTMLElement | undefined } = $props();

	let vThumb: HTMLDivElement | undefined = $state(undefined);
	let hThumb: HTMLDivElement | undefined = $state(undefined);
	let canV = $state(false);
	let canH = $state(false);
	let active = $state(false);

	let vLen = 0;
	let hLen = 0;
	let dragging = false;
	let hovering = false;
	let hideTimer: ReturnType<typeof setTimeout> | null = null;

	function travelOf(el: HTMLElement, vertical: boolean): number {
		return (vertical ? el.clientHeight : el.clientWidth) - TRACK_INSET_PX * 2;
	}

	function layout() {
		const el = target;
		if (!el || !vThumb || !hThumb) return;
		canV = el.scrollHeight > el.clientHeight + 2;
		canH = el.scrollWidth > el.clientWidth + 2;
		vLen = canV
			? Math.max((travelOf(el, true) * el.clientHeight) / el.scrollHeight, MIN_THUMB_PX)
			: 0;
		hLen = canH
			? Math.max((travelOf(el, false) * el.clientWidth) / el.scrollWidth, MIN_THUMB_PX)
			: 0;
		vThumb.style.height = `${vLen}px`;
		hThumb.style.width = `${hLen}px`;
		position(el.scrollTop, el.scrollLeft);
	}

	function position(top: number, left: number) {
		const el = target;
		if (!el || !vThumb || !hThumb) return;
		if (canV) {
			const max = el.scrollHeight - el.clientHeight;
			const y = max > 0 ? (Math.min(Math.max(top, 0), max) / max) * (travelOf(el, true) - vLen) : 0;
			vThumb.style.transform = `translateY(${y}px)`;
		}
		if (canH) {
			const max = el.scrollWidth - el.clientWidth;
			const x = max > 0 ? (Math.min(Math.max(left, 0), max) / max) * (travelOf(el, false) - hLen) : 0;
			hThumb.style.transform = `translateX(${x}px)`;
		}
	}

	function poke() {
		active = true;
		if (hideTimer) clearTimeout(hideTimer);
		hideTimer = setTimeout(() => {
			hideTimer = null;
			if (!dragging && !hovering) active = false;
		}, HIDE_AFTER_MS);
	}

	function onScroll() {
		const el = target;
		if (!el) return;
		position(el.scrollTop, el.scrollLeft);
		poke();
	}

	function onThumbDown(vertical: boolean, event: PointerEvent) {
		const el = target;
		const thumb = vertical ? vThumb : hThumb;
		if (!el || !thumb) return;
		event.preventDefault();
		dragging = true;
		poke();
		const startPointer = vertical ? event.clientY : event.clientX;
		const startScroll = vertical ? el.scrollTop : el.scrollLeft;
		const travel = travelOf(el, vertical) - (vertical ? vLen : hLen);
		const maxScroll = vertical
			? el.scrollHeight - el.clientHeight
			: el.scrollWidth - el.clientWidth;
		const ratio = travel > 0 ? maxScroll / travel : 0;
		const onMove = (ev: PointerEvent) => {
			const delta = (vertical ? ev.clientY : ev.clientX) - startPointer;
			if (vertical) el.scrollTop = startScroll + delta * ratio;
			else el.scrollLeft = startScroll + delta * ratio;
		};
		const onUp = () => {
			window.removeEventListener('pointermove', onMove);
			window.removeEventListener('pointerup', onUp);
			window.removeEventListener('pointercancel', onUp);
			dragging = false;
			poke();
		};
		window.addEventListener('pointermove', onMove);
		window.addEventListener('pointerup', onUp);
		window.addEventListener('pointercancel', onUp);
	}

	$effect(() => {
		const el = target;
		if (!el) return;
		const onEnter = () => {
			hovering = true;
			poke();
		};
		const onLeave = () => {
			hovering = false;
			poke();
		};
		const ro = new ResizeObserver(layout);
		ro.observe(el);
		window.addEventListener('resize', layout, { passive: true });
		el.addEventListener('scroll', onScroll, { passive: true });
		el.addEventListener('pointerenter', onEnter);
		el.addEventListener('pointerleave', onLeave);
		const frame = requestAnimationFrame(layout);
		return () => {
			cancelAnimationFrame(frame);
			ro.disconnect();
			window.removeEventListener('resize', layout);
			el.removeEventListener('scroll', onScroll);
			el.removeEventListener('pointerenter', onEnter);
			el.removeEventListener('pointerleave', onLeave);
			if (hideTimer) clearTimeout(hideTimer);
			hideTimer = null;
		};
	});
</script>

<div class="mac-track mac-track-v" class:mac-on={active && canV}>
	<div
		bind:this={vThumb}
		class="mac-thumb"
		role="scrollbar"
		aria-orientation="vertical"
		aria-label="Scroll grid vertically"
		onpointerdown={(event) => onThumbDown(true, event)}
	></div>
</div>
<div class="mac-track mac-track-h" class:mac-on={active && canH}>
	<div
		bind:this={hThumb}
		class="mac-thumb"
		role="scrollbar"
		aria-orientation="horizontal"
		aria-label="Scroll grid horizontally"
		onpointerdown={(event) => onThumbDown(false, event)}
	></div>
</div>

<style>
	.mac-track {
		position: absolute;
		z-index: 40;
		opacity: 0;
		visibility: hidden;
		transition:
			opacity 150ms ease-out,
			visibility 0s linear 150ms;
		pointer-events: none;
	}
	.mac-track-v {
		top: 6px;
		right: 3px;
		bottom: 6px;
		width: 8px;
	}
	.mac-track-h {
		right: 6px;
		bottom: 3px;
		left: 6px;
		height: 8px;
	}
	.mac-on {
		opacity: 1;
		visibility: visible;
		transition:
			opacity 150ms ease-out,
			visibility 0s;
	}
	.mac-thumb {
		border-radius: 999px;
		background-color: color-mix(in oklab, var(--color-foreground) 38%, transparent);
		pointer-events: auto;
		touch-action: none;
	}
	.mac-track-v .mac-thumb {
		width: 8px;
	}
	.mac-track-h .mac-thumb {
		height: 8px;
	}
	.mac-thumb:hover {
		background-color: color-mix(in oklab, var(--color-foreground) 55%, transparent);
	}
	@media (prefers-reduced-motion: reduce) {
		.mac-track,
		.mac-on {
			transition: none;
		}
	}
	@media (prefers-contrast: more) {
		.mac-thumb {
			background-color: color-mix(in oklab, var(--color-foreground) 65%, transparent);
		}
	}
</style>
