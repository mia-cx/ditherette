import type { DockviewApi, IDockviewGroupPanel } from 'dockview-core';

/** String parameters a window carries through serialized layouts, such as an effect layer id. */
export type DockParams = Readonly<Record<string, string>>;

/** Room for an effect editor without covering the preview. */
const FLOATING_SIZE = { width: 360, height: 520 };

export function isFloating(group: IDockviewGroupPanel) {
	return group.api.location.type === 'floating';
}

/** Float a docked window, or dock a floating one back on the right edge. */
export function toggleFloating(api: DockviewApi, group: IDockviewGroupPanel) {
	const panel = group.activePanel;
	if (isFloating(group)) group.api.moveTo({ position: 'right' });
	else if (panel) api.addFloatingGroup(panel, FLOATING_SIZE);
}
