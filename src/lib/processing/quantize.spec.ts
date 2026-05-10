import { describe, expect, it } from 'vitest';
import {
	quantizeImage,
	type ColorVectorImage,
	type PaletteVectorSpace,
	type QuantizeCaches
} from './quantize';
import type { DitherId, EnabledPaletteColor, ProcessingSettings } from './types';

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
	{ name: 'Red', key: '#FF0000', rgb: { r: 255, g: 0, b: 0 }, kind: 'free', enabled: true },
	{ name: 'White', key: '#FFFFFF', rgb: { r: 255, g: 255, b: 255 }, kind: 'free', enabled: true },
	{ name: 'Transparent', key: 'transparent', kind: 'transparent', enabled: true }
];

const baseSettings: ProcessingSettings = {
	output: {
		width: 3,
		height: 2,
		lockAspect: true,
		resize: 'nearest',
		alphaMode: 'matte',
		alphaThreshold: 0,
		matteKey: '#FFFFFF',
		autoSizeOnUpload: false,
		scaleFactor: 1
	},
	dither: {
		algorithm: 'none',
		strength: 100,
		placement: 'adaptive',
		placementRadius: 1,
		placementThreshold: 12,
		placementSoftness: 8,
		serpentine: true,
		seed: 7,
		useColorSpace: true
	},
	colorSpace: 'oklab'
};

function image() {
	return new ImageData(
		new Uint8ClampedArray([
			0, 0, 0, 255, 120, 40, 30, 180, 255, 255, 255, 255, 255, 0, 0, 255, 50, 80, 180, 220, 220,
			220, 20, 90
		]),
		3,
		2
	);
}

function cacheMaps(colorVectorImageScope?: string): QuantizeCaches & {
	paletteVectors: Map<string, PaletteVectorSpace>;
	images: Map<string, ColorVectorImage>;
	counts: Map<string, number>;
	timings: Map<string, number>;
} {
	const paletteVectors = new Map<string, PaletteVectorSpace>();
	const images = new Map<string, ColorVectorImage>();
	const counts = new Map<string, number>();
	const timings = new Map<string, number>();
	return {
		paletteVectors,
		images,
		counts,
		timings,
		colorVectorImageScope,
		recordCount: (key, amount = 1) => counts.set(key, (counts.get(key) ?? 0) + amount),
		recordTiming: (key, ms) => timings.set(key, (timings.get(key) ?? 0) + ms),
		getPaletteVectorSpace: (key) => paletteVectors.get(key),
		setPaletteVectorSpace: (key, value) => paletteVectors.set(key, value),
		getColorVectorImage: (key) => images.get(key),
		canStoreColorVectorImage: () => true,
		setColorVectorImage: (key, value) => images.set(key, value)
	};
}

function settingsForAlgorithm(algorithm: DitherId): ProcessingSettings {
	return { ...baseSettings, dither: { ...baseSettings.dither, algorithm } };
}

describe('quantizeImage caches', () => {
	it.each([
		['direct', settingsForAlgorithm('none')],
		['bayer vector dither', settingsForAlgorithm('bayer-2')],
		['random vector dither', settingsForAlgorithm('random')],
		['vector error diffusion', settingsForAlgorithm('floyd-steinberg')]
	])('matches uncached output for %s', (_name, settings) => {
		const uncached = quantizeImage(image(), palette, settings);
		const caches = cacheMaps('image-1');
		const cached = quantizeImage(image(), palette, settings, caches);

		expect([...cached.indices]).toEqual([...uncached.indices]);
		expect(caches.paletteVectors.size).toBeGreaterThan(0);
	});

	it('reuses cached color-vector images on repeated runs', () => {
		const caches = cacheMaps('image-1');
		quantizeImage(image(), palette, baseSettings, caches);
		const firstImages = caches.images.size;

		quantizeImage(image(), palette, baseSettings, caches);

		expect(caches.images.size).toBe(firstImages);
		expect(firstImages).toBeGreaterThan(0);
	});

	it('stores cached color-vector images as 32-bit floats', () => {
		const caches = cacheMaps('image-1');

		quantizeImage(image(), palette, baseSettings, caches);

		expect([...caches.images.values()].every((vectors) => vectors.v0 instanceof Float32Array)).toBe(
			true
		);
	});

	it('ignores stale color-vector images with different dimensions', () => {
		const caches = cacheMaps('image-1');
		quantizeImage(
			new ImageData(new Uint8ClampedArray([0, 0, 0, 255]), 1, 1),
			palette,
			baseSettings,
			caches
		);
		const nextImage = new ImageData(
			new Uint8ClampedArray([255, 255, 255, 255, 0, 0, 0, 255]),
			2,
			1
		);
		const uncached = quantizeImage(nextImage, palette, baseSettings);

		const cached = quantizeImage(nextImage, palette, baseSettings, caches);

		expect([...cached.indices]).toEqual([...uncached.indices]);
	});

	it('separates same-size color-vector images by scope', () => {
		const caches = cacheMaps('dark');
		quantizeImage(
			new ImageData(new Uint8ClampedArray([0, 0, 0, 255, 20, 20, 20, 255]), 2, 1),
			palette,
			baseSettings,
			caches
		);
		caches.colorVectorImageScope = 'light';
		const lightImage = new ImageData(
			new Uint8ClampedArray([255, 255, 255, 255, 240, 240, 240, 255]),
			2,
			1
		);
		const uncached = quantizeImage(lightImage, palette, baseSettings);

		const cached = quantizeImage(lightImage, palette, baseSettings, caches);

		expect([...cached.indices]).toEqual([...uncached.indices]);
	});

	it('requires a scope before caching color-vector images', () => {
		const caches = cacheMaps();

		quantizeImage(image(), palette, baseSettings, caches);

		expect(caches.images.size).toBe(0);
		expect(caches.paletteVectors.size).toBeGreaterThan(0);
	});

	it('records RGB matcher memo hits for repeated byte colors', () => {
		const caches = cacheMaps('rgb-repeats');
		const repeated = new ImageData(
			new Uint8ClampedArray([10, 20, 30, 255, 10, 20, 30, 255, 10, 20, 30, 255]),
			3,
			1
		);
		const settings: ProcessingSettings = {
			...baseSettings,
			colorSpace: 'srgb',
			dither: { ...baseSettings.dither, useColorSpace: false, algorithm: 'none' }
		};

		quantizeImage(repeated, palette, settings, caches);

		expect(caches.counts.get('rgb memo hit')).toBeGreaterThan(0);
		expect(caches.counts.get('rgb memo miss')).toBe(1);
	});

	it('records vector matcher memo hits for repeated vector colors', () => {
		const caches = cacheMaps('vector-repeats');
		const repeated = new ImageData(
			new Uint8ClampedArray([10, 20, 30, 255, 10, 20, 30, 255, 10, 20, 30, 255]),
			3,
			1
		);

		quantizeImage(repeated, palette, settingsForAlgorithm('none'), caches);

		expect(caches.counts.get('vector memo hit')).toBeGreaterThan(0);
		expect(caches.counts.get('vector memo miss')).toBe(1);
	});

	it('records quantize sub-stage timings', () => {
		const caches = cacheMaps('timings');

		quantizeImage(image(), palette, baseSettings, caches);

		expect(caches.timings.get('palette prepare')).toBeGreaterThanOrEqual(0);
		expect(caches.timings.get('matcher build')).toBeGreaterThanOrEqual(0);
		expect(caches.timings.get('color space convert palette cache lookup')).toBeGreaterThanOrEqual(
			0
		);
		expect(caches.timings.get('color space convert palette')).toBeGreaterThanOrEqual(0);
		expect(
			caches.timings.get('color space convert composited image cache lookup')
		).toBeGreaterThanOrEqual(0);
		expect(caches.timings.get('color space convert composited image')).toBeGreaterThanOrEqual(0);
		expect(caches.timings.get('quantize direct dither+match loop')).toBeGreaterThanOrEqual(0);
	});
});
