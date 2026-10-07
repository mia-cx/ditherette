import { atom } from 'nanostores';
import type { FitLook } from 'ditherette';
import { effectLayers } from './effects';

/**
 * The looks each palette-fit layer has been applied with, in first-applied order. Seeded with
 * the step's look when the layer appears, appended when the look changes. Session-only.
 */
export const looksApplied = atom<ReadonlyMap<string, readonly FitLook[]>>(new Map());

effectLayers.subscribe((layers) => {
	const live = new Set(layers.map((layer) => layer.id));
	const next = new Map(looksApplied.get());
	let changed = false;
	for (const layer of layers) {
		if (layer.step.effect !== 'palette-fit') continue;
		const applied = next.get(layer.id);
		if (!applied) {
			next.set(layer.id, [layer.step.look]);
			changed = true;
		} else if (applied.at(-1) !== layer.step.look) {
			next.set(layer.id, [...applied, layer.step.look]);
			changed = true;
		}
	}
	for (const id of next.keys()) {
		if (!live.has(id)) {
			next.delete(id);
			changed = true;
		}
	}
	if (changed) looksApplied.set(next);
});
