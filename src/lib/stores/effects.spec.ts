import { beforeEach, describe, expect, it } from 'vitest';
import { isEffect } from 'ditherette';
import { EFFECTS, FLAT, STRAIGHT, packageSteps, type CurvePoints } from '$lib/effects/catalog';
import {
	activeEffectSteps,
	addEffect,
	effectLayers,
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

	it('runs matching RGB curves as one rgb step and differing ones per channel', () => {
		const curves = addEffect('curves');
		if (curves.step.effect !== 'curves' || curves.step.model !== 'srgb')
			throw new Error('Expected RGB curves.');
		const step = curves.step;
		updateEffect(curves.id, { ...step, curves: [LIFT, LIFT, LIFT] });
		expect(activeEffectSteps.get()).toEqual([
			{ effect: 'curves', enabled: true, channel: 'rgb', points: LIFT }
		]);

		updateEffect(curves.id, { ...step, curves: [LIFT, STRAIGHT, STRAIGHT] });
		expect(activeEffectSteps.get()).toEqual([
			{ effect: 'curves', enabled: true, channel: 'red', points: LIFT }
		]);
	});

	it('runs other colour models as one model-curves step and arbitrary XY as one channel-curve', () => {
		expect(
			packageSteps({
				effect: 'curves',
				enabled: true,
				model: 'oklch',
				curves: [LIFT, STRAIGHT, STRAIGHT]
			})
		).toEqual([
			{ effect: 'model-curves', enabled: true, model: 'oklch', curves: [LIFT, STRAIGHT, STRAIGHT] }
		]);
		const x = { model: 'hsl', channel: 'hue' } as const;
		const y = { model: 'hsl', channel: 'saturation' } as const;
		expect(
			packageSteps({ effect: 'curves', enabled: true, model: 'xy', x, y, points: FLAT })
		).toEqual([{ effect: 'channel-curve', enabled: true, x, y, points: FLAT }]);
		expect(
			[
				...packageSteps({
					effect: 'curves',
					enabled: true,
					model: 'hsv',
					curves: [LIFT, LIFT, LIFT]
				})
			].every(isEffect)
		).toBe(true);
	});

	it('never changes a layer into another effect', () => {
		const levels = addEffect('levels');
		updateEffect(levels.id, { effect: 'exposure', enabled: true, stops: 1 });
		expect(effectLayers.get()[0]!.step.effect).toBe('levels');
	});
});
