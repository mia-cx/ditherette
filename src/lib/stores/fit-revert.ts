import { atom } from 'nanostores';
import type { Curve } from 'ditherette';
import { paletteColorEnabled, paletteEnabledKey } from '$lib/palette/wplace';
import {
	activePalette,
	activePaletteName,
	outputSettings,
	paletteEnabled,
	sourceImageData,
	updateOutputSettings
} from './app';
import { effectLayers, type EffectLayer } from './effects';
import {
	decideLockedFit,
	fitFingerprint,
	type FitInputs,
	type FitRevert,
	type FitState
} from '$lib/effects/palette-fit';

/** The inputs a fit was edited under, plus the snapshot Revert restores. */
export type FitLayerInputs = FitInputs<EffectLayer>;
export type FitLayerRevert = FitRevert<EffectLayer>;

/** Revert offers by layer id: shown as "Re-analysed for the new inputs". Session-only. */
export const fitReverts = atom<ReadonlyMap<string, FitLayerRevert>>(new Map());

/** The inputs each fit last analysed or locked under, while its snapshot is tracked. */
const states = new Map<string, FitState<EffectLayer>>();

function currentInputs(layers: readonly EffectLayer[], at: number): FitLayerInputs {
	const palette = activePalette.get();
	const enabled = paletteEnabled.get();
	return {
		paletteName: palette.name,
		enabled: Object.fromEntries(
			palette.colors.map((color) => [
				color.key,
				paletteColorEnabled(enabled, palette.name, color.key)
			])
		),
		crop: outputSettings.get().crop,
		before: layers.slice(0, at).filter((layer) => layer.step.enabled)
	};
}

function offers(next: (map: Map<string, FitLayerRevert>) => void) {
	const map = new Map(fitReverts.get());
	next(map);
	fitReverts.set(map);
}

/**
 * Re-check every palette fit after any input edit. A locked fit whose inputs changed re-analyses
 * and offers Revert; an unlocked fit with a pending offer drops it once its inputs move again.
 */
/** Revert writes several stores; their mid-restore states must not re-analyse the fit. */
let restoring = false;

function check() {
	if (restoring) return;
	const layers = effectLayers.get();
	const live = new Set(layers.map((layer) => layer.id));
	const changed: EffectLayer[] = [];
	const drops: string[] = [];
	layers.forEach((layer, at) => {
		if (layer.step.effect !== 'palette-fit') return;
		const state = states.get(layer.id);
		const inputs = currentInputs(layers, at);
		const same = state && fitFingerprint(state.inputs) === fitFingerprint(inputs);
		if (layer.step.curves === null) {
			// Track whatever it analyses under now; a pending offer dies on the next change.
			states.set(layer.id, { inputs });
			if (!same && fitReverts.get().has(layer.id)) drops.push(layer.id);
			return;
		}
		if (!state) {
			states.set(layer.id, { inputs });
			return;
		}
		const decision = decideLockedFit(layer.step.curves, state, inputs);
		states.set(layer.id, decision.state);
		if (decision.action === 'reanalyse' && decision.state.revert) {
			const revert = decision.state.revert;
			changed.push({ ...layer, step: { ...layer.step, curves: null } });
			offers((map) => map.set(layer.id, revert));
		}
	});
	for (const id of drops) offers((map) => map.delete(id));
	for (const id of [...states.keys(), ...fitReverts.get().keys()])
		if (!live.has(id)) {
			states.delete(id);
			offers((map) => map.delete(id));
		}
	if (changed.length) {
		const byId = new Map(changed.map((layer) => [layer.id, layer]));
		effectLayers.set(layers.map((layer) => byId.get(layer.id) ?? layer));
	}
}

/** A new source gives fits nothing to revert to: their edits clear with no offer. */
function reset() {
	states.clear();
	fitReverts.set(new Map());
	const layers = effectLayers.get();
	if (layers.some((layer) => layer.step.effect === 'palette-fit' && layer.step.curves !== null))
		effectLayers.set(
			layers.map((layer) =>
				layer.step.effect === 'palette-fit' && layer.step.curves !== null
					? { ...layer, step: { ...layer.step, curves: null } }
					: layer
			)
		);
}

/**
 * Undo one re-analysis: restore the palette selection, crop, earlier layers, and the edited
 * curves. The snapshot is then consumed — Revert has only one level.
 */
export function revertFit(layerId: string) {
	const revert = fitReverts.get().get(layerId);
	if (!revert) return;
	const layers = effectLayers.get();
	const at = layers.findIndex((layer) => layer.id === layerId);
	if (at < 0) return;
	offers((map) => map.delete(layerId));
	states.set(layerId, { inputs: revert.inputs });
	restoring = true;
	try {
		// Restore the palette selection: active palette plus each colour's enabled flag.
		activePaletteName.set(revert.inputs.paletteName);
		paletteEnabled.set({
			...paletteEnabled.get(),
			...Object.fromEntries(
				Object.entries(revert.inputs.enabled).map(([key, enabled]) => [
					paletteEnabledKey(revert.inputs.paletteName, key),
					enabled
				])
			)
		});
		updateOutputSettings({ crop: revert.inputs.crop });
		const fitLayer = layers[at]!;
		if (fitLayer.step.effect !== 'palette-fit') return;
		effectLayers.set([
			...revert.inputs.before,
			{
				...fitLayer,
				step: { ...fitLayer.step, curves: [...revert.curves] }
			},
			...layers.slice(at + 1)
		]);
	} finally {
		restoring = false;
	}
}

let source: ImageData | undefined;
sourceImageData.listen((next) => {
	if (next !== source) {
		source = next;
		if (next) reset();
	}
});
[effectLayers, activePalette, paletteEnabled, outputSettings].forEach((store) =>
	store.listen(check)
);
