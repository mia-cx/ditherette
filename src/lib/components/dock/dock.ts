import { atom } from 'nanostores';
import { persistentAtom } from '@nanostores/persistent';
import type { DockviewApi, IDockviewGroupPanel } from 'dockview-core';

/** String parameters a window carries through serialized layouts, such as an effect layer id. */
export type DockParams = Readonly<Record<string, string>>;

/** Room for an effect editor without covering the preview. */
const FLOATING_SIZE = { width: 360, height: 640 };
/** Space kept clear above and below a new floating window. */
const FLOATING_MARGIN = 24;

/** A new floating window's box, centred in the dock. */
export function centredFloating(api: DockviewApi) {
	const height = Math.min(FLOATING_SIZE.height, api.height - 2 * FLOATING_MARGIN);
	return {
		width: FLOATING_SIZE.width,
		height,
		x: Math.max(0, Math.round((api.width - FLOATING_SIZE.width) / 2)),
		y: Math.max(0, Math.round((api.height - height) / 2))
	};
}
/** Matches `--dv-tabs-and-actions-container-height` in dock.css. */
const TAB_BAR = 32;
/** Dockview's default group constraints, restored on expand. */
const MINIMUM_SIZE = 100;
const MAXIMUM_SIZE = Number.MAX_SAFE_INTEGER;

/**
 * How a group collapsed: rolled up to its tab bar (`height`), a floating window rolled up to its
 * title (`float`), or part of a column folded into a strip (`width`). `size` is the height, or
 * for a strip the column width, to restore. A strip also keeps each group's own `height` and
 * whether it was `rolled` up before the column folded.
 */
type Collapse =
	| { axis: 'height' | 'float'; size: number }
	| { axis: 'width'; size: number; height?: number; rolled?: boolean };

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
export function toggleFloating(
	api: DockviewApi,
	group: IDockviewGroupPanel,
	main: string | undefined
) {
	const panel = group.activePanel;
	// Moving a tab out would strand the rest of a collapsed group at its tab-bar size.
	const collapsed = collapsedGroups.get()[group.id];
	if (collapsed?.axis === 'width') toggleSidebar(api, group, main);
	else if (collapsed) expand(api, group, collapsed);
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
	const box = (candidate: IDockviewGroupPanel) =>
		groupElement(api, candidate)?.getBoundingClientRect();
	const own = box(group);
	if (!own) return [group];
	// Docked groups spanning exactly this group's width, top to bottom. Built from geometry rather
	// than nearest-neighbour hops, which can land in another column first in an uneven layout.
	const aligned = api.groups
		.filter((candidate) => !isFloating(candidate))
		.map((candidate) => ({ candidate, rect: box(candidate) }))
		.filter(
			({ rect }) =>
				rect && Math.abs(rect.left - own.left) < 1 && Math.abs(rect.right - own.right) < 1
		)
		.sort((first, second) => first.rect!.top - second.rect!.top);
	// Keep the run that touches this group, so a same-width window elsewhere doesn't join it.
	let start = aligned.findIndex(({ candidate }) => candidate === group);
	let end = start;
	while (start > 0 && Math.abs(aligned[start - 1]!.rect!.bottom - aligned[start]!.rect!.top) < 2)
		start--;
	while (
		end < aligned.length - 1 &&
		Math.abs(aligned[end]!.rect!.bottom - aligned[end + 1]!.rect!.top) < 2
	)
		end++;
	return aligned.slice(start, end + 1).map(({ candidate }) => candidate);
}

/**
 * The group holding `main`, the one area that never folds, or the widest docked group when
 * `main` is closed or floating.
 */
function mainGroup(api: DockviewApi, main: string | undefined) {
	const docked = main ? api.getPanel(main)?.group : undefined;
	if (docked && !isFloating(docked)) return docked;
	return api.groups
		.filter((candidate) => !isFloating(candidate))
		.reduce<
			IDockviewGroupPanel | undefined
		>((widest, candidate) => (!widest || candidate.api.width > widest.api.width ? candidate : widest), undefined);
}

function remember(group: IDockviewGroupPanel, collapse: Collapse) {
	collapsedGroups.set({ ...collapsedGroups.get(), [group.id]: collapse });
}

/**
 * Run a width change on `changing` while other docked groups keep their widths, so the main
 * area's column absorbs the difference. Dockview would share it out proportionally.
 */
function keepOtherWidths(
	api: DockviewApi,
	changing: readonly IDockviewGroupPanel[],
	main: string | undefined,
	change: () => void
) {
	const absorbing = mainGroup(api, main);
	const flexible = absorbing ? column(api, absorbing) : [];
	const held = api.groups
		.filter((other) => !changing.includes(other) && !flexible.includes(other) && !isFloating(other))
		.map((other) => [other, other.api.width] as const);
	change();
	for (const [other, width] of held) other.api.setSize({ width });
}

/**
 * Roll a window up to its tab bar, or a floating window up to its title, or restore the size it
 * had. The whole column folds into a strip only through `toggleSidebar`.
 */
export function toggleCollapsed(
	api: DockviewApi,
	group: IDockviewGroupPanel,
	main: string | undefined
) {
	const collapsed = collapsedGroups.get()[group.id];
	if (collapsed?.axis === 'width') return toggleSidebar(api, group, main);
	if (collapsed) return expand(api, group, collapsed);
	const axis = isFloating(group) ? 'float' : 'height';
	const size =
		axis === 'float'
			? (floatingFrame(api, group)?.offsetHeight ?? FLOATING_SIZE.height)
			: group.api.height;
	remember(group, { axis, size });
	applyCollapse(api, group, axis);
	collapseRevision.set(collapseRevision.get() + 1);
}

function expand(api: DockviewApi, group: IDockviewGroupPanel, collapse: Collapse) {
	forget(group);
	if (collapse.axis === 'float') {
		const frame = floatingFrame(api, group);
		if (frame) frame.style.height = `${collapse.size}px`;
	} else {
		group.api.setConstraints(constraints('height', MINIMUM_SIZE, MAXIMUM_SIZE));
		group.api.setSize({ height: collapse.size });
	}
	collapseRevision.set(collapseRevision.get() + 1);
}

/** Whether a window can roll up: it floats, or shares its column with another window. */
export function canCollapse(api: DockviewApi, group: IDockviewGroupPanel) {
	return isFloating(group) || column(api, group).length > 1;
}

/**
 * The side a docked column sits on, if `group` is the top window of a column other than the one
 * holding `main`. That window carries the sidebar button.
 */
export function sidebarSide(
	api: DockviewApi,
	group: IDockviewGroupPanel,
	main: string | undefined
): 'left' | 'right' | undefined {
	if (isFloating(group)) return undefined;
	const members = column(api, group);
	if (members[0] !== group) return undefined;
	const area = mainGroup(api, main);
	if (!area || members.some((member) => member.id === area.id)) return undefined;
	const left = (candidate: IDockviewGroupPanel) =>
		groupElement(api, candidate)?.getBoundingClientRect().left ?? 0;
	return left(group) < left(area) ? 'left' : 'right';
}

/**
 * Fold `group`'s whole column into a strip of vertical tabs, or open it again. Windows rolled up
 * to their tab bars before the column folded stay rolled up when it opens.
 */
export function toggleSidebar(
	api: DockviewApi,
	group: IDockviewGroupPanel,
	main: string | undefined
) {
	const groups = column(api, group);
	const saved = collapsedGroups.get();
	const top = groups[0]!;
	const folded = saved[top.id];
	if (folded?.axis === 'width') {
		keepOtherWidths(api, groups, main, () => {
			for (const member of groups) {
				const record = saved[member.id];
				member.api.setHeaderPosition('top');
				member.api.setConstraints(constraints('width', MINIMUM_SIZE, MAXIMUM_SIZE));
				forget(member);
				if (record?.axis !== 'width' || record.height === undefined) continue;
				if (record.rolled) {
					remember(member, { axis: 'height', size: record.height });
					applyCollapse(api, member, 'height');
				} else member.api.setSize({ height: record.height });
			}
			top.api.setSize({ width: folded.size });
		});
	} else {
		const width = top.api.width;
		const share = Math.floor(
			groups.reduce((total, member) => total + member.api.height, 0) / groups.length
		);
		keepOtherWidths(api, groups, main, () => {
			for (const member of groups) {
				const record = saved[member.id];
				const rolled = record?.axis === 'height';
				const height = rolled ? record.size : member.api.height;
				remember(member, { axis: 'width', size: width, height, rolled });
				applyCollapse(api, member, 'width');
			}
		});
		// Split the strip evenly, so every window's vertical tabs have room.
		for (const member of groups.slice(0, -1)) member.api.setSize({ height: share });
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
