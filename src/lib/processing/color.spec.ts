import { describe, expect, it } from 'vitest';
import { createPaletteMatcher } from './color';
import type { EnabledPaletteColor } from './types';

const palette: EnabledPaletteColor[] = [
	{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true },
	{ name: 'White', key: '#FFFFFF', rgb: { r: 255, g: 255, b: 255 }, kind: 'free', enabled: true },
	{ name: 'Transparent', key: 'transparent', kind: 'transparent', enabled: true }
];

describe('createPaletteMatcher', () => {
	it('never selects Transparent during nearest-color matching', () => {
		const match = createPaletteMatcher(palette, 'oklab').nearest({ r: 20, g: 20, b: 20 });

		expect(match.color.key).toBe('#000000');
	});

	it('preserves stable palette indices', () => {
		const match = createPaletteMatcher(palette, 'srgb').nearest({ r: 240, g: 240, b: 240 });

		expect(match.index).toBe(1);
	});
});
