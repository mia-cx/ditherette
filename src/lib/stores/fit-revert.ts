import { atom } from 'nanostores';
import { paletteColorEnabled, paletteEnabledKey } from '$lib/palette/wplace';
import {
	activePalette,
	activePaletteName,
	outputSettings,
	paletteEnabled,
	sourceImageData,
	updateOutputSettings
} from './app';
import type { PaletteFitEffect } from 'ditherette';
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
const states = new Map<string, FitState<EffectLayer> & { readonly signature: string }>();

/** What a fit itself contributes to its offer: look, space, and the curve list. */
const signature = (step: PaletteFitEffect) => JSON.stringify([step.look, step.space, step.curves]);

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
		values: Object.fromEntries(palette.colors.map((color) => [color.key, color.rgb ?? null])),
		crop: outputSettings.get().crop,
		before: layers.slice(0, at)
	};
}

function offers(next: (map: Map<string, FitLayerRevert>) => void) {
	const map = new Map(fitReverts.get());
	next(map);
	fitReverts.set(map);
}

/** Revert writes several stores; their mid-restore states must not re-analyse the fit. */
let restoring = false;

/**
 * Re-check every palette fit after any input edit, decided against one snapshot of the layers.
 * A locked fit whose inputs changed re-analyses; a revertable change offers Revert, while an
 * unlocked fit with a pending offer drops it once its inputs move again. The fit's own look,
 * space, and curves are part of its state: editing, Resetting, or re-spacing the fit drops its
 * offer, while strength, mask, enabled, and renames do not.
 */
function check() {
	if (restoring) return;
	const layers = effectLayers.get();
	const live = new Set(layers.map((layer) => layer.id));
	const changed: EffectLayer[] = [];
	const drops: string[] = [];
	layers.forEach((layer, at) => {
		if (layer.step.effect !== 'palette-fit') return;
		const sig = signature(layer.step);
		const state = states.get(layer.id);
		const inputs = currentInputs(layers, at);
		if (layer.step.curves === null) {
			// Track whatever it analyses under now; a pending offer dies on the next change to
			// the fit or its inputs.
			states.set(layer.id, { inputs, signature: sig });
			if (
				state &&
				(state.signature !== sig || fitFingerprint(state.inputs) !== fitFingerprint(inputs))
			)
				drops.push(layer.id);
			return;
		}
		if (!state) {
			states.set(layer.id, { inputs, signature: sig });
			return;
		}
		if (state.signature !== sig) {
			// The fit itself changed: re-edit, Reset, or a space/look switch. A stale offer
			// would restore a fit that no longer exists, so it drops.
			states.set(layer.id, { inputs, signature: sig });
			drops.push(layer.id);
			return;
		}
		const decision = decideLockedFit(layer.step.curves, state, inputs);
		states.set(layer.id, {
			...decision.state,
			signature: decision.action === 'reanalyse' ? signature({ ...layer.step, curves: null }) : sig
		});
		if (decision.action === 'reanalyse') {
			changed.push({ ...layer, step: { ...layer.step, curves: null } });
			const revert = decision.state.revert;
			if (revert) offers((map) => map.set(layer.id, revert));
			else drops.push(layer.id);
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
		// Publishing the unlocks re-enters this check through the layers listener; keep it
		// from deciding twice, then re-baseline the unlocked fits against the new layers.
		restoring = true;
		try {
			effectLayers.set(layers.map((layer) => byId.get(layer.id) ?? layer));
		} finally {
			restoring = false;
		}
		const after = effectLayers.get();
		after.forEach((layer, at) => {
			if (layer.step.effect !== 'palette-fit') return;
			const state = states.get(layer.id);
			if (state) states.set(layer.id, { ...state, inputs: currentInputs(after, at) });
		});
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
 * curves. The snapshot is then consumed; Revert has only one level.
 */
export function revertFit(layerId: string) {
	const revert = fitReverts.get().get(layerId);
	if (!revert) return;
	const layers = effectLayers.get();
	const at = layers.findIndex((layer) => layer.id === layerId);
	if (at < 0) return;
	offers((map) => map.delete(layerId));
	// Track the restored step: the layer gets its edited curves and its own inputs back.
	const step = layers[at]!.step;
	if (step.effect !== 'palette-fit') return;
	states.set(layerId, {
		inputs: revert.inputs,
		signature: signature({ ...step, curves: revert.curves })
	});
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
	// The first source of a session restores saved edits; only replacing it clears them.
	if (source && next && next !== source) reset();
	if (next) source = next;
});
[effectLayers, activePalette, paletteEnabled, outputSettings].forEach((store) =>
	store.listen(check)
);
