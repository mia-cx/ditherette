import { atom, computed, type WritableAtom } from 'nanostores';
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

const stacks = atom<{ past: readonly Settings[]; future: readonly Settings[] }>({
	past: [],
	future: []
});
export const canUndo = computed(stacks, ({ past }) => past.length > 0);
export const canRedo = computed(stacks, ({ future }) => future.length > 0);

let present = snapshot();
let source = sourceMeta.get();
let pending = false;
let pressed = false;
let applying = false;
let timer: ReturnType<typeof setTimeout> | undefined;

function snapshot(): Settings {
	return Object.fromEntries(KEYS.map((key) => [key, STORES[key].get()])) as Settings;
}

const same = (left: unknown, right: unknown) =>
	left === right || JSON.stringify(left) === JSON.stringify(right);

function settleLater() {
	clearTimeout(timer);
	timer = pressed ? undefined : setTimeout(commit, SETTLE_MS);
}

function changed() {
	if (applying) return;
	pending = true;
	settleLater();
}

/**
 * Record the settings reached since the last entry. A new source or crop resets history instead:
 * restoring a size or effect made for another frame would show a misleading result.
 */
function commit() {
	clearTimeout(timer);
	timer = undefined;
	pending = false;
	const next = snapshot();
	if (sourceMeta.get() !== source || !same(next.output.crop, present.output.crop)) {
		reset(next);
		return;
	}
	if (KEYS.every((key) => same(present[key], next[key]))) return;
	const { past } = stacks.get();
	stacks.set({ past: [...past, present].slice(-MAX_HISTORY), future: [] });
	present = next;
}

function reset(next = snapshot()) {
	present = next;
	source = sourceMeta.get();
	stacks.set({ past: [], future: [] });
}

function restore<K extends keyof Settings>(key: K, value: Settings[K]) {
	if (STORES[key].get() !== value) STORES[key].set(value);
}

function apply(target: Settings) {
	applying = true;
	try {
		for (const key of KEYS) restore(key, target[key]);
	} finally {
		applying = false;
	}
	present = target;
}

/** Restore the settings before the last edit. The stores' subscribers reprocess as usual. */
export function undo() {
	if (pending) commit();
	const { past, future } = stacks.get();
	const previous = past.at(-1);
	if (!previous) return;
	stacks.set({ past: past.slice(0, -1), future: [present, ...future] });
	apply(previous);
}

/** Restore the settings the last undo left. */
export function redo() {
	if (pending) commit();
	const { past, future } = stacks.get();
	const [next, ...rest] = future;
	if (!next) return;
	stacks.set({ past: [...past, present], future: rest });
	apply(next);
}

/**
 * Record settings edits until the returned stop runs. A pointer press holds the entry open, so a
 * whole slider drag or curve drag becomes one undo step; a press also commits any earlier edit.
 */
export function startSettingsHistory() {
	reset();
	const press = () => {
		if (pending) commit();
		pressed = true;
	};
	const release = () => {
		pressed = false;
		if (pending) settleLater();
	};
	const unsubscribers = [...KEYS.map((key) => STORES[key]), sourceMeta].map((store) =>
		store.listen(changed)
	);
	window.addEventListener('pointerdown', press, true);
	window.addEventListener('pointerup', release, true);
	window.addEventListener('pointercancel', release, true);
	window.addEventListener('blur', release);
	return () => {
		for (const unsubscribe of unsubscribers) unsubscribe();
		window.removeEventListener('pointerdown', press, true);
		window.removeEventListener('pointerup', release, true);
		window.removeEventListener('pointercancel', release, true);
		window.removeEventListener('blur', release);
		clearTimeout(timer);
		pending = false;
		pressed = false;
	};
}
