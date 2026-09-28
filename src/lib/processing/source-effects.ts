import { atom } from 'nanostores';
import type { Effect, EffectContext } from 'ditherette';
import { colorSpace, previewSettings, selectedPalette, sourceImageData } from '$lib/stores/app';
import { activeEffectSteps } from '$lib/stores/effects';
import { packageEffectContext } from './package-adapter';

export type SourceEffectsRequest =
	| { type: 'source'; source: ImageData }
	| { type: 'apply'; id: number; effects: Effect[]; context: Required<EffectContext> };
export type SourceEffectsResponse = { id: number } & ({ lut: Uint8Array } | { error: string });

/**
 * Lattice points per axis. 255 / 51 = 5, so every lattice colour is a whole byte and the table's
 * texture coordinates land exactly on the colours it was computed from.
 */
export const LUT_SIZE = 52;

/**
 * The enabled effects as a 3D lookup table: `LUT_SIZE`³ RGBA entries, red fastest. The Source pane
 * draws the source through it on the GPU while the preview's `sourceEffects` toggle is on.
 */
export const sourceEffectsLut = atom<Uint8Array | undefined>();

/**
 * Keep `sourceEffectsLut` in step with the source, effects, and palette while the preview's
 * `sourceEffects` toggle is on. One request runs at a time; edits during it coalesce into the next.
 * The worker exists only while there is something to show. Returns a stop function.
 */
export function startSourceEffects() {
	let worker: Worker | undefined;
	let loaded: ImageData | undefined;
	let sentKey: string | undefined;
	let busy = false;
	let requestId = 0;

	const show = (lut: Uint8Array | undefined) => sourceEffectsLut.set(lut);

	function stop() {
		worker?.terminate();
		worker = undefined;
		loaded = undefined;
		sentKey = undefined;
		busy = false;
		show(undefined);
	}

	function update() {
		const source = sourceImageData.get();
		const effects = activeEffectSteps.get();
		if (!previewSettings.get().sourceEffects || !source || !effects.length) return stop();
		const context = packageEffectContext(selectedPalette.get(), colorSpace.get());
		const key = JSON.stringify([effects, context]);
		if (busy || (source === loaded && key === sentKey)) return;
		if (!worker) {
			worker = new Worker(new URL('../workers/source-effects.worker.ts', import.meta.url), {
				type: 'module'
			});
			worker.onmessage = receive;
		}
		if (source !== loaded) {
			show(undefined);
			worker.postMessage({ type: 'source', source } satisfies SourceEffectsRequest);
			loaded = source;
		}
		busy = true;
		sentKey = key;
		worker.postMessage({
			type: 'apply',
			id: ++requestId,
			effects,
			context
		} satisfies SourceEffectsRequest);
	}

	function receive({ data }: MessageEvent<SourceEffectsResponse>) {
		busy = false;
		if (data.id !== requestId) return;
		if ('error' in data) {
			// The preview is optional and separate from the output, so its failure stays out of the
			// output's error; the pane just goes back to the plain source.
			console.error('Could not show effects on the source.', data.error);
			show(undefined);
		}
		// A newer source may have arrived while this one ran; it gets its own request below.
		else if (loaded === sourceImageData.get()) show(data.lut);
		update();
	}

	const unsubscribers = [
		previewSettings.listen(update),
		sourceImageData.listen(update),
		activeEffectSteps.listen(update),
		selectedPalette.listen(update),
		colorSpace.listen(update)
	];
	update();
	return () => {
		for (const unsubscribe of unsubscribers) unsubscribe();
		stop();
	};
}
