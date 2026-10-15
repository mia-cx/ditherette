import { atom, onMount, type ReadableAtom } from 'nanostores';
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
 * keeps the older results as a dimmed stand-in, marked not current. Derived while someone
 * listens, and empty otherwise, so a closed Scopes keeps no image buffers.
 */
export const effectsStage: ReadableAtom<EffectsStage | undefined> = (() => {
	const INPUTS = [
		sourceImageData,
		activeEffectSteps,
		effectsIndex,
		compiledEffects,
		selectedPalette,
		colorSpace,
		outputSettings
	] as const;
	const stage = atom<EffectsStage | undefined>();
	onMount(stage, () => {
		const update = () => {
			const source = sourceImageData.get();
			const steps = activeEffectSteps.get();
			const index = effectsIndex.get();
			const compiled = compiledEffects.get();
			const previous = stage.value;
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
			const next =
				previous && previous.current === current && previous.image === image
					? previous
					: image
						? { image, current }
						: source
							? { current }
							: undefined;
			stage.set(next);
		};
		const unbinds = INPUTS.map((input) => input.listen(update));
		update();
		return () => {
			for (const unbind of unbinds) unbind();
			// Nothing else holds the stage's inputs, so a closed Scopes keeps no image buffers.
			stage.set(undefined);
		};
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
