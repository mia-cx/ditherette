import { beforeEach, describe, expect, it } from 'vitest';
import { EFFECTS } from '$lib/effects/catalog';
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

	it('never changes a layer into another effect', () => {
		const levels = addEffect('levels');
		updateEffect(levels.id, { effect: 'exposure', enabled: true, stops: 1 });
		expect(effectLayers.get()[0]!.step.effect).toBe('levels');
	});
});
