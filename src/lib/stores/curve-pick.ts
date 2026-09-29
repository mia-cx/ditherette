import { atom } from 'nanostores';

/**
 * A Curves editor picking from the preview. The preview hands it the source colour under a click,
 * then how far the pointer has moved up since, in CSS pixels.
 */
export type CurvePicker = {
	readonly pick: (rgb: readonly [number, number, number]) => void;
	readonly push: (up: number) => void;
};

/** The editor picking from the preview, if any. At most one picks at a time. */
export const curvePicker = atom<CurvePicker | undefined>();
