import { describe, expect, it } from 'vitest';
import { createPaletteMatcher } from './color';
import type { ColorSpaceId, EnabledPaletteColor } from './types';

const palette: EnabledPaletteColor[] = [
	{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true },
	{ name: 'Red', key: '#FF0000', rgb: { r: 255, g: 0, b: 0 }, kind: 'free', enabled: true },
	{ name: 'White', key: '#FFFFFF', rgb: { r: 255, g: 255, b: 255 }, kind: 'free', enabled: true },
	{ name: 'Transparent', key: 'transparent', kind: 'transparent', enabled: true }
];

const colorSpaces: ColorSpaceId[] = [
	'oklab',
	'srgb',
	'linear-rgb',
	'weighted-rgb',
	'weighted-rgb-601',
	'weighted-rgb-709',
	'cielab',
	'oklch'
];

const samples = [
	[0, 0, 0],
	[255, 255, 255],
	[240, 20, 10],
	[20, 40, 180],
	[120, 120, 120]
] as const;

describe('createPaletteMatcher', () => {
	it('never selects Transparent during nearest-color matching', () => {
		const match = createPaletteMatcher(palette, 'oklab').nearest({ r: 20, g: 20, b: 20 });

		expect(match.color.key).toBe('#000000');
	});

	it('preserves stable palette indices', () => {
		const match = createPaletteMatcher(palette, 'srgb').nearest({ r: 240, g: 240, b: 240 });

		expect(match.index).toBe(2);
	});

	it.each(colorSpaces)('returns the same index through no-allocation matching in %s', (space) => {
		const matcher = createPaletteMatcher(palette, space);

		for (const [r, g, b] of samples) {
			expect(matcher.nearestIndexRgb(r, g, b)).toBe(matcher.nearestRgb(r, g, b).index);
		}
	});

	it('memoizes repeated byte RGB matches without changing the result', () => {
		const matcher = createPaletteMatcher(palette, 'oklab');
		const first = matcher.nearestIndexRgb(120, 120, 120);
		const afterFirst = matcher.memoStats();
		const second = matcher.nearestIndexRgb(120, 120, 120);
		const afterSecond = matcher.memoStats();

		expect(second).toBe(first);
		expect(afterFirst.rgbMisses).toBe(1);
		expect(afterFirst.rgbSets).toBe(1);
		expect(afterSecond.rgbHits).toBe(1);
	});

	it('preserves first-visible-color tie breaking', () => {
		const tiePalette: EnabledPaletteColor[] = [
			{ name: 'A', key: 'a', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true },
			{ name: 'B', key: 'b', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true }
		];

		expect(createPaletteMatcher(tiePalette, 'srgb').nearestIndexRgb(0, 0, 0)).toBe(0);
	});

	it('throws through every matching API when no visible colors are enabled', () => {
		const matcher = createPaletteMatcher([palette[3]!], 'srgb');

		expect(() => matcher.nearest({ r: 0, g: 0, b: 0 })).toThrow(/visible palette/i);
		expect(() => matcher.nearestRgb(0, 0, 0)).toThrow(/visible palette/i);
		expect(() => matcher.nearestIndexRgb(0, 0, 0)).toThrow(/visible palette/i);
		expect(() => matcher.paletteRgbAt(0)).toThrow(/visible palette/i);
	});
});
