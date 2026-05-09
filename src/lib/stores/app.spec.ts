import { beforeEach, describe, expect, it } from 'vitest';
import { paletteEnabledKey } from '$lib/palette/wplace';
import type { Palette, ProcessedImage } from '$lib/processing/types';
import { cancelProcessing, scheduleProcessing } from '$lib/processing/client';
import {
	activePaletteName,
	customPalettes,
	deleteActivePaletteColors,
	deleteCustomPalette,
	importCustomPaletteData,
	outputSettings,
	paletteEnabled,
	previewCustomPaletteImport,
	processedImage,
	sourceImageData,
	updateOutputSettings
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

function processedFixture(settingsHash = 'stale'): ProcessedImage {
	return {
		width: 1,
		height: 1,
		indices: new Uint8Array([0]),
		palette: [],
		transparentIndex: -1,
		warnings: [],
		settingsHash,
		updatedAt: 1
	};
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

describe('processed output lifecycle', () => {
	beforeEach(() => {
		cancelProcessing();
		processedImage.set(undefined);
		sourceImageData.set(undefined);
		outputSettings.set({
			width: 512,
			height: 512,
			lockAspect: true,
			resize: 'bilinear',
			alphaMode: 'preserve',
			alphaThreshold: 0,
			matteKey: '#FFFFFF',
			autoSizeOnUpload: false,
			scaleFactor: 1
		});
	});

	it('keeps the current output visible while non-crop settings are reprocessed', () => {
		const current = processedFixture();
		processedImage.set(current);
		sourceImageData.set({ width: 1, height: 1, data: new Uint8ClampedArray(4) } as ImageData);

		scheduleProcessing(1_000_000);

		expect(processedImage.get()).toBe(current);
		cancelProcessing();
	});

	it('flushes the current output when crop changes', () => {
		processedImage.set(processedFixture());

		updateOutputSettings({ crop: { x: 0, y: 0, width: 256, height: 256 } });

		expect(processedImage.get()).toBeUndefined();
	});
});
