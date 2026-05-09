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
} {
	const paletteVectors = new Map<string, PaletteVectorSpace>();
	const images = new Map<string, ColorVectorImage>();
	return {
		paletteVectors,
		images,
		colorVectorImageScope,
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
});
