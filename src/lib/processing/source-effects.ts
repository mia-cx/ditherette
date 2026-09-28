import { atom } from 'nanostores';
import type { Effect, EffectContext } from 'ditherette';
import {
	colorSpace,
	previewSettings,
	processingError,
	selectedPalette,
	sourceImageData
} from '$lib/stores/app';
import { activeEffectSteps } from '$lib/stores/effects';
import { packageEffectContext } from './package-adapter';

export type SourceEffectsRequest =
	| { type: 'source'; source: ImageData }
	| { type: 'apply'; id: number; effects: Effect[]; context: EffectContext };
export type SourceEffectsResponse = { id: number } & ({ bitmap: ImageBitmap } | { error: string });

/** The full-resolution source after the enabled effects, while the Source pane shows them. */
export const adjustedSource = atom<ImageBitmap | undefined>();

/**
 * Keep `adjustedSource` in step with the source, effects, and palette while the preview's
 * `sourceEffects` toggle is on. One request runs at a time; edits during it coalesce into the next.
 * The worker exists only while there is something to show. Returns a stop function.
 */
export function startSourceEffects() {
	let worker: Worker | undefined;
	let loaded: ImageData | undefined;
	let sentKey: string | undefined;
	let busy = false;
	let requestId = 0;

	function show(bitmap: ImageBitmap | undefined) {
		adjustedSource.get()?.close();
		adjustedSource.set(bitmap);
	}

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
		if ('error' in data) processingError.set(`Could not show effects on the source: ${data.error}`);
		// A newer source may have arrived while this one ran; it gets its own request below.
		else if (loaded === sourceImageData.get()) show(data.bitmap);
		else data.bitmap.close();
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
