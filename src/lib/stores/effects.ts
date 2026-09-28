import { persistentAtom } from '@nanostores/persistent';
import { computed } from 'nanostores';
import { isEffect } from 'ditherette';
import {
	EFFECTS,
	MAX_EFFECT_LAYERS,
	packageSteps,
	stepCost,
	type EffectKind,
	type LayerStep
} from '$lib/effects/catalog';

/** One named instance in the effect pipeline. The name only labels it; `step` is what runs. */
export type EffectLayer = {
	readonly id: string;
	readonly name: string;
	readonly step: LayerStep;
};

export const MAX_EFFECT_NAME_LENGTH = 64;

/**
 * A saved step the package accepts. A curves layer counts when every package step it turns into
 * does, so the website keeps exactly what processing would accept.
 */
function isLayerStep(value: unknown): value is LayerStep {
	if (!value || typeof value !== 'object') return false;
	const { effect } = value as { effect?: unknown };
	// Colour-model and channel-to-channel curves only live inside a curves layer.
	if (effect === 'model-curves' || effect === 'channel-curve') return false;
	if (effect !== 'curves') return isEffect(value);
	try {
		return packageSteps(value as LayerStep).every(isEffect);
	} catch {
		// Storage holds untyped JSON; a damaged curves layer is dropped like any invalid step.
		return false;
	}
}

/**
 * Curves layers saved before colour models kept `curves: { red, green, blue }`; read them as RGB
 * curves. Anything else passes through for validation.
 */
function migrateStep(step: unknown): unknown {
	if (!step || typeof step !== 'object') return step;
	const { effect, model, curves } = step as { effect?: unknown; model?: unknown; curves?: unknown };
	if (effect !== 'curves' || model !== undefined || !curves || typeof curves !== 'object')
		return step;
	const { red, green, blue } = curves as Record<string, unknown>;
	return { ...step, model: 'srgb', curves: [red, green, blue] };
}

function isLayer(value: unknown): value is EffectLayer {
	if (!value || typeof value !== 'object') return false;
	const { id, name, step } = value as Record<string, unknown>;
	return typeof id === 'string' && typeof name === 'string' && isLayerStep(step);
}

/** Saved layers with their steps brought up to date, before validation. */
function migrateLayer(value: unknown): unknown {
	if (!value || typeof value !== 'object') return value;
	return { ...value, step: migrateStep((value as { step?: unknown }).step) };
}

/** Keep saved layers the package would accept, so a damaged entry cannot block processing. */
/** Keep the leading layers that fit the package's step limit. */
function withinStepLimit(layers: EffectLayer[]) {
	let left = MAX_EFFECT_LAYERS;
	return layers.filter((layer) => (left -= stepCost(layer.step.effect)) >= 0);
}

function decodeLayers(encoded: string): EffectLayer[] {
	try {
		const value: unknown = JSON.parse(encoded);
		return Array.isArray(value) ? withinStepLimit(value.map(migrateLayer).filter(isLayer)) : [];
	} catch {
		return [];
	}
}

export const effectLayers = persistentAtom<EffectLayer[]>('ditherette:effects', [], {
	encode: JSON.stringify,
	decode: decodeLayers
});

/** Package steps still free under the limit, counting each layer at its most. */
export const effectStepsLeft = computed(
	effectLayers,
	(layers) =>
		MAX_EFFECT_LAYERS - layers.reduce((total, layer) => total + stepCost(layer.step.effect), 0)
);

/** The steps processing runs: enabled layers, in pipeline order. */
export const activeEffectSteps = computed(effectLayers, (layers) =>
	layers.filter((layer) => layer.step.enabled).flatMap((layer) => packageSteps(layer.step))
);

function uniqueName(base: string, layers: readonly EffectLayer[], except?: string) {
	const taken = new Set(layers.filter((layer) => layer.id !== except).map((layer) => layer.name));
	if (!taken.has(base)) return base;
	let suffix = 2;
	while (taken.has(`${base} ${suffix}`)) suffix++;
	return `${base} ${suffix}`;
}

function replaceLayer(id: string, change: (layer: EffectLayer) => EffectLayer) {
	effectLayers.set(effectLayers.get().map((layer) => (layer.id === id ? change(layer) : layer)));
}

/**
 * A random 128-bit layer id. `crypto.randomUUID` exists only in secure contexts, so a dev server
 * opened over plain HTTP from another machine could not add effects with it.
 */
function newLayerId() {
	return Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) =>
		byte.toString(16).padStart(2, '0')
	).join('');
}

/** Append a neutral instance of `kind`, named after the effect, and return it. */
export function addEffect(kind: EffectKind): EffectLayer {
	const layers = effectLayers.get();
	if (stepCost(kind) > effectStepsLeft.get())
		throw new Error(`The pipeline holds at most ${MAX_EFFECT_LAYERS} effect steps.`);
	const layer: EffectLayer = {
		id: newLayerId(),
		name: uniqueName(EFFECTS[kind].label, layers),
		step: EFFECTS[kind].create()
	};
	effectLayers.set([...layers, layer]);
	return layer;
}

export function removeEffect(id: string) {
	effectLayers.set(effectLayers.get().filter((layer) => layer.id !== id));
}

/** Move a layer to `index` in the pipeline, clamped to its ends. */
export function moveEffect(id: string, index: number) {
	const layers = effectLayers.get();
	const from = layers.findIndex((layer) => layer.id === id);
	if (from < 0) return;
	const to = Math.max(0, Math.min(layers.length - 1, index));
	if (from === to) return;
	const next = layers.slice();
	const [layer] = next.splice(from, 1);
	next.splice(to, 0, layer!);
	effectLayers.set(next);
}

/** Rename a layer. Blank names fall back to the effect's label; duplicates get a number. */
export function renameEffect(id: string, name: string) {
	const layers = effectLayers.get();
	const layer = layers.find((candidate) => candidate.id === id);
	if (!layer) return;
	const base = name.trim().slice(0, MAX_EFFECT_NAME_LENGTH) || EFFECTS[layer.step.effect].label;
	const next = uniqueName(base, layers, id);
	if (next !== layer.name) replaceLayer(id, (current) => ({ ...current, name: next }));
}

/** Replace a layer's step arguments. The effect kind never changes. */
export function updateEffect(id: string, step: LayerStep) {
	replaceLayer(id, (layer) => (layer.step.effect === step.effect ? { ...layer, step } : layer));
}

export function setEffectEnabled(id: string, enabled: boolean) {
	replaceLayer(id, (layer) => ({ ...layer, step: { ...layer.step, enabled } }));
}
