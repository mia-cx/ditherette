import { atom, computed } from 'nanostores';
import type { Curve, Effect, EffectContext, MaskCurve } from 'ditherette';
import { colorSpace, outputSettings, selectedPalette, sourceImageData } from '$lib/stores/app';
import { activeEffectSteps, effectLayers, type EffectLayer } from '$lib/stores/effects';
import { packageEffectContext } from './package-adapter';
import type { CropRect } from './types';

type Compile = {
	id: number;
	sourceId: number;
	key: string;
	effects: Effect[];
	context: Required<EffectContext>;
	crop?: CropRect;
};
export type LiveEffectsRequest =
	| { type: 'source'; sourceId: number; source: ImageData }
	| ({ type: 'compile' } & Compile)
	/** `effects` are the steps before the masked one; each colour's result is its mask as grey. */
	| ({ type: 'mask'; mask: MaskCurve[] } & Compile);
export type LiveEffectsResponse =
	| {
			type: 'indexed';
			sourceId: number;
			/** One word per pixel: its colour's index in the low 24 bits, its alpha in the top 8. */
			pixels: Uint32Array;
			count: number;
	  }
	| {
			type: 'compiled' | 'masked';
			id: number;
			sourceId: number;
			key: string;
			results: Uint32Array;
			/** Each palette-fit step's resolved curves, by index into `effects`. Only `compiled`. */
			fits?: readonly { readonly index: number; readonly curves: readonly Curve[] }[];
	  }
	| {
			type: 'failed' | 'mask-failed';
			id: number;
			sourceId: number;
			key: string;
			message: string;
	  };

/** The source's colour index for drawing: see `LiveEffectsResponse`. */
export const effectsIndex = atom<
	{ source: ImageData; pixels: Uint32Array; count: number } | undefined
>();
/** The latest compiled effects: each distinct colour's result, for the source and key named. */
export const compiledEffects = atom<
	{ source: ImageData; key: string; results: Uint32Array } | undefined
>();
/** The layer whose mask the Source preview shows as greys instead of the effects. */
export const shownMask = atom<string | undefined>();
/** The shown mask as each distinct colour's grey: white is full strength. */
export const compiledMask = atom<
	{ source: ImageData; key: string; results: Uint32Array } | undefined
>();
/**
 * The curves the effects worker last resolved for each palette-fit layer, keyed by layer id.
 * Edited fits echo their own list; a cleared source clears this.
 */
export const analysedFits = atom<ReadonlyMap<string, readonly Curve[]>>(new Map());

/**
 * What showing a layer's mask needs: the enabled steps before the layer, which make the colours
 * entering it, and its curves. Nothing when the layer is gone.
 */
export function maskInputs(layers: readonly EffectLayer[], layerId: string | undefined) {
	const at = layers.findIndex((layer) => layer.id === layerId);
	if (at < 0) return undefined;
	const before = layers
		.slice(0, at)
		.filter((layer) => layer.step.enabled)
		.map((layer) => layer.step);
	return { before, mask: [...(layers[at]!.step.mask ?? [])] };
}

/**
 * What a compile depends on. Only palette fit reads the palette and working space, and only one
 * without a recipe reads the (cropped) image, so other edits keep the key.
 */
export function effectsKey(
	effects: readonly Effect[],
	context: Required<EffectContext>,
	crop: CropRect | undefined
) {
	const analyses = effects.filter(
		(step) => step.effect === 'recolour' || step.effect === 'palette-fit'
	);
	const analysing = (step: Effect) =>
		(step.effect === 'recolour' && step.recipe === null) ||
		(step.effect === 'palette-fit' && step.curves === null);
	// A recolour always reads the context (its space); a palette fit reads the palette only
	// while it analyses, and the cropped image only while it analyses.
	const readsContext = analyses.some((step) => step.effect === 'recolour' || analysing(step));
	const readsImage = analyses.some(analysing);
	return JSON.stringify([effects, readsContext && context, readsImage && (crop ?? null)]);
}

/** The key the current settings compile under. */
export function currentEffectsKey() {
	const context = packageEffectContext(selectedPalette.get(), colorSpace.get());
	return effectsKey(activeEffectSteps.get(), context, outputSettings.get().crop);
}

/**
 * What the shown mask depends on: the steps before its layer and its curves. Layers with the same
 * key show the same mask. Nothing while no mask is shown.
 */
export const shownMaskKey = computed(
	[effectLayers, shownMask, selectedPalette, colorSpace, outputSettings],
	(layers, layerId, palette, space, settings) => {
		const masked = maskInputs(layers, layerId);
		if (!masked) return undefined;
		const context = packageEffectContext(palette, space);
		return JSON.stringify([effectsKey(masked.before, context, settings.crop), masked.mask]);
	}
);

type Waiter = {
	source: ImageData;
	key: string;
	resolve: (results: Uint32Array) => void;
	reject: (error: Error) => void;
};
const waiters = new Set<Waiter>();
/** Compiles the current settings if nothing is compiling them; set while live effects run. */
let requestCompile: (() => void) | undefined;

/**
 * The compiled results for `source` under `key`, once the effects worker has them. Processing waits
 * on this, so effects run once per edit and the output reuses them.
 */
export function compiledEffectsFor(source: ImageData, key: string) {
	const current = compiledEffects.get();
	if (current?.source === source && current.key === key) return Promise.resolve(current.results);
	const results = new Promise<Uint32Array>((resolve, reject) =>
		waiters.add({ source, key, resolve, reject })
	);
	// After the worker fails, nothing compiles until asked.
	requestCompile?.();
	return results;
}

function settle(source: ImageData, key: string, outcome: Uint32Array | Error) {
	for (const waiter of waiters) {
		if (waiter.source !== source || waiter.key !== key) continue;
		waiters.delete(waiter);
		if (outcome instanceof Error) waiter.reject(outcome);
		else waiter.resolve(outcome);
	}
}

/**
 * Compile effects the moment they change, in a worker of their own, so the Source preview keeps up
 * while the output is still dithering. One compile runs at a time; edits during it fold into the
 * next, and the latest edit wins. Returns a stop function.
 */
export function startLiveEffects() {
	let worker: Worker | undefined;
	let sources = new WeakMap<ImageData, number>();
	let nextSourceId = 0;
	let loaded: ImageData | undefined;
	let busy = false;
	let maskBusy = false;
	/** The mask that last failed; it waits for new inputs, or for Show mask to turn off and on. */
	let failedMask: { source: ImageData; key: string } | undefined;
	let requestId = 0;
	/** The effects each in-flight compile was sent, so its fit curves map back to layers. */
	const pending = new Map<number, readonly Effect[]>();

	/** Drop the worker and everything it was doing; the next compile starts a fresh one. */
	function reset(reason: string) {
		worker?.terminate();
		worker = undefined;
		loaded = undefined;
		pending.clear();
		busy = maskBusy = false;
		failedMask = undefined;
		effectsIndex.set(undefined);
		compiledEffects.set(undefined);
		compiledMask.set(undefined);
		analysedFits.set(new Map());
		for (const waiter of waiters) waiter.reject(new Error(reason));
		waiters.clear();
	}

	function stop() {
		reset('The image changed.');
		sources = new WeakMap();
	}

	function crashed(event: Event) {
		console.error('The effects worker failed.', event);
		reset('The effects worker failed.');
	}

	function sourceId(source: ImageData) {
		let id = sources.get(source);
		if (id === undefined) sources.set(source, (id = ++nextSourceId));
		return id;
	}

	function update() {
		const source = sourceImageData.get();
		if (!source) return stop();
		const effects = activeEffectSteps.get();
		const masked = maskInputs(effectLayers.get(), shownMask.get());
		if (!masked) failedMask = undefined;
		if (!masked && shownMask.get()) return shownMask.set(undefined);
		if (!effects.length && !masked) return;
		if (!worker) {
			worker = new Worker(new URL('../workers/effects.worker.ts', import.meta.url), {
				type: 'module'
			});
			worker.onmessage = receive;
			worker.onerror = worker.onmessageerror = crashed;
		}
		if (source !== loaded) {
			effectsIndex.set(undefined);
			compiledEffects.set(undefined);
			compiledMask.set(undefined);
			analysedFits.set(new Map());
			worker.postMessage({
				type: 'source',
				sourceId: sourceId(source),
				source
			} satisfies LiveEffectsRequest);
			loaded = source;
		}
		const context = packageEffectContext(selectedPalette.get(), colorSpace.get());
		const crop = outputSettings.get().crop;
		const key = effectsKey(effects, context, crop);
		const current = compiledEffects.get();
		if (effects.length && !busy && (current?.source !== source || current.key !== key)) {
			busy = true;
			pending.set(requestId + 1, effects);
			worker.postMessage({
				type: 'compile',
				id: ++requestId,
				sourceId: sourceId(source),
				key,
				effects,
				context,
				crop
			} satisfies LiveEffectsRequest);
		}
		if (!masked) return;
		const maskKey = shownMaskKey.get()!;
		const shown = compiledMask.get();
		if (maskBusy || (failedMask?.source === source && failedMask.key === maskKey)) return;
		if (shown?.source === source && shown.key === maskKey) return;
		maskBusy = true;
		worker.postMessage({
			type: 'mask',
			id: ++requestId,
			sourceId: sourceId(source),
			key: maskKey,
			effects: masked.before,
			mask: masked.mask,
			context,
			crop
		} satisfies LiveEffectsRequest);
	}

	function receive({ data }: MessageEvent<LiveEffectsResponse>) {
		const source = sourceImageData.get();
		const current = source && sources.get(source) === data.sourceId ? source : undefined;
		if (data.type === 'indexed') {
			if (current) effectsIndex.set({ source: current, pixels: data.pixels, count: data.count });
			return;
		}
		if (data.type === 'masked' || data.type === 'mask-failed') {
			maskBusy = false;
			// A mask whose layer or inputs changed while it ran is stale; `update` asks for the new one.
			const wanted = current && data.key === shownMaskKey.get() ? current : undefined;
			if (wanted && data.type === 'masked')
				compiledMask.set({ source: wanted, key: data.key, results: data.results });
			else if (wanted && data.type === 'mask-failed') {
				failedMask = { source: wanted, key: data.key };
				console.error('Could not show the mask.', data.message);
			}
			return update();
		}
		busy = false;
		if (current && data.type === 'failed') {
			// The preview is optional, so its failure stays out of the output's error; processing
			// waiting on this key reports it instead.
			console.error('Could not compile the effects.', data.message);
			settle(current, data.key, new Error(data.message));
		} else if (current && data.type === 'compiled') {
			compiledEffects.set({ source: current, key: data.key, results: data.results });
			// Each fit index names its position in the sent steps; count enabled fit layers in
			// the same order the worker resolved them.
			const sent = pending.get(data.id);
			const fitLayers = effectLayers
				.get()
				.filter((layer) => layer.step.enabled && layer.step.effect === 'palette-fit');
			const fits = new Map<string, readonly Curve[]>();
			data.fits?.forEach(({ index, curves }) => {
				const at = sent?.slice(0, index + 1).filter((s) => s.effect === 'palette-fit').length;
				const layer = at ? fitLayers[at - 1] : undefined;
				if (layer) fits.set(layer.id, curves);
			});
			analysedFits.set(fits);
			settle(current, data.key, data.results);
		}
		pending.delete(data.id);
		// Settings that moved on will never compile; their processing is superseded anyway.
		const key = currentEffectsKey();
		for (const waiter of waiters) {
			if (waiter.source === current && waiter.key === key) continue;
			waiters.delete(waiter);
			waiter.reject(new Error('The effects changed.'));
		}
		update();
	}

	const unsubscribers = [
		sourceImageData.listen(update),
		activeEffectSteps.listen(update),
		// A disabled layer's mask can be shown too, and its edits never reach the active steps.
		effectLayers.listen(update),
		shownMask.listen(update),
		selectedPalette.listen(update),
		colorSpace.listen(update),
		outputSettings.listen(update)
	];
	requestCompile = update;
	update();
	return () => {
		requestCompile = undefined;
		for (const unsubscribe of unsubscribers) unsubscribe();
		stop();
	};
}
