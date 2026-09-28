import { atom } from 'nanostores';
import { persistentAtom } from '@nanostores/persistent';
import type { DockviewApi, IDockviewGroupPanel } from 'dockview-core';

/** String parameters a window carries through serialized layouts, such as an effect layer id. */
export type DockParams = Readonly<Record<string, string>>;

/** Room for an effect editor without covering the preview. */
const FLOATING_SIZE = { width: 360, height: 520 };

/** A new floating window's box, centred in the dock. */
export function centredFloating(api: DockviewApi) {
	return {
		...FLOATING_SIZE,
		x: Math.max(0, Math.round((api.width - FLOATING_SIZE.width) / 2)),
		y: Math.max(0, Math.round((api.height - FLOATING_SIZE.height) / 2))
	};
}
/** Matches `--dv-tabs-and-actions-container-height` in dock.css. */
const TAB_BAR = 32;
/** Dockview's default group constraints, restored on expand. */
const MINIMUM_SIZE = 100;
const MAXIMUM_SIZE = Number.MAX_SAFE_INTEGER;

/**
 * How a group collapsed: to its tab bar (`height`), to a vertical tab strip (`width`), or a
 * floating window rolled up to its title (`float`). `size` is the size to restore. A strip is
 * either a lone group or a whole column whose groups all collapsed; in a column, `height` is the
 * group's own height to restore when the column opens again.
 */
type Collapse =
	| { axis: 'height' | 'float'; size: number }
	| { axis: 'width'; size: number; height?: number };

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
	// A strip in a column shares the column's height, so drop any tab-bar height it held.
	if (axis === 'width') group.api.setConstraints(constraints('height', MINIMUM_SIZE, MAXIMUM_SIZE));
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

/** The groups stacked above and below `group`, itself included, top to bottom. */
function column(api: DockviewApi, group: IDockviewGroupPanel) {
	const walk = (direction: 'up' | 'down') => {
		const found: IDockviewGroupPanel[] = [];
		for (let next = api.adjacentGroupInDirection(group, direction); next; ) {
			found.push(next);
			next = api.adjacentGroupInDirection(next, direction);
		}
		return found;
	};
	return [...walk('up').reverse(), group, ...walk('down')];
}

function remember(group: IDockviewGroupPanel, collapse: Collapse) {
	collapsedGroups.set({ ...collapsedGroups.get(), [group.id]: collapse });
}

/**
 * Run a width change on `changing` while other docked groups keep their widths, so the widest one,
 * the preview in the studio, absorbs the difference. Dockview would share it out proportionally.
 */
function keepOtherWidths(
	api: DockviewApi,
	changing: readonly IDockviewGroupPanel[],
	change: () => void
) {
	const others = api.groups.filter(
		(other) => !changing.includes(other) && other.api.location.type === 'grid'
	);
	const widest = others.reduce<IDockviewGroupPanel | undefined>(
		(best, other) => (!best || other.api.width > best.api.width ? other : best),
		undefined
	);
	const held = others
		.filter((other) => other !== widest)
		.map((other) => [other, other.api.width] as const);
	change();
	for (const [other, width] of held) other.api.setSize({ width });
}

/**
 * Collapse a group to its tab bar or title, or restore the size it had. Collapsing the last open
 * group in a column collapses the whole column to a strip, like a lone group.
 */
export function toggleCollapsed(api: DockviewApi, group: IDockviewGroupPanel) {
	const collapsed = collapsedGroups.get()[group.id];
	if (collapsed) return expand(api, group, collapsed);
	if (isFloating(group)) {
		const size = floatingFrame(api, group)?.offsetHeight ?? FLOATING_SIZE.height;
		remember(group, { axis: 'float', size });
		applyCollapse(api, group, 'float');
	} else {
		const groups = column(api, group);
		const saved = collapsedGroups.get();
		if (groups.every((member) => member === group || saved[member.id]?.axis === 'height')) {
			// A column's groups share one width, so they collapse to the strip together.
			const width = group.api.width;
			const lone = groups.length === 1;
			const share = Math.floor(
				groups.reduce((total, member) => total + member.api.height, 0) / groups.length
			);
			keepOtherWidths(api, groups, () => {
				for (const member of groups) {
					const height = member === group ? group.api.height : saved[member.id]!.size;
					remember(
						member,
						lone ? { axis: 'width', size: width } : { axis: 'width', size: width, height }
					);
					applyCollapse(api, member, 'width');
				}
			});
			// Split the strip evenly, so every group's vertical tabs have room.
			for (const member of groups.slice(0, -1)) member.api.setSize({ height: share });
		} else {
			remember(group, { axis: 'height', size: group.api.height });
			applyCollapse(api, group, 'height');
		}
	}
	collapseRevision.set(collapseRevision.get() + 1);
}

function expand(api: DockviewApi, group: IDockviewGroupPanel, collapse: Collapse) {
	const { axis, size } = collapse;
	forget(group);
	if (axis === 'width' && collapse.height !== undefined) {
		// Opening one group of a collapsed column widens it; the rest go back to their tab bars.
		const saved = collapsedGroups.get();
		const groups = column(api, group);
		keepOtherWidths(api, groups, () => {
			for (const member of groups) {
				member.api.setHeaderPosition('top');
				member.api.setConstraints(constraints('width', MINIMUM_SIZE, MAXIMUM_SIZE));
				const record = saved[member.id];
				if (member === group || record?.axis !== 'width' || record.height === undefined) continue;
				remember(member, { axis: 'height', size: record.height });
				applyCollapse(api, member, 'height');
			}
			group.api.setSize({ width: size, height: collapse.height });
		});
	} else if (axis === 'float') {
		const frame = floatingFrame(api, group);
		if (frame) frame.style.height = `${size}px`;
	} else {
		const resize = () => {
			if (axis === 'width') group.api.setHeaderPosition('top');
			group.api.setConstraints(constraints(axis, MINIMUM_SIZE, MAXIMUM_SIZE));
			group.api.setSize({ [axis]: size });
		};
		if (axis === 'width') keepOtherWidths(api, [group], resize);
		else resize();
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
