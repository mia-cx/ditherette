import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	outputSettings,
	selectedPalette,
	setPaletteColorEnabled,
	sourceImageData
} from '$lib/stores/app';
import {
	addEffect,
	effectLayers,
	moveEffect,
	removeEffect,
	setEffectEnabled,
	setEffectMask,
	updateEffect
} from '$lib/stores/effects';
import { shownSource } from './effects-view.svelte';
import {
	resolvedFits,
	compiledEffects,
	compiledEffectsFor,
	currentEffectsKey,
	effectsKey,
	effectsIndex,
	maskInputs,
	shownMask,
	startLiveEffects,
	RETAINED_BYTES,
	RETAINED_COMPILES,
	type LiveEffectsRequest,
	type LiveEffectsResponse
} from './live-effects';

class ControlledWorker {
	static instances: ControlledWorker[] = [];
	onmessage?: (event: MessageEvent<LiveEffectsResponse>) => void;
	onerror?: (event: Event) => void;
	messages: LiveEffectsRequest[] = [];
	terminate = vi.fn();
	constructor() {
		ControlledWorker.instances.push(this);
	}
	postMessage(message: LiveEffectsRequest) {
		this.messages.push(message);
	}
	get compiles() {
		return this.messages.filter((message) => message.type === 'compile');
	}
	get masks() {
		return this.messages.filter((message) => message.type === 'mask');
	}
	/** Answer a mask request, the latest by default, with greys or a failure. */
	replyMask(outcome: Uint32Array | Error, request = this.masks.at(-1)!) {
		const { id, sourceId, key } = request;
		this.onmessage?.({
			data:
				outcome instanceof Error
					? { type: 'mask-failed', id, sourceId, key, message: outcome.message }
					: { type: 'masked', id, sourceId, key, results: outcome }
		} as MessageEvent<LiveEffectsResponse>);
	}
	/** Answer a compile request with a failure, the latest by default. */
	fail(message: string, request = this.compiles.at(-1)!) {
		const { id, sourceId, key } = request;
		this.onmessage?.({
			data: { type: 'failed', id, sourceId, key, message }
		} as MessageEvent<LiveEffectsResponse>);
	}
	/** Post the colour index for the loaded source. */
	index(pixels: Uint32Array) {
		const source = this.messages.filter((message) => message.type === 'source').at(-1)!;
		this.onmessage?.({
			data: { type: 'indexed', sourceId: source.sourceId, pixels, count: pixels.length }
		} as MessageEvent<LiveEffectsResponse>);
	}
	/** Answer a compile request, the latest by default. */
	reply(
		results = new Uint32Array([1]),
		fits?: { index: number; curves: readonly object[] }[],
		request = this.compiles.at(-1)!
	) {
		this.onmessage?.({
			data: {
				type: 'compiled',
				id: request.id,
				sourceId: request.sourceId,
				key: request.key,
				results,
				...(fits ? { fits } : {})
			}
		} as MessageEvent<LiveEffectsResponse>);
	}
}

const image = () => ({ width: 1, height: 1, data: new Uint8ClampedArray(4) }) as ImageData;
let stop: () => void;

beforeEach(() => {
	vi.stubGlobal('Worker', ControlledWorker);
	ControlledWorker.instances = [];
	effectLayers.set([]);
	shownMask.set(undefined);
	sourceImageData.set(image());
	stop = startLiveEffects();
});

afterEach(() => {
	stop();
	vi.unstubAllGlobals();
});

describe('shown masks', () => {
	const lights = {
		x: { model: 'oklch', channel: 'lightness' } as const,
		points: [
			[0, 0],
			[1, 1]
		] as const
	};

	it('compile the steps before the masked layer, and stop when the layer goes', () => {
		const first = addEffect('exposure');
		const second = addEffect('levels');
		setEffectMask(second.id, [lights]);
		setEffectEnabled(first.id, false);
		const third = addEffect('exposure');
		shownMask.set(second.id);
		const worker = ControlledWorker.instances[0]!;
		const masks = worker.messages.filter((message) => message.type === 'mask');
		expect(masks).toHaveLength(1);
		expect(masks[0]).toMatchObject({ effects: [], mask: [lights] });
		expect(maskInputs(effectLayers.get(), third.id)!.before).toEqual([effectLayers.get()[1]!.step]);

		removeEffect(second.id);
		expect(shownMask.get()).toBeUndefined();
	});

	it('wait after a failure until the mask changes or is shown again', () => {
		vi.spyOn(console, 'error').mockImplementation(() => {});
		const layer = addEffect('exposure');
		setEffectEnabled(layer.id, false);
		setEffectMask(layer.id, [lights]);
		shownMask.set(layer.id);
		const worker = ControlledWorker.instances[0]!;
		worker.replyMask(new Error('No memory.'));
		expect(worker.masks).toHaveLength(1);

		shownMask.set(undefined);
		shownMask.set(layer.id);
		expect(worker.masks).toHaveLength(2);
		worker.replyMask(new Error('No memory.'));
		setEffectMask(layer.id, []);
		expect(worker.masks).toHaveLength(3);
	});

	it("never show one layer's greys for another", () => {
		const first = addEffect('exposure');
		const second = addEffect('levels');
		setEffectMask(second.id, [lights]);
		const worker = ControlledWorker.instances[0]!;
		worker.reply();
		effectsIndex.set({ source: sourceImageData.get()!, pixels: new Uint32Array(1), count: 1 });
		const white = new Uint32Array([0xffffff]);
		const grey = new Uint32Array([0x808080]);

		// The first layer's answer arrives after switching to the second: it is dropped.
		shownMask.set(first.id);
		shownMask.set(second.id);
		worker.replyMask(white);
		expect(shownSource.get()).toBeUndefined();
		expect(worker.masks).toHaveLength(2);
		worker.replyMask(grey);
		expect(shownSource.get()?.results).toEqual(grey);

		// Switching away from finished greys hides them until the new layer's arrive.
		shownMask.set(first.id);
		expect(shownSource.get()).toBeUndefined();
		worker.replyMask(white);
		expect(shownSource.get()?.results).toEqual(white);
	});
});

describe('live effects', () => {
	it('bounds retiring workers and terminates a readback that never settles', () => {
		vi.useFakeTimers();
		try {
			addEffect('exposure');
			const first = ControlledWorker.instances[0]!;
			stop();
			stop = startLiveEffects();
			const next = ControlledWorker.instances[1]!;
			stop();
			expect(first.terminate).toHaveBeenCalled();
			expect(next.terminate).not.toHaveBeenCalled();
			vi.runAllTimers();
			expect(next.terminate).toHaveBeenCalledOnce();
		} finally {
			vi.useRealTimers();
		}
	});

	it('cancels superseded GPU work, rejects old waiters and ignores duplicate replies', async () => {
		const layer = addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		const first = worker.compiles[0]!;
		const source = sourceImageData.get()!;
		const old = compiledEffectsFor(source, first.key).catch((error: unknown) => error);
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 1 });
		expect(worker.messages.filter((message) => message.type === 'cancel')).toEqual([
			{ type: 'cancel', id: first.id }
		]);
		worker.onmessage?.({
			data: {
				type: 'cancelled',
				operation: 'compile',
				id: first.id,
				sourceId: first.sourceId,
				key: first.key
			}
		} as MessageEvent<LiveEffectsResponse>);
		expect(await old).toMatchObject({ message: 'The effects changed.' });
		expect(worker.compiles).toHaveLength(2);
		expect(compiledEffects.get()).toBeUndefined();
		worker.reply(new Uint32Array([3]), undefined, first);
		expect(worker.compiles).toHaveLength(2);
		expect(compiledEffects.get()).toBeUndefined();
		worker.reply(new Uint32Array([4]));
		expect(compiledEffects.get()?.key).toBe(currentEffectsKey());
	});

	it('disposes workers on stop and ignores replies from an earlier worker', () => {
		addEffect('exposure');
		const first = ControlledWorker.instances[0]!;
		const late = first.onmessage!;
		const request = first.compiles[0]!;
		stop();
		expect(first.messages.at(-1)).toEqual({ type: 'dispose' });
		stop = startLiveEffects();
		late(
			new MessageEvent<LiveEffectsResponse>('message', {
				data: {
					type: 'compiled',
					id: request.id,
					sourceId: request.sourceId,
					key: request.key,
					results: new Uint32Array([99])
				}
			})
		);
		expect(compiledEffects.get()).toBeUndefined();
		first.onmessage?.({ data: { type: 'disposed' } } as MessageEvent<LiveEffectsResponse>);
		expect(first.terminate).toHaveBeenCalled();
		ControlledWorker.instances[1]!.reply(new Uint32Array([1]));
		expect(compiledEffects.get()?.results).toEqual(new Uint32Array([1]));
	});

	it('compiles as soon as effects change, and folds edits during a compile into the next', () => {
		const layer = addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		expect(worker.messages[0]!.type).toBe('source');
		expect(worker.compiles).toHaveLength(1);

		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 1 });
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 2 });
		expect(worker.compiles).toHaveLength(1);

		worker.reply();
		expect(worker.compiles).toHaveLength(2);
		expect(worker.compiles[1]).toMatchObject({ effects: [{ stops: 2 }] });
	});

	it('hands processing the results for its key, and releases keys that moved on', async () => {
		const layer = addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		const source = sourceImageData.get()!;
		const first = compiledEffectsFor(source, currentEffectsKey());
		// Folded away while the first compile runs, so it never compiles.
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 1 });
		const skipped = compiledEffectsFor(source, currentEffectsKey());
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 2 });
		const current = compiledEffectsFor(source, currentEffectsKey());

		worker.reply(new Uint32Array([7]));
		await expect(first).resolves.toEqual(new Uint32Array([7]));
		await expect(skipped).rejects.toThrow('The effects changed.');
		worker.reply(new Uint32Array([9]));
		await expect(current).resolves.toEqual(new Uint32Array([9]));
		expect(compiledEffects.get()?.results).toEqual(new Uint32Array([9]));
		// Already compiled: answered at once.
		await expect(compiledEffectsFor(source, currentEffectsKey())).resolves.toEqual(
			new Uint32Array([9])
		);
	});

	it('loads a new source before compiling for it', () => {
		addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		worker.reply();
		sourceImageData.set(image());
		expect(worker.messages.filter((message) => message.type === 'source')).toHaveLength(2);
		expect(worker.compiles).toHaveLength(2);
		expect(compiledEffects.get()).toBeUndefined();
	});

	it.each(['success', 'failure'])(
		'keeps processing for the new source pending after an old-source %s',
		async (outcome) => {
			const errors = vi.spyOn(console, 'error').mockImplementation(() => {});
			errors.mockClear();
			addEffect('exposure');
			const worker = ControlledWorker.instances[0]!;
			const old = worker.compiles[0]!;
			const superseded = compiledEffectsFor(sourceImageData.get()!, currentEffectsKey());
			const source = image();
			sourceImageData.set(source);
			const waiting = compiledEffectsFor(source, currentEffectsKey());
			const settled = vi.fn();
			void waiting.then(settled, settled);
			expect(worker.compiles).toHaveLength(1);

			if (outcome === 'success') worker.reply(new Uint32Array([1]), undefined, old);
			else worker.fail('Old source failed.', old);

			await expect(superseded).rejects.toThrow('The effects changed.');
			expect(settled).not.toHaveBeenCalled();
			expect(compiledEffects.get()).toBeUndefined();
			expect(errors).not.toHaveBeenCalled();
			expect(worker.compiles).toHaveLength(2);
			expect(worker.compiles[1]!.sourceId).not.toBe(old.sourceId);

			const results = new Uint32Array([2]);
			worker.reply(results);
			await expect(waiting).resolves.toBe(results);
			expect(compiledEffects.get()).toEqual({ source, key: currentEffectsKey(), results });
		}
	);

	it('rejects processing when the current compile fails', async () => {
		const errors = vi.spyOn(console, 'error').mockImplementation(() => {});
		errors.mockClear();
		addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		const waiting = compiledEffectsFor(sourceImageData.get()!, currentEffectsKey());

		worker.fail('Current source failed.');

		await expect(waiting).rejects.toThrow('Current source failed.');
		expect(errors).toHaveBeenCalledExactlyOnceWith(
			'Could not compile the effects.',
			'Current source failed.'
		);
	});

	it('replaces a worker that fails, once asked again', async () => {
		vi.spyOn(console, 'error').mockImplementation(() => {});
		addEffect('exposure');
		const failed = ControlledWorker.instances[0]!;
		const source = sourceImageData.get()!;
		const waiting = compiledEffectsFor(source, currentEffectsKey());
		failed.onerror?.(new Event('error'));
		await expect(waiting).rejects.toThrow('The effects worker failed.');
		expect(failed.terminate).toHaveBeenCalled();

		const retried = compiledEffectsFor(source, currentEffectsKey());
		const fresh = ControlledWorker.instances[1]!;
		expect(fresh.messages.map((message) => message.type)).toEqual(['source', 'compile']);
		fresh.reply(new Uint32Array([3]));
		await expect(retried).resolves.toEqual(new Uint32Array([3]));
	});
});

describe('effectsKey for palette fit', () => {
	const context = { palette: [], space: 'oklab' as const };
	const crop = { x: 0, y: 0, width: 2, height: 2 };
	const analysing = {
		effect: 'palette-fit' as const,
		enabled: true,
		look: 'fitted' as const,
		space: 'oklab' as const,
		strength: 1,
		curves: null
	};
	const edited = {
		...analysing,
		curves: [
			{
				kind: 'remap' as const,
				x: { model: 'oklch' as const, channel: 'lightness' as const },
				y: { model: 'oklch' as const, channel: 'lightness' as const },
				points: [
					[0, 0],
					[1, 1]
				] as [number, number][]
			}
		]
	};

	it('reads the palette and crop while analysing, and neither once edited', () => {
		const key = effectsKey([analysing], context, crop);
		const otherPalette = {
			palette: [{ kind: 'color' as const, rgb: [0, 0, 0] as const }],
			space: 'oklab' as const
		};
		expect(effectsKey([analysing], otherPalette, crop)).not.toBe(key);
		expect(effectsKey([analysing], context, undefined)).not.toBe(key);
		expect(effectsKey([analysing], context, crop)).toBe(key);

		const locked = effectsKey([edited], context, crop);
		expect(effectsKey([edited], otherPalette, crop)).toBe(locked);
		expect(effectsKey([edited], context, undefined)).toBe(locked);
		// Space is the step's own, so the context's space never matters.
		expect(effectsKey([analysing], { ...context, space: 'cielab' as const }, crop)).toBe(key);
	});
});

describe('analysed fits', () => {
	const curves = [
		{
			kind: 'remap',
			x: { model: 'oklch', channel: 'lightness' },
			y: { model: 'oklch', channel: 'lightness' },
			points: [
				[0, 0],
				[1, 1]
			]
		}
	];
	const other = [
		{
			kind: 'remap',
			x: { model: 'oklch', channel: 'lightness' },
			y: { model: 'oklch', channel: 'lightness' },
			points: [
				[0, 0.25],
				[1, 0.75]
			]
		}
	];

	it('publishes curves under the layers that produced them, not the order at reply', () => {
		const first = addEffect('palette-fit');
		const second = addEffect('palette-fit');
		const worker = ControlledWorker.instances.at(-1)!;
		worker.reply(new Uint32Array([1]), [{ index: 0, curves }]);
		// Give the second fit distinct settings so the next compile keys differently.
		const step = effectLayers.get().find((item) => item.id === second.id)!.step;
		if (step.effect !== 'palette-fit') throw new Error('Expected a palette-fit layer.');
		updateEffect(second.id, { ...step, strength: 2 });
		const request = worker.compiles.at(-1)!;
		removeEffect(first.id);
		// The two-fit reply lands stale after the removal: its curves publish under nothing,
		// so the second fit reads them only once the matching compile answers.
		worker.reply(new Uint32Array([1]), [{ index: 1, curves: other }], request);
		expect(resolvedFits.get().size).toBe(0);
		worker.reply(new Uint32Array([1]), [{ index: 0, curves: other }]);
		expect(resolvedFits.get().get(second.id)).toBe(other);
	});

	it.each([
		['removing', () => removeEffect(effectLayers.get()[0]!.id)],
		['disabling', () => setEffectEnabled(effectLayers.get()[0]!.id, false)],
		['reordering', () => moveEffect(effectLayers.get()[0]!.id, 1)]
	])('publishes no stale curves when a reply lands after %s a fit', (_name, change) => {
		addEffect('palette-fit');
		const second = addEffect('palette-fit');
		const worker = ControlledWorker.instances.at(-1)!;
		const step = effectLayers.get().find((item) => item.id === second.id)!.step;
		if (step.effect !== 'palette-fit') throw new Error('Expected a palette-fit layer.');
		updateEffect(second.id, { ...step, strength: 2 });
		const request = worker.compiles.at(-1)!;
		change();
		worker.reply(
			new Uint32Array([1]),
			[
				{ index: 0, curves },
				{ index: 1, curves }
			],
			request
		);
		expect(resolvedFits.get().size).toBe(0);
		const next = worker.compiles.at(-1)!;
		expect(next.key).not.toBe(request.key);
		worker.reply(
			new Uint32Array([1]),
			[
				{ index: 0, curves },
				{ index: 1, curves }
			],
			next
		);
		expect(resolvedFits.get().size).toBeGreaterThan(0);
	});

	it('keeps each layer on the curves of the position it now holds', () => {
		const first = addEffect('palette-fit');
		const worker = ControlledWorker.instances.at(-1)!;
		worker.reply(new Uint32Array([1]), [{ index: 0, curves }]);
		const second = addEffect('palette-fit');
		worker.reply(new Uint32Array([1]), [
			{ index: 0, curves },
			{ index: 1, curves: other }
		]);
		expect(resolvedFits.get().get(first.id)).toBe(curves);
		expect(resolvedFits.get().get(second.id)).toBe(other);

		moveEffect(first.id, 1);
		// Same steps in a new order: no new compile runs, and each layer now reads the
		// curves for the position it occupies.
		expect(worker.compiles.at(-1)).toBeDefined();
		const count = worker.compiles.length;
		expect(resolvedFits.get().get(first.id)).toBe(other);
		expect(resolvedFits.get().get(second.id)).toBe(curves);
		expect(worker.compiles).toHaveLength(count);
	});

	it('publishes no stale curves when the palette changes mid-compile', () => {
		addEffect('palette-fit');
		const worker = ControlledWorker.instances.at(-1)!;
		const request = worker.compiles.at(-1)!;
		const colour = selectedPalette.get().find((item) => item.rgb)!;
		setPaletteColorEnabled(colour.key, false);
		worker.reply(new Uint32Array([1]), [{ index: 0, curves }], request);
		expect(resolvedFits.get().size).toBe(0);
		worker.reply(new Uint32Array([1]), [{ index: 0, curves }]);
		expect(resolvedFits.get().size).toBe(1);
	});
});

describe('retained compiles', () => {
	const curves = [
		{
			kind: 'remap',
			x: { model: 'oklch', channel: 'lightness' },
			y: { model: 'oklch', channel: 'lightness' },
			points: [
				[0, 0],
				[1, 1]
			]
		}
	];
	const other = [
		{
			kind: 'remap',
			x: { model: 'oklch', channel: 'lightness' },
			y: { model: 'oklch', channel: 'lightness' },
			points: [
				[0, 0.25],
				[1, 0.75]
			]
		}
	];

	/** Two enabled exposure layers: the full pipeline is chain A, disabling `y` gives chain B. */
	function twoLayers() {
		const x = addEffect('exposure');
		const y = addEffect('exposure');
		updateEffect(y.id, { effect: 'exposure', enabled: true, stops: 1 });
		return { x, y };
	}

	it('serves a return to recent settings from the retained compiles', async () => {
		const { x } = twoLayers();
		const worker = ControlledWorker.instances[0]!;
		const source = sourceImageData.get()!;
		const resultsA = new Uint32Array([0xa]);
		const resultsB = new Uint32Array([0xb]);

		worker.reply(resultsA);
		expect(worker.compiles).toHaveLength(2);
		worker.reply(resultsA);
		expect(compiledEffects.get()?.results).toBe(resultsA);

		setEffectEnabled(x.id, false);
		expect(worker.compiles).toHaveLength(3);
		worker.reply(resultsB);
		expect(compiledEffects.get()?.results).toBe(resultsB);

		setEffectEnabled(x.id, true);
		expect(worker.compiles).toHaveLength(3);
		expect(compiledEffects.get()?.results).toBe(resultsA);
		await expect(compiledEffectsFor(source, currentEffectsKey())).resolves.toBe(resultsA);
	});

	it('keeps a current chain when an older reply lands after returning to it', () => {
		const { x } = twoLayers();
		const worker = ControlledWorker.instances[0]!;
		const resultsA = new Uint32Array([0xa]);
		const resultsB = new Uint32Array([0xb]);
		worker.reply(resultsA);
		worker.reply(resultsA);

		setEffectEnabled(x.id, false);
		expect(worker.compiles).toHaveLength(3);
		setEffectEnabled(x.id, true);
		expect(compiledEffects.get()?.results).toBe(resultsA);

		worker.reply(resultsB);
		expect(compiledEffects.get()?.results).toBe(resultsA);

		// The stale reply was still retained, so going back to chain B costs nothing.
		setEffectEnabled(x.id, false);
		expect(worker.compiles).toHaveLength(3);
		expect(compiledEffects.get()?.results).toBe(resultsB);
	});

	it('retains the fits that analysis steps read alongside their results', () => {
		const layer = addEffect('palette-fit');
		const worker = ControlledWorker.instances[0]!;
		worker.reply(new Uint32Array([1]), [{ index: 0, curves }]);
		expect(resolvedFits.get().get(layer.id)).toBe(curves);

		const colour = selectedPalette.get().find((item) => item.rgb)!;
		setPaletteColorEnabled(colour.key, false);
		expect(worker.compiles).toHaveLength(2);
		worker.reply(new Uint32Array([2]), [{ index: 0, curves: other }]);
		expect(resolvedFits.get().get(layer.id)).toBe(other);

		setPaletteColorEnabled(colour.key, true);
		expect(worker.compiles).toHaveLength(2);
		expect(resolvedFits.get().get(layer.id)).toBe(curves);

		// The crop is part of the key too, so changing it still compiles.
		const settings = outputSettings.get();
		try {
			outputSettings.set({ ...settings, crop: { x: 0, y: 0, width: 2, height: 2 } });
			expect(worker.compiles).toHaveLength(3);
		} finally {
			outputSettings.set(settings);
		}
	});

	it('retains nothing across a new source', () => {
		addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		worker.reply(new Uint32Array([1]));

		sourceImageData.set(image());
		expect(worker.messages.filter((message) => message.type === 'source')).toHaveLength(2);
		expect(worker.compiles).toHaveLength(2);
	});

	it('does not load a new source early while every effect is off', () => {
		const layer = addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		worker.reply(new Uint32Array([1]));
		worker.index(new Uint32Array(4));
		setEffectEnabled(layer.id, false);

		sourceImageData.set(image());
		expect(effectsIndex.get()).toBeUndefined();
		expect(compiledEffects.get()).toBeUndefined();
		expect(worker.messages.filter((message) => message.type === 'source')).toHaveLength(1);

		setEffectEnabled(layer.id, true);
		const messages = worker.messages.map((message) => message.type);
		expect(messages.slice(-2)).toEqual(['source', 'compile']);
	});

	it('retains nothing across a worker crash', () => {
		vi.spyOn(console, 'error').mockImplementation(() => {});
		const { x } = twoLayers();
		const failed = ControlledWorker.instances[0]!;
		const resultsA = new Uint32Array([0xa]);
		failed.reply(resultsA);
		failed.reply(resultsA);

		setEffectEnabled(x.id, false);
		failed.reply(new Uint32Array([0xb]));
		setEffectEnabled(x.id, true);
		expect(compiledEffects.get()?.results).toBe(resultsA);

		failed.onerror?.(new Event('error'));
		setEffectEnabled(x.id, false);
		const fresh = ControlledWorker.instances[1]!;
		expect(fresh.messages.map((message) => message.type)).toEqual(['source', 'compile']);
	});

	it('does not retain a failed compile', () => {
		vi.spyOn(console, 'error').mockImplementation(() => {});
		const { x } = twoLayers();
		const worker = ControlledWorker.instances[0]!;
		worker.reply(new Uint32Array([0xa]));
		worker.reply(new Uint32Array([0xa]));

		setEffectEnabled(x.id, false);
		// The failure settles waiters, then update retries the still-current key at once.
		worker.fail('No memory.');
		expect(worker.compiles).toHaveLength(4);
		setEffectEnabled(x.id, true);
		expect(worker.compiles).toHaveLength(4);
		// Failing the retry as well keeps chain B out of the retained compiles.
		worker.fail('No memory.');
		setEffectEnabled(x.id, false);
		expect(worker.compiles).toHaveLength(5);
	});

	it('retains nothing across teardown', () => {
		addEffect('exposure');
		const first = ControlledWorker.instances[0]!;
		first.reply(new Uint32Array([1]));

		stop();
		stop = startLiveEffects();
		const next = ControlledWorker.instances[1]!;
		expect(next.messages.map((message) => message.type)).toEqual(['source', 'compile']);
	});

	it('evicts past the count bound', () => {
		const layer = addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		worker.reply(new Uint32Array([0]));
		for (let stops = 1; stops <= RETAINED_COMPILES; stops++) {
			updateEffect(layer.id, { effect: 'exposure', enabled: true, stops });
			worker.reply(new Uint32Array([stops]));
		}
		expect(worker.compiles).toHaveLength(RETAINED_COMPILES + 1);

		// The oldest key was evicted; the second-newest was not.
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 0 });
		expect(worker.compiles).toHaveLength(RETAINED_COMPILES + 2);
		worker.reply(new Uint32Array([0]));
		updateEffect(layer.id, {
			effect: 'exposure',
			enabled: true,
			stops: RETAINED_COMPILES - 1
		});
		expect(worker.compiles).toHaveLength(RETAINED_COMPILES + 2);
	});

	it('evicts past the byte bound', () => {
		addEffect('exposure');
		const y = addEffect('exposure');
		updateEffect(y.id, { effect: 'exposure', enabled: true, stops: 1 });
		const worker = ControlledWorker.instances[0]!;
		const big = () => new Uint32Array(RETAINED_BYTES / 8 + 1);
		worker.reply(big());
		worker.reply(big());

		setEffectEnabled(y.id, false);
		worker.reply(big());
		// Two entries over half the budget evicted the oldest, so chain A compiles again.
		setEffectEnabled(y.id, true);
		expect(worker.compiles).toHaveLength(4);
	});

	it('keeps the index and the canvas while every effect is off', () => {
		const layer = addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		const resultsA = new Uint32Array([0xa]);
		worker.reply(resultsA);
		const pixels = new Uint32Array(4);
		worker.index(pixels);
		const index = effectsIndex.get()!;

		setEffectEnabled(layer.id, false);
		expect(effectsIndex.get()).toBe(index);
		expect(shownSource.get()).toBeUndefined();
		expect(worker.terminate).not.toHaveBeenCalled();

		setEffectEnabled(layer.id, true);
		expect(worker.compiles).toHaveLength(1);
		expect(shownSource.get()?.index.pixels).toBe(pixels);
		expect(shownSource.get()?.results).toBe(resultsA);
	});
});
