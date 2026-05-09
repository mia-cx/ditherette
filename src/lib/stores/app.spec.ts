import { beforeEach, describe, expect, it } from 'vitest';
import { paletteEnabledKey } from '$lib/palette/wplace';
import type { Palette } from '$lib/processing/types';
import {
	activePaletteName,
	customPalettes,
	deleteActivePaletteColors,
	deleteCustomPalette,
	importCustomPaletteData,
	paletteEnabled,
	previewCustomPaletteImport
} from './app';

const customPalette: Palette = {
	name: 'Custom',
	source: 'custom',
	colors: [
		{ name: 'Red', key: '#FF0000', rgb: { r: 255, g: 0, b: 0 }, kind: 'custom' },
		{ name: 'Blue', key: '#0000FF', rgb: { r: 0, g: 0, b: 255 }, kind: 'custom' },
		{ name: 'Transparent', key: 'transparent', kind: 'transparent' }
	]
};

function importRecord(name: string, colors = [{ name: 'Red', key: '#FF0000' }]) {
	return { name, colors };
}

describe('custom palette state persistence', () => {
	beforeEach(() => {
		customPalettes.set([]);
		activePaletteName.set('Wplace (Default)');
		paletteEnabled.set({});
	});

	it('rejects imported palette names after normalization', () => {
		expect(() => previewCustomPaletteImport(importRecord(' Wplace (Default) '))).toThrow(/Wplace/);
	});

	it('rejects duplicate imported palette names after normalization', () => {
		expect(() =>
			previewCustomPaletteImport([importRecord('Custom'), importRecord(' Custom ')])
		).toThrow(/duplicate palette name/i);
	});

	it('prunes deleted color enabled keys', () => {
		customPalettes.set([customPalette]);
		activePaletteName.set('Custom');
		paletteEnabled.set({
			[paletteEnabledKey('Custom', '#FF0000')]: false,
			[paletteEnabledKey('Custom', '#0000FF')]: false,
			[paletteEnabledKey('Custom', '#00FF00')]: false
		});

		deleteActivePaletteColors(['#0000FF']);

		expect(paletteEnabled.get()).toHaveProperty(paletteEnabledKey('Custom', '#FF0000'));
		expect(paletteEnabled.get()).not.toHaveProperty(paletteEnabledKey('Custom', '#0000FF'));
		expect(paletteEnabled.get()).not.toHaveProperty(paletteEnabledKey('Custom', '#00FF00'));
	});

	it('prunes deleted palette enabled keys', () => {
		customPalettes.set([customPalette]);
		paletteEnabled.set({ [paletteEnabledKey('Custom', '#FF0000')]: false });

		deleteCustomPalette('Custom');

		expect(paletteEnabled.get()).not.toHaveProperty(paletteEnabledKey('Custom', '#FF0000'));
	});

	it('prunes stale color keys when overwriting an imported palette', () => {
		customPalettes.set([customPalette]);
		paletteEnabled.set({
			[paletteEnabledKey('Custom', '#FF0000')]: false,
			[paletteEnabledKey('Custom', '#0000FF')]: false
		});

		importCustomPaletteData(importRecord('Custom', [{ name: 'Green', key: '#00FF00' }]));

		expect(paletteEnabled.get()).not.toHaveProperty(paletteEnabledKey('Custom', '#FF0000'));
		expect(paletteEnabled.get()).not.toHaveProperty(paletteEnabledKey('Custom', '#0000FF'));
		expect(paletteEnabled.get()).toHaveProperty(paletteEnabledKey('Custom', '#00FF00'));
	});
});
