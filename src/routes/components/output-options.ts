import type { AlphaMode, DitherPlacement, ResizeId } from '$lib/processing/types';
import type { LabeledOption } from './option-types';

export const RESIZE_MODES = [
	{ id: 'lanczos3', label: 'Lanczos3' },
	{ id: 'lanczos3-scale-aware', label: 'Lanczos3 AA' },
	{ id: 'lanczos2', label: 'Lanczos2' },
	{ id: 'lanczos2-scale-aware', label: 'Lanczos2 AA' },
	{ id: 'bilinear', label: 'Bilinear' },
	{ id: 'nearest', label: 'Nearest' },
	{ id: 'area', label: 'Area / Box' }
] as const satisfies readonly LabeledOption<ResizeId>[];

export const ALPHA_MODES = [
	{ id: 'preserve', label: 'Preserve transparency' },
	{ id: 'premultiplied', label: 'Premultiplied' },
	{ id: 'matte', label: 'Matte' }
] as const satisfies readonly LabeledOption<AlphaMode>[];

export const PLACEMENT_MODES = [
	{ id: 'everywhere', label: 'Everywhere' },
	{ id: 'adaptive', label: 'Adaptive' }
] as const satisfies readonly LabeledOption<DitherPlacement>[];
