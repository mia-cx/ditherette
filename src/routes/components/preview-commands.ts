import { atom } from 'nanostores';

/** View commands of the mounted preview, for menus and keyboard shortcuts. */
export type PreviewCommands = {
	zoomIn(): void;
	zoomOut(): void;
	fit(): void;
	actualSize(): void;
	toggleCrop(): void;
};

/** The mounted preview's commands; undefined while no preview is mounted. */
export const previewCommands = atom<PreviewCommands | undefined>();
/** Whether the preview is in crop mode. */
export const cropping = atom(false);
