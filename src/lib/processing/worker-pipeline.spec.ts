import { describe, expect, it } from 'vitest';
import { ProcessorWorkerPipeline, transferablesForWorkerResponse } from './worker-pipeline';
import type {
	DitherSettings,
	EnabledPaletteColor,
	OutputSettings,
	ProcessingSettings,
	WorkerRequest
} from './types';

class TestImageData implements ImageData {
	readonly data: Uint8ClampedArray<ArrayBuffer>;
	readonly width: number;
	readonly height: number;
	readonly colorSpace: PredefinedColorSpace = 'srgb';

	constructor(data: Uint8ClampedArray<ArrayBuffer>, width: number, height: number) {
		this.data = data;
		this.width = width;
		this.height = height;
	}
}

Object.defineProperty(globalThis, 'ImageData', { value: TestImageData, configurable: true });

const palette: EnabledPaletteColor[] = [
	{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true },
	{ name: 'White', key: '#FFFFFF', rgb: { r: 255, g: 255, b: 255 }, kind: 'free', enabled: true }
];

const output: OutputSettings = {
	width: 2,
	height: 1,
	lockAspect: true,
	resize: 'nearest',
	alphaMode: 'preserve',
	alphaThreshold: 0,
	matteKey: '#FFFFFF',
	autoSizeOnUpload: false,
	scaleFactor: 1
};

const dither: DitherSettings = {
	algorithm: 'none',
	strength: 100,
	placement: 'everywhere',
	placementRadius: 3,
	placementThreshold: 12,
	placementSoftness: 8,
	serpentine: true,
	seed: 1,
	useColorSpace: false
};

function sourceImage() {
	return new ImageData(new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255]), 2, 1);
}

function processRequest(overrides: Partial<Extract<WorkerRequest, { type: 'process' }>> = {}) {
	return {
		id: 2,
		type: 'process',
		sourceId: 'source-1',
		settings: { output, dither, colorSpace: 'srgb' },
		palette,
		settingsHash: 'hash',
		...overrides
	} satisfies WorkerRequest;
}

describe('ProcessorWorkerPipeline', () => {
	it('loads a source before processing', () => {
		const pipeline = new ProcessorWorkerPipeline();

		const response = pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() },
			() => undefined
		);

		expect(response).toEqual({ id: 1, type: 'source-loaded', sourceId: 'source-1' });
	});

	it('rejects processing for an unknown source id', () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() },
			() => undefined
		);

		expect(() =>
			pipeline.handle(processRequest({ sourceId: 'missing-source' }), () => undefined)
		).toThrow(/source/i);
	});

	it('reuses resized pixels for settings-only processing changes', () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() },
			() => undefined
		);
		pipeline.handle(processRequest(), () => undefined);
		const progressStages: string[] = [];

		const response = pipeline.handle(
			processRequest({
				id: 3,
				settings: {
					output,
					dither: { ...dither, algorithm: 'bayer-2' },
					colorSpace: 'srgb'
				},
				settingsHash: 'hash-2'
			}),
			(stage) => progressStages.push(stage)
		);

		expect(progressStages).toContain('Using cached resize');
		expect(response).toMatchObject({ type: 'complete' });
		expect(pipeline.resizeCacheSize).toBe(1);
	});

	it('reuses quantized output when toggling back to a previous dither branch', () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() },
			() => undefined
		);
		const adaptiveSettings: ProcessingSettings = {
			output,
			dither: { ...dither, algorithm: 'bayer-2', placement: 'adaptive' },
			colorSpace: 'oklab'
		};
		const everywhereSettings: ProcessingSettings = {
			...adaptiveSettings,
			dither: { ...adaptiveSettings.dither, placement: 'everywhere' as const }
		};
		const first = pipeline.handle(
			processRequest({ settings: adaptiveSettings, settingsHash: 'adaptive' }),
			() => undefined
		);
		pipeline.handle(
			processRequest({ id: 3, settings: everywhereSettings, settingsHash: 'everywhere' }),
			() => undefined
		);
		const progressStages: string[] = [];

		const second = pipeline.handle(
			processRequest({ id: 4, settings: adaptiveSettings, settingsHash: 'adaptive' }),
			(stage) => progressStages.push(stage)
		);

		expect(progressStages).toContain('Using cached quantization');
		if (!first || first.type !== 'complete' || !second || second.type !== 'complete') {
			throw new Error('Expected complete responses.');
		}
		expect([...second.image.indices]).toEqual([...first.image.indices]);
		expect(second.image.indices).not.toBe(first.image.indices);
	});

	it('creates prior branches for resize-affecting changes', () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() },
			() => undefined
		);

		pipeline.handle(processRequest(), () => undefined);
		pipeline.handle(
			processRequest({
				id: 3,
				settings: { output: { ...output, width: 1 }, dither, colorSpace: 'srgb' },
				settingsHash: 'hash-2'
			}),
			() => undefined
		);

		expect(pipeline.branchCacheSize).toBe(2);
	});

	it('clears active and prior branches when a new source loads', () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() },
			() => undefined
		);
		pipeline.handle(processRequest(), () => undefined);
		expect(pipeline.branchCacheSize).toBe(1);

		pipeline.handle(
			{ id: 3, type: 'load-source', sourceId: 'source-2', source: sourceImage() },
			() => undefined
		);

		expect(pipeline.branchCacheSize).toBe(0);
	});

	it('includes processing metrics on complete responses', () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() },
			() => undefined
		);

		const response = pipeline.handle(processRequest(), () => undefined);

		if (!response || response.type !== 'complete') throw new Error('Expected complete response.');
		expect(response.metrics).toMatchObject({
			id: 2,
			sourceId: 'source-1',
			settingsHash: 'hash',
			outputPixels: 2,
			resize: 'nearest'
		});
		expect(response.metrics?.timings.length).toBeGreaterThan(0);
		expect(response.metrics?.cache.delta.resizedMisses).toBe(1);
		expect(response.metrics?.memory.resizedBytes).toBe(8);
	});

	it('splits quantize metrics into color conversion and dither matching stages', () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() },
			() => undefined
		);

		const response = pipeline.handle(
			processRequest({
				settings: {
					output,
					dither: { ...dither, algorithm: 'bayer-2', useColorSpace: true },
					colorSpace: 'oklab'
				}
			}),
			() => undefined
		);

		if (!response || response.type !== 'complete') throw new Error('Expected complete response.');
		const timingNames = response.metrics?.timings.map((timing) => timing.name);
		expect(timingNames).toEqual(
			expect.arrayContaining([
				'color space convert palette cache lookup',
				'color space convert palette',
				'color space convert composited image cache lookup',
				'quantize direct dither+match loop'
			])
		);
	});

	it('records cache hits and replays cached compute timings in metrics', () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() },
			() => undefined
		);
		const first = pipeline.handle(processRequest(), () => undefined);
		if (!first || first.type !== 'complete') throw new Error('Expected complete response.');
		const firstResizeMs = first.metrics?.timings.find(
			(timing) => timing.name === 'resize compute'
		)?.ms;
		const firstQuantizeMs = first.metrics?.timings.find(
			(timing) => timing.name === 'quantize compute'
		)?.ms;

		const response = pipeline.handle(processRequest({ id: 3 }), () => undefined);

		if (!response || response.type !== 'complete') throw new Error('Expected complete response.');
		expect(response.metrics?.cache.delta.resizedHits).toBe(1);
		expect(response.metrics?.cache.delta.derivedHits).toBeGreaterThan(0);
		expect(response.metrics?.timings).toEqual(
			expect.arrayContaining([
				expect.objectContaining({ name: 'resize compute', ms: firstResizeMs, replayed: true }),
				expect.objectContaining({ name: 'quantize compute', ms: firstQuantizeMs, replayed: true })
			])
		);
	});

	it('returns transferred index buffers for complete responses', () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() },
			() => undefined
		);
		const response = pipeline.handle(processRequest(), () => undefined);
		if (!response || response.type !== 'complete') throw new Error('Expected complete response.');

		expect(transferablesForWorkerResponse(response)).toEqual([response.image.indices.buffer]);
	});
});
