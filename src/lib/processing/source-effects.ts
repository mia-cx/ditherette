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
 * Tables kept per source, so toggling the preview or an effect back to a recent state is instant.
 * Each is `LUT_SIZE`³ × 4 bytes, about half a megabyte.
 */
const CACHED_TABLES = 8;

/**
 * The enabled effects as a 3D lookup table: `LUT_SIZE`³ RGBA entries, red fastest. The Source pane
 * draws the source through it on the GPU while the preview's `sourceEffects` toggle is on. The
 * table stays while the toggle is off, so turning it back on shows it at once.
 */
export const sourceEffectsLut = atom<Uint8Array | undefined>();

/**
 * Keep `sourceEffectsLut` in step with the source, effects, and palette while the preview's
 * `sourceEffects` toggle is on. One request runs at a time; edits during it coalesce into the next.
 * The worker starts the first time the toggle is on and keeps its copy of the source, and recent
 * tables, until the source changes. Returns a stop function.
 */
export function startSourceEffects() {
	let worker: Worker | undefined;
	let loaded: ImageData | undefined;
	let busy = false;
	let requestId = 0;
	let requestedKey = '';
	/** Recent tables for the loaded source by settings, least recently used first. */
	const tables = new Map<string, Uint8Array>();

	const show = (lut: Uint8Array | undefined) => sourceEffectsLut.set(lut);

	function remember(key: string, lut: Uint8Array) {
		tables.delete(key);
		tables.set(key, lut);
		if (tables.size > CACHED_TABLES) tables.delete(tables.keys().next().value!);
	}

	function stop() {
		worker?.terminate();
		worker = undefined;
		loaded = undefined;
		busy = false;
		tables.clear();
		show(undefined);
	}

	function load(source: ImageData) {
		if (!worker) {
			worker = new Worker(new URL('../workers/source-effects.worker.ts', import.meta.url), {
				type: 'module'
			});
			worker.onmessage = receive;
		}
		// Palette fit analyses the source, so a new one invalidates every table, including the
		// one still being computed.
		tables.clear();
		requestId++;
		show(undefined);
		worker.postMessage({ type: 'source', source } satisfies SourceEffectsRequest);
		loaded = source;
	}

	function update() {
		const source = sourceImageData.get();
		if (!source) return stop();
		// Hidden: keep the worker, the table, and the cache for when the toggle comes back on.
		if (!previewSettings.get().sourceEffects) return;
		const effects = activeEffectSteps.get();
		if (!effects.length) return show(undefined);
		const context = packageEffectContext(selectedPalette.get(), colorSpace.get());
		const key = JSON.stringify([effects, context]);
		if (source !== loaded) load(source);
		const cached = tables.get(key);
		if (cached) {
			remember(key, cached);
			return show(cached);
		}
		if (busy) return;
		busy = true;
		requestedKey = key;
		worker!.postMessage({
			type: 'apply',
			id: ++requestId,
			effects,
			context
		} satisfies SourceEffectsRequest);
	}

	function receive({ data }: MessageEvent<SourceEffectsResponse>) {
		busy = false;
		// A reply for an earlier source: ask for the current one instead.
		if (data.id !== requestId) return update();
		if ('error' in data) {
			// The preview is optional and separate from the output, so its failure stays out of the
			// output's error; the pane just goes back to the plain source.
			console.error('Could not show effects on the source.', data.error);
			show(undefined);
			return;
		}
		remember(requestedKey, data.lut);
		// Show it even if the settings moved on, so edits update progressively; update() then
		// asks for the latest.
		if (previewSettings.get().sourceEffects) show(data.lut);
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
