import { computed, onStop, type ReadableAtom } from 'nanostores';
import { compiledEffects, currentEffectsKey, effectsIndex } from '$lib/processing/live-effects';
import { colorSpace, outputSettings, selectedPalette, sourceImageData } from '$lib/stores/app';
import { activeEffectSteps } from '$lib/stores/effects';
import { MAX_SAMPLES, samplePixels, type Samples } from './scopes';

/** The images the scopes inspect, in pipeline order. */
export const STAGES = [
	{ id: 'source', label: 'Source' },
	{ id: 'effects', label: 'Effects' },
	{ id: 'output', label: 'Output' }
] as const;
export type Stage = (typeof STAGES)[number]['id'];

/** The full-colour image after the enabled effects, before resize and palette matching. */
export type EffectsImage =
	| { readonly kind: 'source'; readonly source: ImageData } // no effect on: the source itself
	| {
			readonly kind: 'effected';
			readonly width: number;
			readonly height: number;
			readonly pixels: Uint32Array;
			readonly results: Uint32Array;
	  };
export type EffectsStage = { readonly image?: EffectsImage; readonly current: boolean };

/** Same image when the references that identify it are unchanged. */
function sameImage(a: EffectsImage | undefined, b: EffectsImage | undefined): boolean {
	if (!a || !b || a.kind !== b.kind) return false;
	if (a.kind === 'source' && b.kind === 'source') return a.source === b.source;
	if (a.kind === 'effected' && b.kind === 'effected')
		return a.pixels === b.pixels && a.results === b.results;
	return false;
}

/**
 * The effects stage: each source pixel's colour through the effects worker's current results.
 * With no enabled step it is the source itself; while newer settings are still compiling it
 * keeps the older results as a dimmed stand-in, marked not current.
 */
export const effectsStage: ReadableAtom<EffectsStage | undefined> = (() => {
	let previous: EffectsStage | undefined;
	const stage = computed(
		[
			sourceImageData,
			activeEffectSteps,
			effectsIndex,
			compiledEffects,
			selectedPalette,
			colorSpace,
			outputSettings
		],
		(source, steps, index, compiled): EffectsStage | undefined => {
			let image: EffectsImage | undefined;
			let current = false;
			if (!source) {
				// Nothing to show.
			} else if (!steps.length) {
				image = { kind: 'source', source };
				current = true;
			} else if (index?.source === source && compiled?.source === source) {
				image = {
					kind: 'effected',
					width: source.width,
					height: source.height,
					pixels: index.pixels,
					results: compiled.results
				};
				current = compiled.key === currentEffectsKey();
			}
			// Reuse the previous image when its references are unchanged, so an edit that
			// only flips currentness, or touches none of it, never triggers a resample.
			if (sameImage(image, previous?.image)) image = previous!.image;
			if (previous && previous.current === current && previous.image === image) return previous;
			return (previous = image ? { image, current } : source ? { current } : undefined);
		}
	);
	// The retained image can hold a whole source index, so drop it when nobody listens.
	onStop(stage, () => {
		previous = undefined;
	});
	return stage;
})();

/**
 * Samples an effected image: each pixel's colour index through the compile's results, exactly as
 * `applyCompiledEffects` maps it, with fully transparent pixels skipped. Because `samplePixels`
 * is seeded, the samples hit the same positions as `sampleSource` of the same image, so the two
 * stages compare like for like.
 */
export function sampleEffected(
	{ width, height, pixels, results }: Extract<EffectsImage, { kind: 'effected' }>,
	maxSamples = MAX_SAMPLES
): Samples {
	return samplePixels(width, height, maxSamples, (index, rgb, at) => {
		const word = pixels[index]!;
		if (!(word >>> 24)) return false;
		const result = results[word & 0xffffff]!;
		rgb[at] = result & 0xff;
		rgb[at + 1] = (result >>> 8) & 0xff;
		rgb[at + 2] = (result >>> 16) & 0xff;
		return true;
	});
}

/** Keeps at most two valid stages, in pipeline order, so one stage compares against another. */
export function pickStages(next: readonly string[], recent: readonly Stage[]): Stage[] {
	const shown = STAGES.map(({ id }) => id).filter((id) => next.includes(id));
	if (shown.length <= 2) return shown;
	const dropped = recent.find((stage) => shown.includes(stage)) ?? shown[0]!;
	return shown.filter((stage) => stage !== dropped);
}
