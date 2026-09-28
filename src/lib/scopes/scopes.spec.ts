import { describe, expect, it } from 'vitest';
import type { ProcessedImage } from '$lib/processing/types';
import { chromaticity, D65, vectorPlane } from './colour';
import {
	chromaticityBackdrop,
	chromaticityScope,
	histogram,
	LEVELS,
	PLANE_SIZE,
	sampleOutput,
	sampleSource,
	vectorscope,
	waveform,
	WAVE_COLUMNS
} from './scopes';

const row = (...pixels: [number, number, number, number][]) => ({
	data: new Uint8ClampedArray(pixels.flat()),
	width: pixels.length,
	height: 1
});
/** Each channel's non-empty bins, as `bin: weight`. */
const filled = (bins: readonly Float32Array[]) =>
	bins.map((channel) =>
		Object.fromEntries([...channel.entries()].filter(([, weight]) => weight > 0))
	);
const total = (bins: Float32Array) => bins.reduce((sum, weight) => sum + weight, 0);

describe('scopes', () => {
	it('keeps every pixel of a small image, and skips transparent ones', () => {
		const samples = sampleSource(row([255, 0, 0, 255], [0, 0, 255, 0], [0, 0, 255, 255]));
		expect(samples.count).toBe(2);
		expect([...samples.rgb.subarray(0, 6)]).toEqual([255, 0, 0, 0, 0, 255]);
		expect([...samples.column.subarray(0, 2)]).toEqual([0.5 / 3, 2.5 / 3].map(Math.fround));
	});

	it('bins levels on the normalised axes the curves use', () => {
		const red = sampleSource(row([255, 0, 0, 255]));
		expect(filled(histogram(red, 'srgb'))).toEqual([{ [LEVELS - 1]: 1 }, { 0: 1 }, { 0: 1 }]);
		expect(filled(histogram(red, 'cmy'))).toEqual([
			{ 0: 1 },
			{ [LEVELS - 1]: 1 },
			{ [LEVELS - 1]: 1 }
		]);
		// White sits at full lightness with neutral opponent axes.
		const white = sampleSource(row([255, 255, 255, 255]));
		expect(filled(histogram(white, 'oklab')).map(Object.keys)).toEqual([
			[String(LEVELS - 1)],
			[String(LEVELS / 2)],
			[String(LEVELS / 2)]
		]);
	});

	it('leaves greys out of hue traces', () => {
		const samples = sampleSource(row([128, 128, 128, 255], [255, 0, 0, 255]));
		for (const model of ['hsl', 'oklch', 'cielch'] as const) {
			const hue = histogram(samples, model)[model === 'hsl' ? 0 : 2];
			expect(total(hue)).toBe(1);
		}
		expect(total(histogram(samples, 'cielab')[0])).toBe(2);
	});

	it('places waveform samples by column and level, top level first', () => {
		const [red] = waveform(sampleSource(row([255, 0, 0, 255], [0, 0, 0, 255])), 'srgb');
		expect(red[0 * WAVE_COLUMNS + WAVE_COLUMNS / 4]).toBe(1);
		expect(red[(LEVELS - 1) * WAVE_COLUMNS + (3 * WAVE_COLUMNS) / 4]).toBe(1);
	});

	it('puts greys at the centre of the vectorscope', () => {
		const { density } = vectorscope(sampleSource(row([90, 90, 90, 255])), vectorPlane('srgb'));
		const centre = PLANE_SIZE / 2;
		expect(density[centre * PLANE_SIZE + centre]).toBe(1);
	});

	it('puts white at D65 on the chromaticity diagram, inside the spectral locus', () => {
		const [x, y] = chromaticity(255, 255, 255)!;
		expect(x).toBeCloseTo(D65[0], 3);
		expect(y).toBeCloseTo(D65[1], 3);
		const { density } = chromaticityScope(sampleSource(row([255, 255, 255, 255])));
		const cell = density.findIndex((weight) => weight > 0);
		expect(chromaticityBackdrop().density[cell]).toBe(1);
	});

	it('samples the output through its palette, without transparent pixels', () => {
		const output = {
			width: 5,
			height: 1,
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
		expect(filled(histogram(sampleOutput(output), 'srgb'))[0]).toEqual({ 0: 3, [LEVELS - 1]: 1 });
	});
});
