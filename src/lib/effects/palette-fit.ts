import type { Curve, Effect } from 'ditherette';
import type { CropRect } from '$lib/processing/types';

/**
 * The inputs a palette fit analysed: the selected palette (name, enabled colours, and the
 * colours themselves), the crop, and the steps before it. `before` keeps every layer so a
 * restore re-adds disabled ones; the fingerprint reads only the enabled steps. Elements need
 * only carry their step: callers pass layers, tests pass plain holders.
 */
export interface FitInputs<B extends { readonly step: Effect } = { readonly step: Effect }> {
	readonly paletteName: string;
	/** Colour key -> enabled, for the selected palette's colours. */
	readonly enabled: Readonly<Record<string, boolean>>;
	/** Colour key -> its RGB value (or null for transparent), so colour edits are seen too. */
	readonly values: Readonly<Record<string, { r: number; g: number; b: number } | null>>;
	readonly crop: CropRect | undefined;
	readonly before: readonly B[];
}

/** Layer names and other non-semantic state never read by analysis stay out of the fingerprint. */
export function fitFingerprint(inputs: FitInputs): string {
	return JSON.stringify([
		inputs.paletteName,
		inputs.enabled,
		inputs.values,
		inputs.crop ?? null,
		inputs.before.filter((layer) => layer.step.enabled !== false).map((layer) => layer.step)
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
 * locked. Changed inputs re-analyse: the curves unlock (`null`) and, when the change is one a
 * restore can undo, a snapshot keeps the previous inputs and the edited curves for Revert. A
 * colour edit inside the same palette is not revertable, so it records no snapshot. After
 * re-analysis the fit is unlocked; the next input change drops any offer.
 */
export function decideLockedFit<B extends { readonly step: Effect }>(
	curves: readonly Curve[],
	tracked: FitState<B>,
	inputs: FitInputs<B>
): FitDecision<B> {
	if (fitFingerprint(tracked.inputs) === fitFingerprint(inputs))
		return { action: 'unchanged', state: tracked };
	// Revert can restore the selection, flags, crop, and steps, but not edited colour values.
	const revertable =
		tracked.inputs.paletteName !== inputs.paletteName ||
		JSON.stringify(tracked.inputs.values) === JSON.stringify(inputs.values);
	return {
		action: 'reanalyse',
		state: {
			inputs,
			revert: revertable ? { inputs: tracked.inputs, curves } : undefined
		}
	};
}
