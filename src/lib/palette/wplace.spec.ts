import { describe, expect, it } from 'vitest';
import {
	WPLACE_PALETTE,
	WPLACE_PALETTE_NAME,
	correctedWplaceKey,
	enabledPalette,
	enabledWplacePalette,
	paletteColorEnabled,
	paletteEnabledKey
} from './wplace';
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

describe('Wplace Teal correction', () => {
	it('serves the corrected Teal and keeps every other colour in order', () => {
		const teal = WPLACE_PALETTE.find((color) => color.name === 'Teal');

		expect(teal).toMatchObject({
			key: '#10AEA6',
			rgb: { r: 16, g: 174, b: 166 },
			kind: 'free'
		});
		expect(WPLACE_PALETTE.filter((color) => color.kind === 'free')).toHaveLength(31);
		expect(WPLACE_PALETTE.filter((color) => color.kind === 'premium')).toHaveLength(32);
		expect(WPLACE_PALETTE.at(-1)).toMatchObject({ kind: 'transparent' });
		expect(WPLACE_PALETTE).toHaveLength(64);
		expect(WPLACE_PALETTE.some((color) => color.key === '#10AE82')).toBe(false);
		expect(WPLACE_PALETTE.map((color) => color.key)).toEqual([
			'#000000',
			'#3C3C3C',
			'#787878',
			'#D2D2D2',
			'#FFFFFF',
			'#600018',
			'#ED1C24',
			'#FF7F27',
			'#F6AA09',
			'#F9DD3B',
			'#FFFABC',
			'#0EB968',
			'#13E67B',
			'#87FF5E',
			'#0C816E',
			'#10AEA6',
			'#13E1BE',
			'#60F7F2',
			'#28509E',
			'#4093E4',
			'#6B50F6',
			'#99B1FB',
			'#780C99',
			'#AA38B9',
			'#E09FF9',
			'#CB007A',
			'#EC1F80',
			'#F38DA9',
			'#684634',
			'#95682A',
			'#F8B277',
			'#AAAAAA',
			'#A50E1E',
			'#FA8072',
			'#E45C1A',
			'#9C8431',
			'#C5AD31',
			'#E8D45F',
			'#4A6B3A',
			'#5A944A',
			'#84C573',
			'#0F799F',
			'#BBFAF2',
			'#7DC7FF',
			'#4D31B8',
			'#4A4284',
			'#7A71C4',
			'#B5AEF1',
			'#9B5249',
			'#D18078',
			'#FAB6A4',
			'#DBA463',
			'#7B6352',
			'#9C846B',
			'#D6B594',
			'#D18051',
			'#FFC5A5',
			'#6D643F',
			'#948C6B',
			'#CDC59E',
			'#333941',
			'#6D758D',
			'#B3B9D1',
			'transparent'
		]);
	});

	it('reads Teal visibility saved under the previous key', () => {
		expect(paletteColorEnabled({ '#10AE82': false }, WPLACE_PALETTE_NAME, '#10AEA6')).toBe(false);
		expect(
			paletteColorEnabled({ '#10AE82': false, '#10AEA6': true }, WPLACE_PALETTE_NAME, '#10AEA6')
		).toBe(true);
		expect(paletteColorEnabled({}, WPLACE_PALETTE_NAME, '#10AEA6')).toBe(true);

		const enabled = enabledWplacePalette({ '#10AE82': false });
		expect(enabled.some((color) => color.name === 'Teal')).toBe(false);
		expect(enabled).toHaveLength(63);
	});

	it('keeps a custom palette holding the previous Teal hex independent', () => {
		const oldCopy: Palette = {
			name: 'Old copy',
			source: 'custom',
			colors: [{ name: 'Old teal', key: '#10AE82', rgb: { r: 16, g: 174, b: 130 }, kind: 'custom' }]
		};

		expect(paletteColorEnabled({ '#10AE82': false }, 'Old copy', '#10AE82')).toBe(true);
		expect(enabledPalette(oldCopy, { '#10AE82': false }).map((color) => color.key)).toEqual([
			'#10AE82'
		]);
		expect(
			paletteColorEnabled(
				{ [paletteEnabledKey('Old copy', '#10AE82')]: false },
				WPLACE_PALETTE_NAME,
				'#10AEA6'
			)
		).toBe(true);
	});

	it('maps the previous key to the corrected key only one way', () => {
		expect(correctedWplaceKey('#10AE82')).toBe('#10AEA6');
		expect(correctedWplaceKey('#10AEA6')).toBeUndefined();
	});
});
