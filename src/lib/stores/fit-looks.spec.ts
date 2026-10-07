import { beforeEach, describe, expect, it, vi } from 'vitest';
import { addEffect, effectLayers, removeEffect, updateEffect } from './effects';
import { looksApplied } from './fit-looks';
import './fit-looks';
import { useTestStorageEngine } from '@nanostores/persistent';

useTestStorageEngine();

beforeEach(() => {
	effectLayers.set([]);
});

const look = (id: string, next: 'natural' | 'fitted' | 'vivid') => {
	const layer = effectLayers.get().find((item) => item.id === id)!;
	if (layer.step.effect !== 'palette-fit') throw new Error('Expected a palette-fit layer.');
	updateEffect(id, { ...layer.step, look: next });
};

describe('looksApplied', () => {
	it("seeds the layer's look, appends switches, and drops removed layers", () => {
		const layer = addEffect('exposure');
		const fit = addEffect('palette-fit');
		expect(looksApplied.get().get(fit.id)).toEqual(['fitted']);
		expect(looksApplied.get().has(layer.id)).toBe(false);

		look(fit.id, 'vivid');
		look(fit.id, 'natural');
		expect(looksApplied.get().get(fit.id)).toEqual(['fitted', 'vivid', 'natural']);

		// Setting the same look again records nothing.
		look(fit.id, 'natural');
		expect(looksApplied.get().get(fit.id)).toEqual(['fitted', 'vivid', 'natural']);

		removeEffect(fit.id);
		expect(looksApplied.get().has(fit.id)).toBe(false);
	});
});

describe('looksApplied seeding', () => {
	it('seeds the look of a layer that existed before the module loaded', async () => {
		vi.resetModules();
		const effects = await import('./effects');
		const { EFFECTS } = await import('$lib/effects/catalog');
		effects.effectLayers.set([
			{ id: 'pre', name: 'Palette fit', step: EFFECTS['palette-fit'].create() }
		]);
		const store = await import('./fit-looks');
		expect(store.looksApplied.get().get('pre')).toEqual(['fitted']);
		const step = effects.effectLayers.get()[0]!.step;
		if (step.effect !== 'palette-fit') throw new Error('Expected a palette-fit layer.');
		effects.updateEffect('pre', { ...step, look: 'vivid' });
		expect(store.looksApplied.get().get('pre')).toEqual(['fitted', 'vivid']);
	});
});
