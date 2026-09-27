import { persistentAtom } from '@nanostores/persistent';
import { computed } from 'nanostores';
import { isEffect, type Effect } from 'ditherette';
import { EFFECTS, MAX_EFFECT_LAYERS, type EffectKind } from '$lib/effects/catalog';

/** One named instance in the effect pipeline. The name only labels it; `step` is what runs. */
export type EffectLayer = {
	readonly id: string;
	readonly name: string;
	readonly step: Effect;
};

export const MAX_EFFECT_NAME_LENGTH = 64;

function isLayer(value: unknown): value is EffectLayer {
	if (!value || typeof value !== 'object') return false;
	const { id, name, step } = value as Record<string, unknown>;
	return typeof id === 'string' && typeof name === 'string' && isEffect(step);
}

/** Keep saved layers the package would accept, so a damaged entry cannot block processing. */
function decodeLayers(encoded: string): EffectLayer[] {
	try {
		const value: unknown = JSON.parse(encoded);
		return Array.isArray(value) ? value.filter(isLayer).slice(0, MAX_EFFECT_LAYERS) : [];
	} catch {
		return [];
	}
}

export const effectLayers = persistentAtom<EffectLayer[]>('ditherette:effects', [], {
	encode: JSON.stringify,
	decode: decodeLayers
});

/** The steps processing runs: enabled layers, in pipeline order. */
export const activeEffectSteps = computed(effectLayers, (layers) =>
	layers.filter((layer) => layer.step.enabled).map((layer) => layer.step)
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

/** Append a neutral instance of `kind`, named after the effect, and return it. */
export function addEffect(kind: EffectKind): EffectLayer {
	const layers = effectLayers.get();
	if (layers.length >= MAX_EFFECT_LAYERS)
		throw new Error(`The pipeline holds at most ${MAX_EFFECT_LAYERS} effects.`);
	const layer: EffectLayer = {
		id: crypto.randomUUID(),
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
export function updateEffect(id: string, step: Effect) {
	replaceLayer(id, (layer) => (layer.step.effect === step.effect ? { ...layer, step } : layer));
}

export function setEffectEnabled(id: string, enabled: boolean) {
	replaceLayer(id, (layer) => ({ ...layer, step: { ...layer.step, enabled } }));
}
