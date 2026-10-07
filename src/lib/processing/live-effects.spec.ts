import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { sourceImageData } from '$lib/stores/app';
import {
	addEffect,
	effectLayers,
	removeEffect,
	setEffectEnabled,
	setEffectMask,
	updateEffect
} from '$lib/stores/effects';
import { shownSource } from './effects-view.svelte';
import {
	analysedFits,
	compiledEffects,
	compiledEffectsFor,
	currentEffectsKey,
	effectsKey,
	effectsIndex,
	maskInputs,
	shownMask,
	startLiveEffects,
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
	/** Answer the latest compile. */
	reply(results = new Uint32Array([1]), fits?: { index: number; curves: readonly object[] }[]) {
		const request = this.compiles.at(-1)!;
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

	it('publishes curves under the layers that produced them, not the order at reply', () => {
		const first = addEffect('palette-fit');
		const worker = ControlledWorker.instances.at(-1)!;
		worker.reply(new Uint32Array([1]), [{ index: 0, curves }]);
		const second = addEffect('palette-fit');
		// Remove the first fit while the two-step compile is in flight: the second fit's
		// curves still land under its own id.
		removeEffect(first.id);
		worker.reply(new Uint32Array([1]), [{ index: 1, curves }]);
		expect(analysedFits.get()).toEqual(new Map([[second.id, curves]]));
	});
});
