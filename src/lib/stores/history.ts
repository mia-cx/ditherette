import { atom } from 'nanostores';
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

/** Entries kept per direction. Snapshots share structure with the live settings, so this stays small. */
export const HISTORY_LIMIT = 100;
/** Edits to the same fields merge into one entry until this long after the last one. */
const SETTLE_MS = 400;

/** Every setting that changes the processed output. Preview pan and zoom are deliberately absent. */
type Snapshot = {
	output: OutputSettings;
	dither: DitherSettings;
	colorSpace: ColorSpaceId;
	effects: EffectLayer[];
	activePalette: string;
	customPalettes: Palette[];
	paletteEnabled: Record<string, boolean>;
};

function snapshot(): Snapshot {
	return {
		output: outputSettings.get(),
		dither: ditherSettings.get(),
		colorSpace: colorSpace.get(),
		effects: effectLayers.get(),
		activePalette: activePaletteName.get(),
		customPalettes: customPalettes.get(),
		paletteEnabled: paletteEnabled.get()
	};
}

function apply(next: Snapshot) {
	outputSettings.set(next.output);
	ditherSettings.set(next.dither);
	colorSpace.set(next.colorSpace);
	effectLayers.set(next.effects);
	customPalettes.set(next.customPalettes);
	paletteEnabled.set(next.paletteEnabled);
	activePaletteName.set(next.activePalette);
}

const same = (a: unknown, b: unknown) => a === b || JSON.stringify(a) === JSON.stringify(b);

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function changedFields(name: string, before: unknown, after: unknown): string[] {
	if (same(before, after)) return [];
	if (!isRecord(before) || !isRecord(after)) return [name];
	return [...new Set([...Object.keys(before), ...Object.keys(after)])]
		.filter((key) => !same(before[key], after[key]))
		.map((key) => `${name}.${key}`);
}

/** Names what differs between two snapshots, or an empty string when nothing does. */
function changeKey(before: Snapshot, after: Snapshot) {
	return (Object.keys(before) as Array<keyof Snapshot>)
		.flatMap((name) => changedFields(name, before[name], after[name]))
		.sort()
		.join('|');
}

/** Whether undo is possible, counting an edit still settling. */
export const canUndo = atom(false);
export const canRedo = atom(false);

let past: Snapshot[] = [];
let future: Snapshot[] = [];
/** The last committed state. */
let present: Snapshot;
/** The live settings as of the last settle. */
let seen: Snapshot;
let seenSource: unknown;
/** Fields of the edit still settling, or undefined once it is committed. */
let pendingKey: string | undefined;
let timer: ReturnType<typeof setTimeout> | undefined;
let settleQueued = false;
let pointerHeld = false;
let stopHistory: (() => void) | undefined;

function publish() {
	canUndo.set(past.length > 0 || pendingKey !== undefined);
	canRedo.set(future.length > 0);
}

/** Start over from the live settings, so a boundary leaves nothing to undo across it. */
function reset() {
	if (timer) clearTimeout(timer);
	timer = undefined;
	past = [];
	future = [];
	present = seen = snapshot();
	seenSource = sourceMeta.get();
	pendingKey = undefined;
	publish();
}

function commit() {
	if (timer) clearTimeout(timer);
	timer = undefined;
	if (pendingKey === undefined) return;
	pendingKey = undefined;
	if (changeKey(present, seen)) {
		past.push(present);
		if (past.length > HISTORY_LIMIT) past.shift();
		present = seen;
	}
	publish();
}

/** A drag holds its entry open until the pointer lifts. */
function armCommit() {
	if (timer) clearTimeout(timer);
	timer = pointerHeld ? undefined : setTimeout(commit, SETTLE_MS);
}

/** Runs once per burst of changes, so an action that sets several stores is one entry. */
function settle() {
	settleQueued = false;
	const next = snapshot();
	// A new source or crop changes the frame itself; undoing across it would show a stale output.
	if (sourceMeta.get() !== seenSource || !same(next.output.crop, seen.output.crop)) return reset();
	const key = changeKey(seen, next);
	if (!key) return;
	if (pendingKey !== undefined && pendingKey !== key) commit();
	future = [];
	pendingKey = key;
	seen = next;
	armCommit();
	publish();
}

function markChanged() {
	if (settleQueued) return;
	settleQueued = true;
	queueMicrotask(settle);
}

function step(from: Snapshot[], to: Snapshot[]) {
	if (settleQueued) settle();
	commit();
	const target = from.pop();
	if (!target) return;
	to.push(present);
	present = seen = target;
	apply(target);
	// Undo is a deliberate jump, so it skips the slider debounce.
	scheduleProcessing(0);
	publish();
}

export const undo = () => step(past, future);
export const redo = () => step(future, past);

type KeyEvent = Pick<KeyboardEvent, 'key' | 'shiftKey' | 'altKey' | 'metaKey' | 'ctrlKey'>;

/** Cmd+Z / Cmd+Shift+Z on macOS; Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y elsewhere. */
export function historyCommand(event: KeyEvent, mac: boolean) {
	const modifier = mac ? event.metaKey : event.ctrlKey;
	const foreign = mac ? event.ctrlKey : event.metaKey;
	if (!modifier || foreign || event.altKey) return;
	const key = event.key.toLowerCase();
	if (key === 'z') return event.shiftKey ? 'redo' : 'undo';
	if (key === 'y' && !mac && !event.shiftKey) return 'redo';
}

const TEXT_FIELD = [
	'textarea',
	'[contenteditable]:not([contenteditable="false"])',
	'input:not([type])',
	'input:is([type=text],[type=search],[type=number],[type=email],[type=url],[type=tel],[type=password])'
].join(',');

/** Text fields keep the browser's own undo; sliders, checkboxes and buttons have none. */
export function editsText(target: EventTarget | null) {
	return Boolean((target as Element | null)?.closest?.(TEXT_FIELD));
}

/** Track the settings from now on. Returns a stop function. */
export function startHistory() {
	if (stopHistory) return stopHistory;
	const stores = [
		outputSettings,
		ditherSettings,
		colorSpace,
		effectLayers,
		activePaletteName,
		customPalettes,
		paletteEnabled,
		sourceMeta
	];
	const unsubscribers = stores.map((store) => store.listen(markChanged));
	// Listening mounts persisted stores, which may load saved values, so the baseline comes after.
	reset();
	const hold = () => (pointerHeld = true);
	const release = () => {
		pointerHeld = false;
		if (pendingKey !== undefined) armCommit();
	};
	const pointerEvents = { capture: true } as const;
	window.addEventListener('pointerdown', hold, pointerEvents);
	window.addEventListener('pointerup', release, pointerEvents);
	window.addEventListener('pointercancel', release, pointerEvents);
	stopHistory = () => {
		for (const unsubscribe of unsubscribers) unsubscribe();
		window.removeEventListener('pointerdown', hold, pointerEvents);
		window.removeEventListener('pointerup', release, pointerEvents);
		window.removeEventListener('pointercancel', release, pointerEvents);
		if (timer) clearTimeout(timer);
		timer = undefined;
		pointerHeld = false;
		stopHistory = undefined;
	};
	return stopHistory;
}
