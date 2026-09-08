import { describe, expect, it, vi } from 'vitest';
import {
	faithfulTypeScriptFallback,
	initializePackageProcessor,
	PackageInitializationFailure
} from './package-fallback';
import type { EnabledPaletteColor, ProcessingSettings } from './types';

const palette: EnabledPaletteColor[] = [
	{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'custom', enabled: true },
	{ name: 'White', key: '#FFFFFF', rgb: { r: 255, g: 255, b: 255 }, kind: 'custom', enabled: true }
];
const settings: ProcessingSettings = {
	output: {
		width: 2,
		height: 1,
		resize: 'nearest',
		alphaMode: 'preserve',
		alphaThreshold: 0,
		matteKey: '#FFFFFF',
		lockAspect: false,
		autoSizeOnUpload: false,
		scaleFactor: 1
	},
	dither: {
		algorithm: 'none',
		strength: 100,
		placement: 'everywhere',
		placementRadius: 1,
		placementThreshold: 0,
		placementSoftness: 0,
		serpentine: false,
		seed: 1,
		useColorSpace: false
	},
	colorSpace: 'srgb'
};
const source = {
	width: 2,
	height: 1,
	data: new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255])
};

describe('faithful TypeScript fallback admission', () => {
	it('classifies a failed package module load at its initialization boundary', async () => {
		vi.doMock('ditherette', () => {
			throw new TypeError('Module fetch failed');
		});
		try {
			await expect(initializePackageProcessor()).rejects.toBeInstanceOf(
				PackageInitializationFailure
			);
		} finally {
			vi.doUnmock('ditherette');
		}
	});
	it('admits exact palette pixels, duplicate palette entries, and thresholded invisible pixels', () => {
		expect(faithfulTypeScriptFallback(source, palette, settings)).toBe(true);
		expect(faithfulTypeScriptFallback(source, [palette[1], palette[0], palette[1]], settings)).toBe(
			true
		);
		const transparent = { ...source, data: new Uint8ClampedArray([0, 0, 0, 255, 37, 81, 219, 0]) };
		expect(faithfulTypeScriptFallback(transparent, palette, settings)).toBe(true);
	});
	it('admits arbitrary pixel values only when one distinct visible color determines every match', () => {
		const unmatched = { ...source, data: new Uint8ClampedArray([1, 2, 3, 255, 80, 90, 100, 255]) };
		expect(faithfulTypeScriptFallback(unmatched, [palette[1]], settings)).toBe(true);
		expect(faithfulTypeScriptFallback(unmatched, palette, settings)).toBe(false);
	});
	it('rejects the established 2-to-49 nearest center tie, but admits exact 2-to-4 sampling', () => {
		expect(
			faithfulTypeScriptFallback(source, palette, {
				...settings,
				output: { ...settings.output, width: 49 }
			})
		).toBe(false);
		expect(
			faithfulTypeScriptFallback(source, palette, {
				...settings,
				output: { ...settings.output, width: 4 }
			})
		).toBe(true);
	});
	it('keeps unsupported resampling, matching, dithering, and compositing visible', () => {
		for (const resize of ['area', 'bilinear'] as const)
			expect(
				faithfulTypeScriptFallback(source, palette, {
					...settings,
					output: { ...settings.output, resize }
				})
			).toBe(false);
		expect(faithfulTypeScriptFallback(source, palette, { ...settings, colorSpace: 'oklab' })).toBe(
			false
		);
		expect(
			faithfulTypeScriptFallback(source, palette, {
				...settings,
				dither: { ...settings.dither, algorithm: 'random' }
			})
		).toBe(false);
		expect(
			faithfulTypeScriptFallback(source, palette, {
				...settings,
				output: { ...settings.output, alphaMode: 'matte' }
			})
		).toBe(false);
	});
});
