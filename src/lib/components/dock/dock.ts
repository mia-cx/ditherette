import { atom } from 'nanostores';
import { persistentAtom } from '@nanostores/persistent';
import type { DockviewApi, IDockviewGroupPanel } from 'dockview-core';

/** String parameters a window carries through serialized layouts, such as an effect layer id. */
export type DockParams = Readonly<Record<string, string>>;

/** Room for an effect editor without covering the preview. */
const FLOATING_SIZE = { width: 360, height: 520 };
/** Matches `--dv-tabs-and-actions-container-height` in dock.css. */
const TAB_BAR = 32;
/** Dockview's default group constraints, restored on expand. */
const MINIMUM_SIZE = 100;
const MAXIMUM_SIZE = Number.MAX_SAFE_INTEGER;

/**
 * How a group collapsed: to its tab bar (`height`), to a vertical tab strip when it has no
 * neighbour above or below (`width`), or a floating window rolled up to its title (`float`).
 * `size` is the size to restore.
 */
type Collapse = { axis: 'height' | 'width' | 'float'; size: number };

/**
 * Bumped on every collapse or expand. A floating window's height lives on its frame, which Dockview
 * does not report as a layout change, so layout persistence listens here too.
 */
export const collapseRevision = atom(0);

/** Collapsed groups by id. Group ids survive serialized layouts, so collapse persists too. */
export const collapsedGroups = persistentAtom<Record<string, Collapse>>(
	'ditherette:dock-collapsed',
	{},
	{
		encode: JSON.stringify,
		decode: (encoded) => {
			try {
				return JSON.parse(encoded) as Record<string, Collapse>;
			} catch {
				return {};
			}
		}
	}
);

export function isFloating(group: IDockviewGroupPanel) {
	return group.api.location.type === 'floating';
}

/** Float a docked window, or dock a floating one back on the right edge. */
export function toggleFloating(api: DockviewApi, group: IDockviewGroupPanel) {
	const panel = group.activePanel;
	// Moving a tab out would strand the rest of a collapsed group at its tab-bar size.
	const collapsed = collapsedGroups.get()[group.id];
	if (collapsed) expand(api, group, collapsed);
	if (isFloating(group)) group.api.moveTo({ position: 'right' });
	else if (panel) api.addFloatingGroup(panel, FLOATING_SIZE);
}

/** The group's root element; header actions only receive the element-less interface. */
function groupElement(api: DockviewApi, group: IDockviewGroupPanel) {
	return api.groups.find((candidate) => candidate.id === group.id)?.element;
}

/** A floating window's frame, which holds its size. */
function floatingFrame(api: DockviewApi, group: IDockviewGroupPanel) {
	return groupElement(api, group)?.closest<HTMLElement>('.dv-resize-container');
}

function constraints(axis: 'height' | 'width', minimum: number, maximum: number) {
	return axis === 'height'
		? { minimumHeight: minimum, maximumHeight: maximum }
		: { minimumWidth: minimum, maximumWidth: maximum };
}

/** Hold a group at its collapsed size. Also re-applies a saved collapse after a layout loads. */
function applyCollapse(api: DockviewApi, group: IDockviewGroupPanel, axis: Collapse['axis']) {
	if (axis === 'float') {
		const frame = floatingFrame(api, group);
		const content = frame?.querySelector<HTMLElement>('.dv-content-container');
		if (frame && content) frame.style.height = `${frame.offsetHeight - content.offsetHeight}px`;
		return;
	}
	if (axis === 'width') group.api.setHeaderPosition('left');
	group.api.setConstraints(constraints(axis, TAB_BAR, TAB_BAR));
	group.api.setSize({ [axis]: TAB_BAR });
}

function forget(group: IDockviewGroupPanel) {
	const { [group.id]: _removed, ...rest } = collapsedGroups.get();
	if (_removed) collapsedGroups.set(rest);
}

/** Collapse a group to its tab bar or title, or restore the size it had. */
export function toggleCollapsed(api: DockviewApi, group: IDockviewGroupPanel) {
	const collapsed = collapsedGroups.get()[group.id];
	if (collapsed) return expand(api, group, collapsed);
	const stacked = (['up', 'down'] as const).some((direction) =>
		api.adjacentGroupInDirection(group, direction)
	);
	const axis: Collapse['axis'] = isFloating(group) ? 'float' : stacked ? 'height' : 'width';
	const size =
		axis === 'float'
			? (floatingFrame(api, group)?.offsetHeight ?? FLOATING_SIZE.height)
			: axis === 'height'
				? group.api.height
				: group.api.width;
	collapsedGroups.set({ ...collapsedGroups.get(), [group.id]: { axis, size } });
	applyCollapse(api, group, axis);
	collapseRevision.set(collapseRevision.get() + 1);
}

function expand(api: DockviewApi, group: IDockviewGroupPanel, { axis, size }: Collapse) {
	forget(group);
	if (axis === 'float') {
		const frame = floatingFrame(api, group);
		if (frame) frame.style.height = `${size}px`;
	} else {
		if (axis === 'width') group.api.setHeaderPosition('top');
		group.api.setConstraints(constraints(axis, MINIMUM_SIZE, MAXIMUM_SIZE));
		group.api.setSize({ [axis]: size });
	}
	collapseRevision.set(collapseRevision.get() + 1);
}

/**
 * Re-apply saved collapses after a layout loads, drop records for groups that no longer exist,
 * and forget a collapse when its group is removed. Returns the cleanup.
 */
export function trackCollapse(api: DockviewApi) {
	const saved = collapsedGroups.get();
	const live = Object.fromEntries(
		Object.entries(saved).filter(([id]) => api.getGroup(id) !== undefined)
	);
	collapsedGroups.set(live);
	for (const group of api.groups) {
		const collapse = live[group.id];
		if (collapse) applyCollapse(api, group, collapse.axis);
	}
	const listener = api.onDidRemoveGroup((group) => forget(group));
	return () => listener.dispose();
}

/** Forget every collapse, for a layout reset. */
export function clearCollapsed() {
	collapsedGroups.set({});
}
