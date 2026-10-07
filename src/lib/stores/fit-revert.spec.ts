import { useTestStorageEngine } from '@nanostores/persistent';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Curve } from 'ditherette';
import { WPLACE_PALETTE_NAME } from '$lib/palette/wplace';
import {
	activePalette,
	activePaletteName,
	customPalettes,
	outputSettings,
	paletteEnabled,
	setPaletteColorEnabled,
	sourceImageData,
	updateOutputSettings
} from './app';
import type { Palette } from '$lib/processing/types';
import { addEffect, effectLayers, setEffectEnabled, updateEffect } from './effects';
import { fitReverts, revertFit } from './fit-revert';

const image = () => ({ width: 1, height: 1, data: new Uint8ClampedArray(4) }) as ImageData;

const edited: Curve[] = [
	{
		kind: 'remap',
		x: { model: 'oklch', channel: 'lightness' },
		y: { model: 'oklch', channel: 'lightness' },
		points: [
			[0, 0],
			[1, 0.5]
		]
	}
];

const fitLayer = () => {
	const layer = addEffect('palette-fit');
	const step = layer.step;
	if (step.effect !== 'palette-fit') throw new Error('Expected a palette-fit layer.');
	return { layer, step };
};

/** Lock the layer by editing its curves under the current inputs. */
function lock(layerId: string) {
	const layer = effectLayers.get().find((item) => item.id === layerId)!;
	if (layer.step.effect !== 'palette-fit') throw new Error('Expected a palette-fit layer.');
	updateEffect(layerId, { ...layer.step, curves: edited });
}

beforeEach(() => {
	effectLayers.set([]);
	sourceImageData.set(image());
	activePaletteName.set(WPLACE_PALETTE_NAME);
	customPalettes.set([]);
});

useTestStorageEngine();

describe('palette-fit input tracking', () => {
	it('re-analyses a locked fit when its inputs change, offering one-level Revert', () => {
		const { layer } = fitLayer();
		lock(layer.id);
		expect(fitReverts.get().has(layer.id)).toBe(false);

		const crop = { x: 1, y: 1, width: 4, height: 4 };
		updateOutputSettings({ crop });
		const step = effectLayers.get().find((item) => item.id === layer.id)!.step;
		expect(step.effect === 'palette-fit' && step.curves).toBe(null);
		const revert = fitReverts.get().get(layer.id);
		expect(revert?.curves).toEqual(edited);
		expect(revert?.inputs.crop).toBeUndefined();

		revertFit(layer.id);
		const restored = effectLayers.get().find((item) => item.id === layer.id)!.step;
		expect(restored.effect === 'palette-fit' && restored.curves).toEqual(edited);
		expect(outputSettings.get().crop).toBeUndefined();
		expect(fitReverts.get().has(layer.id)).toBe(false);
	});

	it('tracks palette selection changes, and Revert restores them', () => {
		const { layer } = fitLayer();
		lock(layer.id);
		const colour = activePalette.get().colors.find((color) => color.rgb)!.key;

		setPaletteColorEnabled(colour, false);
		const step = effectLayers.get().find((item) => item.id === layer.id)!.step;
		expect(step.effect === 'palette-fit' && step.curves).toBe(null);
		expect(fitReverts.get().has(layer.id)).toBe(true);

		revertFit(layer.id);
		// The colour is enabled again and the curves restored.
		expect(paletteEnabled.get()[colour]).toBe(true);
		const restored = effectLayers.get().find((item) => item.id === layer.id)!.step;
		expect(restored.effect === 'palette-fit' && restored.curves).toEqual(edited);
	});

	it('restores the earlier layers, including disabled ones', () => {
		addEffect('exposure');
		const middle = addEffect('levels');
		const { layer } = fitLayer();
		setEffectEnabled(middle.id, false);
		lock(layer.id);

		// Edit an earlier enabled step: the fit re-analyses.
		const earlier = effectLayers.get()[0]!;
		if (earlier.step.effect !== 'exposure') throw new Error('Expected an exposure layer.');
		updateEffect(earlier.id, { ...earlier.step, stops: 1 });
		expect(fitReverts.get().has(layer.id)).toBe(true);

		revertFit(layer.id);
		const steps = effectLayers.get().map((item) => item.step);
		expect(steps.map((step) => step.effect)).toEqual(['exposure', 'levels', 'palette-fit']);
		expect(steps[1]!.enabled).toBe(false);
		expect(steps[0]).toMatchObject({ effect: 'exposure', stops: 0 });
	});

	it('strength and mask edits never unlock, and a new source drops edits silently', () => {
		const { layer } = fitLayer();
		lock(layer.id);
		const layerStep = () => {
			const step = effectLayers.get().find((item) => item.id === layer.id)!.step;
			if (step.effect !== 'palette-fit') throw new Error('Expected a palette-fit layer.');
			return step;
		};
		updateEffect(layer.id, { ...layerStep(), strength: 0.5 });
		expect(layerStep().curves).toEqual(edited);
		expect(fitReverts.get().has(layer.id)).toBe(false);

		sourceImageData.set(image());
		expect(layerStep().curves).toBe(null);
		expect(fitReverts.get().has(layer.id)).toBe(false);
	});

	it("drops the offer once the unlocked fit's inputs change again", () => {
		const { layer } = fitLayer();
		lock(layer.id);
		updateOutputSettings({ crop: { x: 0, y: 0, width: 2, height: 2 } });
		expect(fitReverts.get().has(layer.id)).toBe(true);
		updateOutputSettings({ crop: { x: 0, y: 0, width: 3, height: 3 } });
		expect(fitReverts.get().has(layer.id)).toBe(false);
		const step = effectLayers.get().find((item) => item.id === layer.id)!.step;
		expect(step.effect === 'palette-fit' && step.curves).toBe(null);
	});
});

describe('palette-fit tracking across sources and palette edits', () => {
	it("keeps restored edits through the session's first source, clears them on a new one", async () => {
		vi.resetModules();
		const {
			effectLayers: layers,
			updateEffect: update,
			addEffect: add
		} = await import('./effects');
		const app = await import('./app');
		const { fitReverts: reverts } = await import('./fit-revert');

		layers.set([]);
		const layer = add('palette-fit');
		const step = layer.step;
		if (step.effect !== 'palette-fit') throw new Error('Expected a palette-fit layer.');
		update(layer.id, { ...step, curves: edited });

		// Restoring the persisted source keeps the saved edits.
		app.sourceImageData.set(image());
		const kept = layers.get().find((item) => item.id === layer.id)!.step;
		expect(kept.effect === 'palette-fit' && kept.curves).toEqual(edited);

		// Loading a different image clears them without offering Revert.
		app.sourceImageData.set(image());
		const cleared = layers.get().find((item) => item.id === layer.id)!.step;
		expect(cleared.effect === 'palette-fit' && cleared.curves).toBe(null);
		expect(reverts.get().has(layer.id)).toBe(false);
	});

	it('re-analyses a locked fit on a colour value edit, offering no Revert', () => {
		const palette: Palette = {
			name: 'Custom test',
			source: 'custom',
			colors: [{ name: 'Dark', key: '112233', rgb: { r: 0x11, g: 0x22, b: 0x33 }, kind: 'custom' }]
		};
		customPalettes.set([palette]);
		activePaletteName.set('Custom test');
		const { layer } = fitLayer();
		lock(layer.id);

		customPalettes.set([
			{
				...palette,
				colors: [{ ...palette.colors[0]!, rgb: { r: 0, g: 0, b: 0 } }]
			}
		]);
		const step = effectLayers.get().find((item) => item.id === layer.id)!.step;
		expect(step.effect === 'palette-fit' && step.curves).toBe(null);
		expect(fitReverts.get().has(layer.id)).toBe(false);

		// Switching to a different palette keeps the offer.
		lock(layer.id);
		activePaletteName.set(WPLACE_PALETTE_NAME);
		expect(fitReverts.get().has(layer.id)).toBe(true);
	});
});

it('re-analyses every edited fit on one input change and offers Revert on each', () => {
	const first = fitLayer().layer;
	const second = fitLayer().layer;
	lock(first.id);
	lock(second.id);

	updateOutputSettings({ crop: { x: 0, y: 0, width: 2, height: 2 } });
	for (const layer of [first, second]) {
		const step = effectLayers.get().find((item) => item.id === layer.id)!.step;
		expect(step.effect === 'palette-fit' && step.curves).toBe(null);
		expect(fitReverts.get().has(layer.id)).toBe(true);
	}

	revertFit(second.id);
	const restored = effectLayers.get().find((item) => item.id === second.id)!.step;
	expect(restored.effect === 'palette-fit' && restored.curves).toEqual(edited);
	expect(fitReverts.get().has(first.id)).toBe(true);
});
