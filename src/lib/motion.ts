import { animate, type AnimationPlaybackControls } from 'motion';

/**
 * Spring presets in the terms the interface actually uses.
 *
 * Apple's fluid-interface rules: motion starts from the current on-screen
 * value, inherits whatever velocity the gesture had, and stays interruptible.
 * `motion` re-targets from the live value by default, which is why every
 * enter/exit here goes through it instead of a CSS keyframe.
 */
export const SPRING = {
	/** Default: critically damped, no overshoot. Menus, panels, dialogs. */
	settle: { type: 'spring', bounce: 0, visualDuration: 0.34 },
	/** A flick or throw happened; a little bounce is earned. */
	momentum: { type: 'spring', bounce: 0.18, visualDuration: 0.32 },
	/** Sheets and the command palette use a slower, softer settle. */
	sheet: { type: 'spring', bounce: 0, visualDuration: 0.4 }
} as const;

export function prefersReducedMotion(): boolean {
	if (typeof window === 'undefined' || !window.matchMedia) return false;
	return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

type MotionOptions = Parameters<typeof animate>[2];

function options(base: MotionOptions): MotionOptions {
	if (!prefersReducedMotion()) return base;
	return { duration: 0.12, ease: 'easeOut' };
}

/**
 * Animate an element in from just off its final state.
 * Returns a cancel function so a caller can interrupt cleanly.
 */
export function enterPanel(
	element: HTMLElement,
	{
		from = { opacity: 0, scale: 0.975, y: 6 },
		spring = SPRING.settle
	}: { from?: { opacity?: number; scale?: number; y?: number }; spring?: typeof SPRING.settle } = {}
): () => void {
	const animation: AnimationPlaybackControls = animate(
		element,
		{
			opacity: [from.opacity ?? 0, 1],
			scale: [from.scale ?? 1, 1],
			y: [from.y ?? 0, 0]
		},
		options({ ...spring })
	);
	return () => animation.stop();
}

/**
 * Animate an element out. Resolves when the animation settles, so a caller can
 * hold it mounted until the exit actually finishes.
 */
export function exitPanel(
	element: HTMLElement,
	{
		to = { opacity: 0, scale: 0.98, y: 4 },
		duration = 0.16
	}: { to?: { opacity?: number; scale?: number; y?: number }; duration?: number } = {}
): Promise<void> {
	if (prefersReducedMotion()) {
		element.style.opacity = '0';
		return Promise.resolve();
	}
	const animation = animate(
		element,
		{
			opacity: [1, to.opacity ?? 0],
			scale: [1, to.scale ?? 1],
			y: [0, to.y ?? 0]
		},
		{ duration, ease: [0.4, 0, 1, 1] }
	);
	return animation.finished.then(
		() => undefined,
		() => undefined
	);
}

/** A brief, one-shot confirmation pulse. Used when a value lands in place. */
export function settlePulse(element: HTMLElement): void {
	if (prefersReducedMotion()) return;
	animate(element, { scale: [1, 1.035, 1] }, { duration: 0.32, ease: [0.22, 1, 0.36, 1] });
}

/**
 * Apple's momentum projection. The resting point is where the gesture was
 * *going*, not where it stopped — this is scroll deceleration, not physics
 * textbook v²/2a.
 */
export function projectMomentum(velocity: number, decelerationRate = 0.998): number {
	return ((velocity / 1000) * decelerationRate) / (1 - decelerationRate);
}

/**
 * Soft boundary: the further past the edge, the less the element follows.
 */
export function rubberband(overshoot: number, dimension: number, constant = 0.55): number {
	return (overshoot * dimension * constant) / (dimension + constant * Math.abs(overshoot));
}
