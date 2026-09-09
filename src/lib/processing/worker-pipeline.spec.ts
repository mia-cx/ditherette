import { afterEach, describe, expect, it, vi } from 'vitest';
import { createDitherette, DitheretteError } from 'ditherette';
import { ProcessorWorkerPipeline, transferablesForWorkerResponse } from './worker-pipeline';
import type { DitherSettings, EnabledPaletteColor, OutputSettings, WorkerRequest } from './types';

class TestImageData implements ImageData {
	readonly data: Uint8ClampedArray<ArrayBuffer>;
	readonly width: number;
	readonly height: number;
	readonly colorSpace: PredefinedColorSpace = 'srgb';

	constructor(data: Uint8ClampedArray<ArrayBuffer> | number, width: number, height?: number) {
		this.width = typeof data === 'number' ? data : width;
		this.height = typeof data === 'number' ? width : height!;
		this.data =
			typeof data === 'number' ? new Uint8ClampedArray(this.width * this.height * 4) : data;
	}
}

Object.defineProperty(globalThis, 'ImageData', { value: TestImageData, configurable: true });

vi.mock('ditherette', () => ({
	createDitherette: vi.fn(),
	DitheretteError: class extends Error {
		constructor(
			public code: string,
			public path: string,
			message: string
		) {
			super(message);
		}
	}
}));
afterEach(() => {
	vi.unstubAllEnvs();
	vi.resetAllMocks();
});

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

function processorMock() {
	return {
		process: vi.fn(() => ({
			width: 2,
			height: 1,
			indices: new Uint8Array([0, 1]),
			palette: { rgba: new Uint8Array([0, 0, 0, 255, 255, 255, 255, 255]), transparentIndex: null },
			warnings: []
		})),
		resize: vi.fn(),
		quantize: vi.fn(),
		perturb: vi.fn(),
		ditherAndQuantize: vi.fn(),
		dispose: vi.fn()
	};
}

function loadedPipeline() {
	const pipeline = new ProcessorWorkerPipeline();
	pipeline.handle({ id: 1, type: 'load-source', sourceId: 'source-1', source: sourceImage() });
	return pipeline;
}

describe('ProcessorWorkerPipeline', () => {
	it.each(['initialization', 'capability'] as const)(
		'reports %s failure and retries on the next processing request',
		async (code) => {
			const error = new DitheretteError(code, 'wasm', 'Load failed');
			const processor = processorMock();
			vi.mocked(createDitherette).mockRejectedValueOnce(error).mockResolvedValue(processor);
			const pipeline = loadedPipeline();
			await expect(pipeline.handleAsync(processRequest(), () => undefined)).rejects.toMatchObject({
				message: 'Wasm could not initialize. Try processing again.',
				cause: error
			});
			expect(processor.process).not.toHaveBeenCalled();
			expect(await pipeline.handleAsync(processRequest({ id: 3 }), () => undefined)).toMatchObject({
				type: 'complete',
				image: { indices: new Uint8Array([0, 1]), settingsHash: 'hash' }
			});
			expect(createDitherette).toHaveBeenCalledTimes(2);
		}
	);

	it.each([
		'invalid-request',
		'memory-limit',
		'wasm-memory-unavailable',
		'callback',
		'runtime'
	] as const)('keeps %s initialization-boundary errors visible', async (code) => {
		const error = new DitheretteError(code, 'wasm', 'Visible failure');
		vi.mocked(createDitherette).mockRejectedValue(error);
		await expect(loadedPipeline().handleAsync(processRequest(), () => undefined)).rejects.toBe(
			error
		);
	});

	it('requires the current loaded source and replaces it for subsequent requests', async () => {
		const processor = processorMock();
		vi.mocked(createDitherette).mockResolvedValue(processor);
		const pipeline = new ProcessorWorkerPipeline();
		await expect(pipeline.handleAsync(processRequest(), () => undefined)).rejects.toThrow(
			/source/i
		);
		const source = sourceImage();
		expect(pipeline.handle({ id: 1, type: 'load-source', sourceId: 'source-1', source })).toEqual({
			id: 1,
			type: 'source-loaded',
			sourceId: 'source-1'
		});
		await pipeline.handleAsync(processRequest(), () => undefined);
		const next = new ImageData(new Uint8ClampedArray([255, 255, 255, 255]), 1, 1);
		pipeline.handle({ id: 3, type: 'load-source', sourceId: 'source-2', source: next });
		await expect(pipeline.handleAsync(processRequest(), () => undefined)).rejects.toThrow(
			/source/i
		);
		await pipeline.handleAsync(processRequest({ id: 4, sourceId: 'source-2' }), () => undefined);
		expect(processor.process.mock.calls.at(-1)).toEqual([
			expect.objectContaining({ source: { width: 1, height: 1, data: new Uint8Array(next.data) } })
		]);
		expect(createDitherette).toHaveBeenCalledOnce();
	});

	it('uses one scalar public processor and transfers only completed index buffers', async () => {
		const processor = processorMock();
		vi.mocked(createDitherette).mockResolvedValue(processor);
		const pipeline = loadedPipeline();
		const response = await pipeline.handleAsync(processRequest(), () => undefined);
		await pipeline.handleAsync(processRequest({ id: 3 }), () => undefined);
		expect(response).toMatchObject({
			type: 'complete',
			image: {
				indices: new Uint8Array([0, 1]),
				palette,
				transparentIndex: -1,
				settingsHash: 'hash'
			}
		});
		expect(createDitherette).toHaveBeenCalledExactlyOnceWith();
		expect(processor.process).toHaveBeenCalledTimes(2);
		if (response?.type !== 'complete') throw new Error('Expected completed output.');
		expect(transferablesForWorkerResponse(response)).toEqual([response.image.indices.buffer]);
		expect(
			transferablesForWorkerResponse({ id: 1, type: 'source-loaded', sourceId: 'source-1' })
		).toEqual([]);
	});

	it('does not initialize a canceled request', async () => {
		const pipeline = loadedPipeline();
		pipeline.handle({ id: 2, type: 'cancel' });
		expect(await pipeline.handleAsync(processRequest(), () => undefined)).toBeUndefined();
		expect(createDitherette).not.toHaveBeenCalled();
	});

	it('does not process a request canceled while scalar initialization is pending', async () => {
		const pending = Promise.withResolvers<Awaited<ReturnType<typeof createDitherette>>>();
		vi.mocked(createDitherette).mockReturnValue(pending.promise);
		const processor = processorMock();
		const pipeline = loadedPipeline();
		const response = pipeline.handleAsync(processRequest(), () => undefined);
		pipeline.handle({ id: 2, type: 'cancel' });
		pending.resolve(processor);
		expect(await response).toBeUndefined();
		expect(processor.process).not.toHaveBeenCalled();
	});

	it('forwards public work counts and skipped stages without inventing intermediate work', async () => {
		const processor = processorMock();
		vi.mocked(createDitherette).mockResolvedValue({
			...processor,
			process(request) {
				request.onProgress?.({ stage: 'quantize', completed: 1, total: 2 });
				request.onProgress?.({ stage: 'complete' });
				return processor.process();
			}
		});
		const progress = vi.fn();
		await loadedPipeline().handleAsync(processRequest(), progress);
		expect(progress.mock.calls).toContainEqual(['quantize', 0.5, { completed: 1, total: 2 }]);
		expect(progress.mock.calls.at(-1)).toEqual([
			'complete',
			1,
			{ completed: undefined, total: undefined }
		]);
		expect(progress.mock.calls.some(([stage]) => stage === 'resize')).toBe(false);
	});

	it.each(['initialization', 'memory-limit', 'callback', 'runtime'] as const)(
		'keeps processing %s failures visible without repeating initialization',
		async (code) => {
			const error = new DitheretteError(code, 'process', 'Processing failed');
			const processor = processorMock();
			processor.process.mockImplementationOnce(() => {
				throw error;
			});
			vi.mocked(createDitherette).mockResolvedValue(processor);
			const pipeline = loadedPipeline();
			await expect(pipeline.handleAsync(processRequest(), () => undefined)).rejects.toBe(error);
			expect(await pipeline.handleAsync(processRequest({ id: 3 }), () => undefined)).toMatchObject({
				type: 'complete'
			});
			expect(createDitherette).toHaveBeenCalledOnce();
		}
	);
});
