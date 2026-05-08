import { describe, expect, it } from 'vitest';
import { enabledPalette, paletteColorEnabled, paletteEnabledKey } from './wplace';
import type { Palette } from '$lib/processing/types';

const customPalette: Palette = {
	name: 'Custom',
	source: 'custom',
	colors: [
		{ name: 'Red', key: '#FF0000', rgb: { r: 255, g: 0, b: 0 }, kind: 'custom' },
		{ name: 'Blue', key: '#0000FF', rgb: { r: 0, g: 0, b: 255 }, kind: 'custom' }
	]
};

describe('palette enabled state', () => {
	it('reads legacy custom-palette keys for existing persisted state', () => {
		const enabled = { ['Custom\u0000#FF0000']: false };

		expect(paletteColorEnabled(enabled, 'Custom', '#FF0000')).toBe(false);
		expect(enabledPalette(customPalette, enabled).map((color) => color.key)).toEqual(['#0000FF']);
	});

	it('lets the collision-safe key override legacy persisted state', () => {
		const enabled = {
			['Custom\u0000#FF0000']: false,
			[paletteEnabledKey('Custom', '#FF0000')]: true
		};

		expect(paletteColorEnabled(enabled, 'Custom', '#FF0000')).toBe(true);
	});
});
