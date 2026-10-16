import { afterEach, expect, it, vi } from 'vitest';
import {
	buildOutputPyramid,
	outputPyramidBitmaps,
	releaseLevels,
	withOutputPreviewLevel
} from './output-preview';
import type { ProcessedImage } from './types';
import { drawProcessedImage, processedToImageData } from './render';
import { processedImage, setOutputPreview } from '$lib/stores/app';

function image(width = 3, height = 3): ProcessedImage {
	return {
		width,
		height,
		indices: Uint8Array.from({ length: width * height }, (_, index) => index % 2),
		palette: [
			{ name: 'Red', key: '#ff0000', rgb: { r: 255, g: 0, b: 0 }, kind: 'free', enabled: true },
			{ name: 'Clear', key: 'transparent', kind: 'transparent', enabled: true }
		],
		transparentIndex: 1,
		warnings: [],
		settingsHash: 'exact',
		updatedAt: 1
	};
}

afterEach(() => {
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
});

it('bounds retained mip pixels and RGBA staging while keeping full-size output', () => {
	const puts = vi.spyOn(OffscreenCanvasRenderingContext2D.prototype, 'putImageData');
	const levels = buildOutputPyramid(image(4096, 4097));
	try {
		expect([levels[0].width, levels[0].height]).toEqual([4096, 4097]);
		expect(
			levels.slice(1).reduce((bytes, level) => bytes + level.width * level.height * 4, 0)
		).toBeLessThanOrEqual(16 * 1024 * 1024);
		expect(Math.max(...puts.mock.calls.map(([data]) => data.data.byteLength))).toBeLessThanOrEqual(
			1024 * 1024
		);
		expect(levels.at(-1)?.width).toBe(1);
		expect(levels.at(-1)?.height).toBe(1);
	} finally {
		releaseLevels(levels);
	}
});

it('releases canvas pixels when replaced or cleared', () => {
	const levels = buildOutputPyramid(image());
	releaseLevels(levels);
	expect(levels.map((level) => [level.width, level.height])).toEqual([
		[0, 0],
		[0, 0],
		[0, 0]
	]);
});

it('preserves exact indexed pixels, alpha, and repeated odd edges in retained mip levels', () => {
	const output = image();
	const levels = buildOutputPyramid(output);
	try {
		const read = (level: OffscreenCanvas | HTMLCanvasElement) => {
			const context =
				level instanceof OffscreenCanvas ? level.getContext('2d')! : level.getContext('2d')!;
			return Array.from(context.getImageData(0, 0, level.width, level.height).data);
		};
		expect(read(levels[0])).toEqual([
			255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0,
			255, 0, 0, 0, 0, 255, 0, 0, 255
		]);
		// Chromium's 2x bilinear path truncates alpha at each mip.
		expect(read(levels[1])).toEqual([
			255, 0, 0, 127, 255, 0, 0, 127, 255, 0, 0, 127, 255, 0, 0, 255
		]);
		expect(read(levels[2])).toEqual([255, 0, 0, 159]);
		expect(output.indices).toEqual(new Uint8Array([0, 1, 0, 1, 0, 1, 0, 1, 0]));
	} finally {
		releaseLevels(levels);
	}
});

it('draws every strip and its partial final row without changing full RGBA rendering', () => {
	const output = image(257, 1023);
	const canvas = new OffscreenCanvas(output.width, output.height);
	const context = canvas.getContext('2d')!;
	drawProcessedImage(context, output);
	try {
		for (const y of [0, 1019, 1020, 1022]) {
			expect(Array.from(context.getImageData(0, y, 2, 1).data)).toEqual(
				y % 2 === 0 ? [255, 0, 0, 255, 0, 0, 0, 0] : [0, 0, 0, 0, 255, 0, 0, 255]
			);
		}
		expect(Array.from(processedToImageData(image(2, 1)).data)).toEqual([
			255, 0, 0, 255, 0, 0, 0, 0
		]);
	} finally {
		releaseLevels([canvas]);
	}
});

it('releases partial levels and padding when drawing fails', () => {
	const contexts = vi.spyOn(OffscreenCanvas.prototype, 'getContext');
	vi.spyOn(OffscreenCanvasRenderingContext2D.prototype, 'drawImage').mockImplementation(() => {
		throw new Error('drawing failed');
	});
	expect(() => buildOutputPyramid(image())).toThrow('drawing failed');
	expect(
		contexts.mock.results.map(({ value }) => [value.canvas.width, value.canvas.height])
	).toEqual([
		[0, 0],
		[0, 0]
	]);
});

it('releases a newly allocated canvas when context acquisition throws', () => {
	const contexts = vi.spyOn(OffscreenCanvas.prototype, 'getContext').mockImplementationOnce(() => {
		throw new Error('context failed');
	});
	expect(() => buildOutputPyramid(image())).toThrow('context failed');
	const allocated = contexts.mock.contexts[0] as OffscreenCanvas;
	expect([allocated.width, allocated.height]).toEqual([0, 0]);
});

it('disposes a reconstructed prefilter when the final consumer fails without releasing retained pixels', () => {
	const levels = [new OffscreenCanvas(16, 16), new OffscreenCanvas(2, 2)];
	const contexts = vi.spyOn(OffscreenCanvas.prototype, 'getContext');
	try {
		expect(() =>
			withOutputPreviewLevel(levels, 3, () => {
				throw new Error('destination failed');
			})
		).toThrow('destination failed');
		expect(
			contexts.mock.results.map(({ value }) => [value.canvas.width, value.canvas.height])
		).toEqual([
			[0, 0],
			[0, 0]
		]);
		expect(levels.map((level) => [level.width, level.height])).toEqual([
			[16, 16],
			[2, 2]
		]);
	} finally {
		releaseLevels(levels);
	}
});

it('closes transferred bitmaps and releases all canvases when a later transfer fails', () => {
	const contexts = vi.spyOn(OffscreenCanvas.prototype, 'getContext');
	const transfer = OffscreenCanvas.prototype.transferToImageBitmap;
	let first: ImageBitmap | undefined;
	vi.spyOn(OffscreenCanvas.prototype, 'transferToImageBitmap').mockImplementation(function (
		this: OffscreenCanvas
	) {
		if (first) {
			expect(contexts.mock.results[0].value.canvas.width).toBe(0);
			throw new Error('transfer failed');
		}
		return (first = transfer.call(this));
	});
	expect(() => outputPyramidBitmaps(image())).toThrow('transfer failed');
	expect(first?.width).toBe(0);
	expect(
		contexts.mock.results.every(
			({ value }) => value.canvas.width === 0 && value.canvas.height === 0
		)
	).toBe(true);
});

it('returns transferable full-size pixels and closes them when superseded', () => {
	const levels = outputPyramidBitmaps(image())!;
	expect(levels.map((level) => [level.width, level.height])).toEqual([
		[3, 3],
		[2, 2],
		[1, 1]
	]);
	releaseLevels(levels);
	expect(levels.every((level) => level.width === 0 && level.height === 0)).toBe(true);
});

it('disposes replaced and cleared output previews through their public store ownership', () => {
	const first = image();
	const firstLevels = outputPyramidBitmaps(first)!;
	setOutputPreview({ image: first, levels: firstLevels });
	processedImage.set(first);
	const next = image();
	const nextLevels = buildOutputPyramid(next);
	setOutputPreview({ image: next, levels: nextLevels });
	processedImage.set(next);
	expect(firstLevels.every((level) => level.width === 0)).toBe(true);
	processedImage.set(undefined);
	expect(nextLevels.every((level) => level.width === 0 && level.height === 0)).toBe(true);
});

it('draws and releases HTML canvas fallback levels when offscreen drawing is unavailable', () => {
	vi.stubGlobal('OffscreenCanvas', undefined);
	expect(outputPyramidBitmaps(image())).toBeUndefined();
	const levels = buildOutputPyramid(image(2, 1));
	try {
		expect(levels[0]).toBeInstanceOf(HTMLCanvasElement);
		const context = (levels[0] as HTMLCanvasElement).getContext('2d')!;
		expect(Array.from(context.getImageData(0, 0, 2, 1).data)).toEqual([255, 0, 0, 255, 0, 0, 0, 0]);
	} finally {
		releaseLevels(levels);
	}
	expect(levels.every((level) => level.width === 0 && level.height === 0)).toBe(true);
});
