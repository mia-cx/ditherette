import { atom } from 'nanostores';
import { persistentAtom } from '@nanostores/persistent';
import type { DockviewApi, SerializedDockview } from 'dockview-core';
import { effectLayers, type EffectLayer } from '$lib/stores/effects';

/** The studio's fixed windows, in Windows-menu order. Their ids double as component names. */
export const WINDOWS = [
	{ id: 'preview', title: 'Preview' },
	{ id: 'effects', title: 'Effects' },
	{ id: 'dimensions', title: 'Dimensions' },
	{ id: 'dither', title: 'Dither' },
	{ id: 'color-space', title: 'Color space' },
	{ id: 'palette', title: 'Palette' }
] as const;
export type WindowId = (typeof WINDOWS)[number]['id'];

/** Every effect instance gets its own window, rendered by this component. */
export const EFFECT_COMPONENT = 'effect';
const EFFECT_PREFIX = 'effect:';
const effectWindowId = (layerId: string) => `${EFFECT_PREFIX}${layerId}`;
const PERSIST_DELAY_MS = 250;
/** Side columns take a share of the dock, within these bounds, so the preview keeps the rest. */
const EFFECTS_WIDTH = { share: 0.2, min: 240, max: 300 };
const STAGES_WIDTH = { share: 0.28, min: 300, max: 400 };
const columnWidth = (api: DockviewApi, { share, min, max }: typeof EFFECTS_WIDTH) =>
	Math.round(Math.min(max, Math.max(min, api.width * share)));

export const studioApi = atom<DockviewApi | undefined>();
/** Ids of the windows currently open, for the Windows menu. */
export const openWindows = atom<ReadonlySet<string>>(new Set());

export const studioLayout = persistentAtom<SerializedDockview | null>(
	'ditherette:studio-layout',
	null,
	{
		encode: JSON.stringify,
		decode: (encoded) => {
			try {
				return JSON.parse(encoded) as SerializedDockview;
			} catch {
				return null;
			}
		}
	}
);

function windowOptions(id: WindowId) {
	const { title } = WINDOWS.find((window) => window.id === id)!;
	return { id, component: id, title };
}

/** Preview in the middle, effects on the left, processing stages tabbed on the right. */
function defaultLayout(api: DockviewApi) {
	api.clear();
	api.addPanel(windowOptions('preview'));
	api.addPanel({
		...windowOptions('effects'),
		position: { referencePanel: 'preview', direction: 'left' },
		initialWidth: columnWidth(api, EFFECTS_WIDTH)
	});
	api.addPanel({
		...windowOptions('dimensions'),
		position: { referencePanel: 'preview', direction: 'right' },
		initialWidth: columnWidth(api, STAGES_WIDTH)
	});
	for (const id of ['dither', 'color-space'] as const)
		api.addPanel({
			...windowOptions(id),
			position: { referencePanel: 'dimensions', direction: 'within' },
			inactive: true
		});
	api.addPanel({
		...windowOptions('palette'),
		position: { referencePanel: 'dimensions', direction: 'below' }
	});
	// Initial widths are ignored while the dock is still empty, so size the side columns last.
	api.getPanel('effects')?.group.api.setSize({ width: columnWidth(api, EFFECTS_WIDTH) });
	api.getPanel('dimensions')?.group.api.setSize({ width: columnWidth(api, STAGES_WIDTH) });
}

/** Restore the saved layout, or dock the default one when there is none or it no longer loads. */
export function startStudio(api: DockviewApi) {
	const saved = studioLayout.get();
	try {
		if (!saved) throw new Error('No saved layout.');
		api.fromJSON(saved);
	} catch {
		defaultLayout(api);
	}
	const syncOpen = () => openWindows.set(new Set(api.panels.map((panel) => panel.id)));
	let timer: ReturnType<typeof setTimeout> | undefined;
	// Snapshot on every change and write it later, so teardown can still save the latest one.
	let pending: SerializedDockview | undefined;
	const save = () => {
		clearTimeout(timer);
		if (pending) studioLayout.set(pending);
		pending = undefined;
	};
	const listeners = [
		api.onDidAddPanel(syncOpen),
		api.onDidRemovePanel(syncOpen),
		api.onDidLayoutChange(() => {
			pending = api.toJSON();
			clearTimeout(timer);
			timer = setTimeout(save, PERSIST_DELAY_MS);
		})
	];
	syncOpen();
	studioApi.set(api);
	return () => {
		save();
		for (const listener of listeners) listener.dispose();
		studioApi.set(undefined);
	};
}

export function resetLayout() {
	const api = studioApi.get();
	if (api) defaultLayout(api);
}

/** Open a closed window where the default layout puts it, or close an open one. */
export function toggleWindow(id: WindowId) {
	const api = studioApi.get();
	if (!api) return;
	const open = api.getPanel(id);
	if (open) return api.removePanel(open);
	const anchor = api.panels.find((panel) => !panel.id.startsWith(EFFECT_PREFIX));
	api.addPanel({
		...windowOptions(id),
		...(anchor ? { position: { referencePanel: anchor, direction: 'within' } } : {})
	});
}

/** Show a layer's window, opening it beside the other effect windows, or below the effects list. */
export function openEffectWindow(layerId: string) {
	const api = studioApi.get();
	const layer = effectLayers.get().find((candidate) => candidate.id === layerId);
	if (!api || !layer) return;
	const id = effectWindowId(layer.id);
	const existing = api.getPanel(id);
	if (existing) return existing.api.setActive();
	const sibling = api.panels.find((panel) => panel.id.startsWith(EFFECT_PREFIX));
	const effects = api.getPanel('effects');
	api.addPanel({
		id,
		component: EFFECT_COMPONENT,
		title: layer.name,
		params: { layerId: layer.id },
		...(sibling
			? { position: { referencePanel: sibling, direction: 'within' } }
			: effects
				? { position: { referencePanel: effects, direction: 'below' } }
				: { floating: true })
	});
}

/** Keep effect window titles on their layer names, and close windows whose layer is gone. */
export function syncEffectWindows(api: DockviewApi, layers: readonly EffectLayer[]) {
	const names = new Map(layers.map((layer) => [effectWindowId(layer.id), layer.name]));
	for (const panel of api.panels.filter((candidate) => candidate.id.startsWith(EFFECT_PREFIX))) {
		const name = names.get(panel.id);
		if (name === undefined) api.removePanel(panel);
		else if (panel.title !== name) panel.api.setTitle(name);
	}
}
