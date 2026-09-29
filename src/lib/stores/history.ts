import { atom, computed, type WritableAtom } from 'nanostores';
import { scheduleProcessing } from '$lib/processing/client';
import type { ColorSpaceId, DitherSettings, OutputSettings, Palette } from '$lib/processing/types';
import {
	activePaletteName,
	colorSpace,
	customPalettes,
	ditherSettings,
	outputSettings,
	paletteEnabled,
	sourceMeta
} from './app';
import { effectLayers, type EffectLayer } from './effects';

/** The processing settings one history entry restores. Images and caches never enter history. */
type Settings = {
	output: OutputSettings;
	dither: DitherSettings;
	colorSpace: ColorSpaceId;
	palettes: Palette[];
	paletteName: string;
	paletteEnabled: Record<string, boolean>;
	effects: EffectLayer[];
};

const STORES: { [K in keyof Settings]: WritableAtom<Settings[K]> } = {
	output: outputSettings,
	dither: ditherSettings,
	colorSpace,
	palettes: customPalettes,
	paletteName: activePaletteName,
	paletteEnabled,
	effects: effectLayers
};
const KEYS = Object.keys(STORES) as (keyof Settings)[];

/** An edit commits once no pointer is pressed and settings stay still this long. */
export const SETTLE_MS = 300;
/** Undo steps kept. Entries share the stores' immutable values, so each costs a few references. */
export const MAX_HISTORY = 100;
/** What a pointer gesture touched: always one entry, however many controls it moved. */
const GESTURE = ['pointer'];

const stacks = atom<{ past: readonly Settings[]; future: readonly Settings[] }>({
	past: [],
	future: []
});
export const canUndo = computed(stacks, ({ past }) => past.length > 0);
export const canRedo = computed(stacks, ({ future }) => future.length > 0);

/** The settings of the latest entry. */
let present = snapshot();
/** The live settings as of the last burst of changes. */
let seen = present;
let source = sourceMeta.get();
/** The setting paths the uncommitted edit last touched, or undefined when nothing is pending. */
let pending: readonly string[] | undefined;
let queued = false;
let applying = false;
const pointers = new Set<number>();
let timer: ReturnType<typeof setTimeout> | undefined;

function snapshot(): Settings {
	return Object.fromEntries(KEYS.map((key) => [key, STORES[key].get()])) as Settings;
}

const same = (left: unknown, right: unknown) =>
	left === right || JSON.stringify(left) === JSON.stringify(right);

const isRecord = (value: unknown): value is Record<string, unknown> =>
	typeof value === 'object' && value !== null && !Array.isArray(value);

/** The paths that differ between two values, descending into plain objects. */
function paths(path: string, before: unknown, after: unknown): string[] {
	if (same(before, after)) return [];
	if (!isRecord(before) || !isRecord(after)) return [path];
	return [...new Set([...Object.keys(before), ...Object.keys(after)])].flatMap((key) =>
		paths(`${path}.${key}`, before[key], after[key])
	);
}

const byId = (layers: readonly EffectLayer[]) =>
	Object.fromEntries(layers.map((layer) => [layer.id, layer]));

/**
 * Names the setting paths an edit touched. Effect layers compare by id, so edits to two layers
 * touch separate paths, while adding, removing, or reordering layers touches the list itself.
 */
function touched(before: Settings, after: Settings) {
	return KEYS.flatMap((key) => {
		if (key !== 'effects') return paths(key, before[key], after[key]);
		const order = same(
			before.effects.map(({ id }) => id),
			after.effects.map(({ id }) => id)
		);
		return [...(order ? [] : [key]), ...paths(key, byId(before.effects), byId(after.effects))];
	});
}

function settleLater() {
	clearTimeout(timer);
	timer = pointers.size ? undefined : setTimeout(commit, SETTLE_MS);
}

function changed() {
	if (applying || queued) return;
	queued = true;
	queueMicrotask(settle);
}

/**
 * Take in one burst of store changes, so an action that sets several stores is one edit. A burst
 * that shares a path with the pending edit repeats its control and extends it, even when derived
 * fields such as rounded sizes change only on some ticks; any other burst commits it first. A new
 * source or crop resets history instead: a size or effect made for another frame would mislead.
 */
function settle() {
	if (!queued) return;
	queued = false;
	const next = snapshot();
	if (sourceMeta.get() !== source || !same(next.output.crop, seen.output.crop)) {
		reset(next);
		return;
	}
	const controls = touched(seen, next);
	if (!controls.length) return;
	const edit = pointers.size ? GESTURE : controls;
	if (pending && !pending.some((path) => edit.includes(path))) commit();
	pending = edit;
	seen = next;
	settleLater();
}

/** Record the pending edit, unless it ended where the latest entry already is. */
function commit() {
	clearTimeout(timer);
	timer = undefined;
	if (pending === undefined) return;
	pending = undefined;
	if (KEYS.every((key) => same(present[key], seen[key]))) return;
	const { past } = stacks.get();
	stacks.set({ past: [...past, present].slice(-MAX_HISTORY), future: [] });
	present = seen;
}

function flush() {
	settle();
	commit();
}

function reset(next = snapshot()) {
	clearTimeout(timer);
	timer = undefined;
	present = seen = next;
	source = sourceMeta.get();
	pending = undefined;
	stacks.set({ past: [], future: [] });
}

function restore<K extends keyof Settings>(key: K, value: Settings[K]) {
	if (STORES[key].get() !== value) STORES[key].set(value);
}

/** Set every store at once, then reprocess without the slider debounce: undo is a deliberate jump. */
function apply(target: Settings) {
	applying = true;
	try {
		for (const key of KEYS) restore(key, target[key]);
	} finally {
		applying = false;
	}
	present = seen = target;
	scheduleProcessing(0);
}

/** Restore the settings before the last edit. */
export function undo() {
	flush();
	const { past, future } = stacks.get();
	const previous = past.at(-1);
	if (!previous) return;
	stacks.set({ past: past.slice(0, -1), future: [present, ...future] });
	apply(previous);
}

/** Restore the settings the last undo left. */
export function redo() {
	flush();
	const { past, future } = stacks.get();
	const [next, ...rest] = future;
	if (!next) return;
	stacks.set({ past: [...past, present], future: rest });
	apply(next);
}

/**
 * Record settings edits until the returned stop runs. Pressed pointers hold the entry open until
 * the last one lifts, so a whole slider or curve drag becomes one undo step; the first press of a
 * gesture commits any earlier edit. Losing focus releases every pointer, since their ups go unseen.
 */
export function startSettingsHistory() {
	const press = (event: PointerEvent) => {
		if (!pointers.size) flush();
		pointers.add(event.pointerId);
	};
	// Changes still queued at a release happened during the gesture, so they settle into it first.
	const release = (event: PointerEvent) => {
		settle();
		pointers.delete(event.pointerId);
		if (!pointers.size && pending !== undefined) settleLater();
	};
	const releaseAll = () => {
		settle();
		pointers.clear();
		if (pending !== undefined) settleLater();
	};
	const unsubscribers = [...KEYS.map((key) => STORES[key]), sourceMeta].map((store) =>
		store.listen(changed)
	);
	// Listening mounts persistent stores, which can load saved values, so the baseline comes after.
	reset();
	window.addEventListener('pointerdown', press, true);
	window.addEventListener('pointerup', release, true);
	window.addEventListener('pointercancel', release, true);
	window.addEventListener('blur', releaseAll);
	return () => {
		for (const unsubscribe of unsubscribers) unsubscribe();
		window.removeEventListener('pointerdown', press, true);
		window.removeEventListener('pointerup', release, true);
		window.removeEventListener('pointercancel', release, true);
		window.removeEventListener('blur', releaseAll);
		clearTimeout(timer);
		queued = false;
		pending = undefined;
		pointers.clear();
	};
}
