import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { compiledEffects, currentEffectsKey, effectsIndex } from '$lib/processing/live-effects';
import { colorSpace, sourceImageData } from '$lib/stores/app';
import {
	addEffect,
	effectLayers,
	removeEffect,
	updateEffect,
	type EffectLayer
} from '$lib/stores/effects';
import { MAX_SAMPLES, sampleSource } from './scopes';
import { effectsStage, pickStages, sampleEffected } from './stages';

const image = (width: number, height: number, fill: readonly number[] = [10, 20, 30, 255]) => {
	const data = new Uint8ClampedArray(width * height * 4);
	for (let pixel = 0; pixel < width * height; pixel++) data.set(fill, pixel * 4);
	return { data, width, height } as ImageData;
};

describe('sampleEffected', () => {
	it("maps each pixel through its colour's result, skipping only fully transparent ones", () => {
		// Two pixels share colour 0 (result: red) and one holds colour 1 (result: green).
		const pixels = new Uint32Array([
			0, // alpha 0: skipped
			(128 << 24) | 0, // 50% alpha: kept, red
			(255 << 24) | 1 // opaque, green
		]);
		const results = new Uint32Array([0x0000ff, 0x00ff00]);
		const samples = sampleEffected({ kind: 'effected', width: 3, height: 1, pixels, results });
		expect(samples.count).toBe(2);
		expect([...samples.rgb.subarray(0, 6)]).toEqual([255, 0, 0, 0, 255, 0]);
		expect([...samples.column.subarray(0, 2)]).toEqual([1.5 / 3, 2.5 / 3].map(Math.fround));
	});

	it('samples the same positions and colours as sampleSource on identity results', () => {
		const [width, height] = [2048, 1024];
		const data = new Uint8ClampedArray(width * height * 4);
		const slots = new Map<number, number>();
		const pixels = new Uint32Array(width * height);
		for (let pixel = 0; pixel < width * height; pixel++) {
			const [r, g, b] = [
				(pixel * 37) & 0xff,
				(pixel * 91 + (pixel >> 8)) & 0xff,
				(pixel * 53) & 0xff
			];
			data.set([r, g, b, 255], pixel * 4);
			// The colour and its result pack the same way: red in the low byte.
			const colour = r | (g << 8) | (b << 16);
			if (!slots.has(colour)) slots.set(colour, slots.size);
			pixels[pixel] = (slots.get(colour)! | (255 << 24)) >>> 0;
		}
		const results = new Uint32Array(slots.size);
		for (const [colour, slot] of slots) results[slot] = colour;
		const source = { data, width, height } as ImageData;
		const direct = sampleSource(source);
		const mapped = sampleEffected({ kind: 'effected', width, height, pixels, results });
		expect(mapped.count).toBe(direct.count);
		expect(mapped.count).toBeLessThanOrEqual(MAX_SAMPLES);
		expect([...mapped.column.subarray(0, mapped.count)]).toEqual([
			...direct.column.subarray(0, direct.count)
		]);
		expect([...mapped.rgb.subarray(0, mapped.count * 3)]).toEqual([
			...direct.rgb.subarray(0, direct.count * 3)
		]);
	});

	it('stays within the sample budget on a huge index', () => {
		const [width, height] = [4096, 4096];
		const pixels = new Uint32Array(width * height).fill(255 << 24);
		const samples = sampleEffected({
			kind: 'effected',
			width,
			height,
			pixels,
			results: new Uint32Array([0])
		});
		expect(samples.count).toBeLessThanOrEqual(MAX_SAMPLES);
	});
});

describe('effectsStage', () => {
	const key = () => currentEffectsKey();
	let layer: EffectLayer;
	let unsubscribe: () => void;

	beforeEach(() => {
		effectLayers.set([]);
		effectsIndex.set(undefined);
		compiledEffects.set(undefined);
		sourceImageData.set(undefined);
		// The panel always listens while it shows; a live listener also keeps `previous`.
		unsubscribe = effectsStage.subscribe(() => {});
	});

	afterEach(() => {
		unsubscribe();
	});

	/** Publish an index/results pair for `source`, as if the effects worker answered. */
	const publish = (source: ImageData, compileKey = key()) => {
		const pixels = new Uint32Array(source.width * source.height).fill((255 << 24) | 0);
		effectsIndex.set({ source, pixels, count: 1 });
		compiledEffects.set({ source, key: compileKey, results: new Uint32Array([0x302010]) });
	};

	it('is undefined without a source', () => {
		expect(effectsStage.get()).toBeUndefined();
	});

	it('is the source itself, current, when no step is enabled', () => {
		const source = image(4, 4);
		sourceImageData.set(source);
		// Stale results from an earlier edit never change that.
		publish(source, 'old-key');
		const stage = effectsStage.get();
		expect(stage).toEqual({
			image: { kind: 'source', source },
			current: true
		});
	});

	it('maps through the results, current, when index and compile match the source and key', () => {
		const source = image(4, 4);
		sourceImageData.set(source);
		layer = addEffect('exposure');
		publish(source);
		const stage = effectsStage.get()!;
		expect(stage.current).toBe(true);
		expect(stage.image).toMatchObject({ kind: 'effected', width: 4, height: 4 });
	});

	it('keeps the old results as a dimmed stand-in after a step edit', () => {
		const source = image(4, 4);
		sourceImageData.set(source);
		layer = addEffect('exposure');
		publish(source);
		const before = effectsStage.get()!;
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 2 });
		const stage = effectsStage.get()!;
		expect(stage.image).toBe(before.image);
		expect(stage.current).toBe(false);
	});

	it('has no image for a source the index and compile do not belong to', () => {
		const first = image(4, 4);
		sourceImageData.set(first);
		layer = addEffect('exposure');
		publish(first);
		sourceImageData.set(image(4, 4));
		expect(effectsStage.get()).toEqual({ current: false });
	});

	it('never pairs this source with an index built for another', () => {
		sourceImageData.set(image(4, 4));
		layer = addEffect('exposure');
		publish(image(4, 4, [1, 2, 3, 255]));
		expect(effectsStage.get()).toEqual({ current: false });
	});

	it('stops being current when an analysing step reads a changed palette or space', async () => {
		const source = image(4, 4);
		sourceImageData.set(source);
		layer = addEffect('palette-fit'); // curves: null — it analyses the palette
		publish(source);
		expect(effectsStage.get()!.current).toBe(true);
		const { customPalettes, activePaletteName, selectedPalette } = await import('$lib/stores/app');
		const restore = activePaletteName.get();
		const custom = {
			name: 'Two',
			source: 'custom' as const,
			colors: selectedPalette.get().slice(0, 2)
		};
		customPalettes.set([custom]);
		activePaletteName.set('Two');
		expect(effectsStage.get()!.current).toBe(false);
		activePaletteName.set(restore);
		customPalettes.set([]);
		// A recolour without a recipe always reads the working space.
		removeEffect(layer.id);
		layer = addEffect('recolour');
		publish(source);
		expect(effectsStage.get()!.current).toBe(true);
		colorSpace.set('srgb');
		expect(effectsStage.get()!.current).toBe(false);
		colorSpace.set('oklab');
	});

	it('forgets the retained image once nobody listens', () => {
		vi.useFakeTimers();
		try {
			const first = image(4, 4);
			sourceImageData.set(first);
			layer = addEffect('exposure');
			publish(first);
			expect(effectsStage.value?.image).toBeDefined();
			const off = unsubscribe;
			unsubscribe = () => {};
			off();
			vi.advanceTimersByTime(1000);
			// `.value`, not `.get()`: get mounts briefly and would hide the leak.
			expect(effectsStage.value).toBeUndefined();
			sourceImageData.set(undefined);
			effectsIndex.set(undefined);
			compiledEffects.set(undefined);
			const resubscribe = effectsStage.subscribe(() => {});
			expect(effectsStage.get()).toBeUndefined();
			resubscribe();
		} finally {
			vi.useRealTimers();
		}
	});

	it('returns the identical object when an unrelated store changes', () => {
		const source = image(4, 4);
		sourceImageData.set(source);
		layer = addEffect('exposure');
		publish(source);
		const before = effectsStage.get();
		colorSpace.set('srgb'); // exposure is not analysing: the key is unchanged
		expect(effectsStage.get()).toBe(before);
		colorSpace.set('oklab');
	});
});

describe('pickStages', () => {
	it('keeps up to two valid stages in pipeline order', () => {
		expect(pickStages(['output', 'source'], ['source', 'output'])).toEqual(['source', 'output']);
		expect(pickStages(['effects'], ['effects'])).toEqual(['effects']);
	});

	it('drops the earliest in recent when three are on', () => {
		expect(pickStages(['source', 'effects', 'output'], ['source', 'effects', 'output'])).toEqual([
			'effects',
			'output'
		]);
		expect(pickStages(['source', 'effects', 'output'], ['effects', 'output', 'source'])).toEqual([
			'source',
			'output'
		]);
	});

	it('ignores unknown ids', () => {
		expect(pickStages(['source', 'nonsense', 'output'], ['source', 'output'])).toEqual([
			'source',
			'output'
		]);
	});
});
