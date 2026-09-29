import { setTestStorageKey, useTestStorageEngine } from '@nanostores/persistent';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CurvePoints } from 'ditherette';
import { EFFECTS, neutralCurve } from '$lib/effects/catalog';
import {
	activeEffectSteps,
	addEffect,
	effectLayers,
	effectStepsLeft,
	moveEffect,
	removeEffect,
	renameEffect,
	setEffectEnabled,
	updateEffect
} from './effects';

const LIFT: CurvePoints = [
	[0, 0.1],
	[1, 1]
];

beforeEach(() => effectLayers.set([]));

describe('effect layers', () => {
	it('names repeated instances apart and keeps their arguments separate', () => {
		const first = addEffect('levels');
		const second = addEffect('levels');
		expect([first.name, second.name]).toEqual(['Levels', 'Levels 2']);

		updateEffect(second.id, { ...EFFECTS.levels.create(), gamma: 2 });
		expect(activeEffectSteps.get().map((step) => 'gamma' in step && step.gamma)).toEqual([1, 2]);
	});

	it('renames, falling back to the label and numbering duplicates', () => {
		const levels = addEffect('levels');
		const curves = addEffect('curves');
		renameEffect(curves.id, '  Shadows  ');
		renameEffect(levels.id, 'Shadows');
		renameEffect(curves.id, '');
		expect(effectLayers.get().map((layer) => layer.name)).toEqual(['Shadows 2', 'Curves']);
	});

	it('runs only enabled layers, in pipeline order', () => {
		const exposure = addEffect('exposure');
		const levels = addEffect('levels');
		const curves = addEffect('curves');
		moveEffect(curves.id, 0);
		setEffectEnabled(levels.id, false);
		expect(activeEffectSteps.get().map((step) => step.effect)).toEqual(['curves', 'exposure']);

		removeEffect(exposure.id);
		expect(effectLayers.get().map((layer) => layer.id)).toEqual([curves.id, levels.id]);
	});

	it('makes ids without secure-context APIs', () => {
		const randomUUID = crypto.randomUUID;
		// Pages served over plain HTTP from another host have no randomUUID.
		Object.defineProperty(crypto, 'randomUUID', { value: undefined, configurable: true });
		try {
			const [first, second] = [addEffect('levels'), addEffect('levels')];
			expect(first.id).toMatch(/^[0-9a-f]{32}$/);
			expect(first.id).not.toBe(second.id);
		} finally {
			Object.defineProperty(crypto, 'randomUUID', { value: randomUUID, configurable: true });
		}
	});

	it('runs a curves layer as one package step with every curve in order', () => {
		const layer = addEffect('curves');
		if (layer.step.effect !== 'curves') throw new Error('Expected curves.');
		const lightness = { ...layer.step.curves[0]!, points: LIFT };
		const hue = neutralCurve(
			{ model: 'hsl', channel: 'hue' },
			{ model: 'hsl', channel: 'saturation' }
		);
		updateEffect(layer.id, { ...layer.step, curves: [lightness, hue] });
		expect(activeEffectSteps.get()).toEqual([
			{ effect: 'curves', enabled: true, curves: [lightness, hue] }
		]);
	});

	it('drops saved steps in the old curve shapes', async () => {
		const keep = { id: 'b', name: 'Exposure', step: EFFECTS.exposure.create() };
		useTestStorageEngine();
		setTestStorageKey(
			'ditherette:effects',
			JSON.stringify([
				{
					id: 'a',
					name: 'Curves',
					step: { effect: 'curves', enabled: true, channel: 'rgb', points: LIFT }
				},
				keep,
				{
					id: 'c',
					name: 'Model',
					step: {
						effect: 'model-curves',
						enabled: true,
						model: 'oklch',
						curves: [LIFT, LIFT, LIFT]
					}
				}
			])
		);
		vi.resetModules();
		const saved = await import('./effects');
		expect(saved.effectLayers.get()).toEqual([keep]);
	});

	it('counts every layer as one step so the chain never exceeds the package limit', () => {
		for (let count = 0; count < 63; count++) addEffect('exposure');
		expect(effectStepsLeft.get()).toBe(1);
		addEffect('curves');
		expect(effectStepsLeft.get()).toBe(0);
		expect(() => addEffect('exposure')).toThrow();
	});

	it('never changes a layer into another effect', () => {
		const levels = addEffect('levels');
		updateEffect(levels.id, { effect: 'exposure', enabled: true, stops: 1 });
		expect(effectLayers.get()[0]!.step.effect).toBe('levels');
	});
});
