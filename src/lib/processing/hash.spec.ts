import { describe, expect, it } from 'vitest';
import { processingIdentityHash, settingsHash } from './hash';
import type { DitherSettings, EnabledPaletteColor, OutputSettings } from './types';

const output: OutputSettings = {
	width: 10,
	height: 10,
	lockAspect: true,
	fit: 'contain',
	resize: 'nearest',
	alphaMode: 'preserve',
	alphaThreshold: 0,
	matteKey: '#FFFFFF',
	autoSizeOnUpload: false,
	scaleFactor: 1
};

const dither: DitherSettings = {
	algorithm: 'none',
	strength: 100,
	placement: 'everywhere',
	placementRadius: 3,
	placementThreshold: 12,
	placementSoftness: 8,
	serpentine: true,
	seed: 1,
	useColorSpace: false
};

const palette: EnabledPaletteColor[] = [
	{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'custom', enabled: true }
];

describe('settingsHash', () => {
	it('serializes object keys canonically', () => {
		expect(settingsHash({ b: 2, a: 1 })).toBe(settingsHash({ a: 1, b: 2 }));
	});

	it('throws on values that cannot be represented as JSON settings', () => {
		expect(() => settingsHash({ value: Number.NaN })).toThrow(/hashed/i);
	});
});

describe('processingIdentityHash', () => {
	it('hashes effective palette colors without unrelated persisted enabled state', () => {
		const base = processingIdentityHash({
			output,
			dither,
			colorSpace: 'oklab',
			paletteName: 'Custom',
			paletteSource: 'custom',
			palette,
			source: { name: 'source.png', width: 10, height: 10, type: 'image/png', updatedAt: 1 }
		});
		const withUnrelatedState = processingIdentityHash({
			output: { ...output },
			dither: { ...dither },
			colorSpace: 'oklab',
			paletteName: 'Custom',
			paletteSource: 'custom',
			palette: [...palette],
			source: { name: 'source.png', width: 10, height: 10, type: 'image/png', updatedAt: 1 }
		});

		expect(withUnrelatedState).toBe(base);
	});

	it('changes when an effective output input changes', () => {
		const base = processingIdentityHash({
			output,
			dither,
			colorSpace: 'oklab',
			paletteName: 'Custom',
			paletteSource: 'custom',
			palette,
			source: undefined
		});
		const changed = processingIdentityHash({
			output: { ...output, width: output.width + 1 },
			dither,
			colorSpace: 'oklab',
			paletteName: 'Custom',
			paletteSource: 'custom',
			palette,
			source: undefined
		});

		expect(changed).not.toBe(base);
	});
});
