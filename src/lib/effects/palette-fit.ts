import type { Curve, Effect } from 'ditherette';
import type { CropRect } from '$lib/processing/types';

/**
 * The inputs a palette fit analysed: the selected palette (name and enabled colours), the crop,
 * and the enabled steps before it. `before` elements need only carry their step — callers pass
 * layers, tests pass plain holders.
 */
export interface FitInputs<B extends { readonly step: Effect } = { readonly step: Effect }> {
	readonly paletteName: string;
	/** Colour key -> enabled, for the selected palette's colours. */
	readonly enabled: Readonly<Record<string, boolean>>;
	readonly crop: CropRect | undefined;
	readonly before: readonly B[];
}

/** Layer names and other non-semantic state never read by analysis stay out of the fingerprint. */
export function fitFingerprint(inputs: FitInputs): string {
	return JSON.stringify([
		inputs.paletteName,
		inputs.enabled,
		inputs.crop ?? null,
		inputs.before.map((layer) => layer.step)
	]);
}

/** What Revert restores: the inputs before the change plus the curves edited under them. */
export interface FitRevert<B extends { readonly step: Effect } = { readonly step: Effect }> {
	readonly inputs: FitInputs<B>;
	readonly curves: readonly Curve[];
}

/** A locked fit's tracked state: the inputs it was edited under and one revert snapshot. */
export interface FitState<B extends { readonly step: Effect } = { readonly step: Effect }> {
	readonly inputs: FitInputs<B>;
	readonly revert?: FitRevert<B>;
}

export type FitDecision<B extends { readonly step: Effect } = { readonly step: Effect }> =
	| { readonly action: 'unchanged'; readonly state: FitState<B> }
	| { readonly action: 'reanalyse'; readonly state: FitState<B> };

/**
 * Compare a locked fit's inputs now with those it was edited under. Unchanged inputs keep it
 * locked. Changed inputs re-analyse: the curves unlock (`null`) and one revert snapshot keeps the
 * previous inputs and the edited curves. A further change replaces the snapshot — there is only
 * ever one level.
 */
export function decideLockedFit<B extends { readonly step: Effect }>(
	curves: readonly Curve[],
	tracked: FitState<B>,
	inputs: FitInputs<B>
): FitDecision<B> {
	if (fitFingerprint(tracked.inputs) === fitFingerprint(inputs))
		return { action: 'unchanged', state: tracked };
	return {
		action: 'reanalyse',
		state: { inputs, revert: { inputs: tracked.inputs, curves } }
	};
}
