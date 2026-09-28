import { describe, expect, it } from 'vitest';
import type { ProcessedImage } from '$lib/processing/types';
import { BINS, outputHistogram, sourceHistogram, type Histogram } from './histogram';

const image = (...pixels: [number, number, number, number][]) => ({
	data: new Uint8ClampedArray(pixels.flat())
});
/** Each channel's non-empty bins, as `bin: weight`. */
const filled = (histogram: Histogram) =>
	histogram.map((bins) =>
		Object.fromEntries([...bins.entries()].filter(([, weight]) => weight > 0))
	);

describe('sourceHistogram', () => {
	it('bins on the normalised axes the curves use', () => {
		expect(filled(sourceHistogram(image([255, 0, 0, 255]), 'srgb'))).toEqual([
			{ [BINS - 1]: 1 },
			{ 0: 1 },
			{ 0: 1 }
		]);
		expect(filled(sourceHistogram(image([255, 0, 0, 255]), 'cmy'))).toEqual([
			{ 0: 1 },
			{ [BINS - 1]: 1 },
			{ [BINS - 1]: 1 }
		]);
		// White sits at full lightness with neutral opponent axes.
		const [lightness, a, b] = filled(sourceHistogram(image([255, 255, 255, 255]), 'oklab'));
		expect(Object.keys(lightness!)).toEqual([String(BINS - 1)]);
		expect(Object.keys(a!)).toEqual([String(BINS / 2)]);
		expect(Object.keys(b!)).toEqual([String(BINS / 2)]);
	});

	it('leaves greys out of hue plots and skips transparent pixels', () => {
		const pixels = image([128, 128, 128, 255], [255, 0, 0, 255], [0, 0, 255, 0]);
		for (const model of ['hsl', 'oklch', 'cielch'] as const) {
			const hue = sourceHistogram(pixels, model)[model === 'hsl' ? 0 : 2];
			expect(hue.reduce((sum, weight) => sum + weight, 0)).toBe(1);
		}
		const [lightness] = sourceHistogram(pixels, 'cielab');
		expect(lightness.reduce((sum, weight) => sum + weight, 0)).toBe(2);
	});
});

describe('outputHistogram', () => {
	it('weights each palette colour by its pixel count, without transparent pixels', () => {
		const processed = {
			indices: new Uint8Array([0, 0, 0, 1, 2]),
			palette: [
				{ name: 'Black', key: 'black', kind: 'free', enabled: true, rgb: { r: 0, g: 0, b: 0 } },
				{
					name: 'White',
					key: 'white',
					kind: 'free',
					enabled: true,
					rgb: { r: 255, g: 255, b: 255 }
				},
				{ name: 'Clear', key: 'clear', kind: 'transparent', enabled: true }
			]
		} as unknown as ProcessedImage;
		expect(filled(outputHistogram(processed, 'srgb'))[0]).toEqual({ 0: 3, [BINS - 1]: 1 });
	});
});
