import { computed, atom } from 'nanostores';
import { persistentJSON } from '@nanostores/persistent';
import { defaultEnabledState, enabledPalette } from '$lib/palette/wplace';
import type {
	ColorSpaceId,
	DitherSettings,
	OutputSettings,
	ProcessedImage,
	SourceImageRecord
} from '$lib/processing/types';

export const outputSettings = persistentJSON<OutputSettings>('ditherette:output', {
	width: 512,
	height: 512,
	lockAspect: true,
	fit: 'contain',
	resize: 'lanczos3',
	alphaMode: 'preserve',
	alphaThreshold: 0,
	matteKey: '#FFFFFF'
});

export const ditherSettings = persistentJSON<DitherSettings>('ditherette:dither', {
	algorithm: 'none',
	strength: 100,
	coverage: 'full',
	serpentine: true,
	seed: 0xc0ffee42
});

export const colorSpace = persistentJSON<ColorSpaceId>('ditherette:color-space', 'oklab');

export type PreviewMode = 'side-by-side' | 'ab-reveal';
export type PreviewSettings = {
	mode?: PreviewMode;
	revealValue?: number;
};

export const previewSettings = persistentJSON<PreviewSettings>('ditherette:preview', {});

export const paletteEnabled = persistentJSON<Record<string, boolean>>(
	'ditherette:palette-enabled',
	defaultEnabledState()
);

export const selectedPalette = computed(paletteEnabled, (enabled) => enabledPalette(enabled));

export type SourceMeta = Omit<SourceImageRecord, 'blob'>;

export const sourceMeta = atom<SourceMeta | undefined>();
export const sourceObjectUrl = atom<string | undefined>();
export const sourceImageData = atom<ImageData | undefined>();
export const processedImage = atom<ProcessedImage | undefined>();
export const processingProgress = atom<{ stage: string; progress: number } | undefined>();
export const processingError = atom<string | undefined>();

export const hasImage = computed(sourceMeta, (source) => Boolean(source));
export const activeColorCount = computed(selectedPalette, (palette) => palette.length);

export function updateOutputSettings(patch: Partial<OutputSettings>) {
	outputSettings.set({ ...outputSettings.get(), ...patch });
}

export function updateDitherSettings(patch: Partial<DitherSettings>) {
	ditherSettings.set({ ...ditherSettings.get(), ...patch });
}

export function updatePreviewSettings(patch: Partial<PreviewSettings>) {
	previewSettings.set({ ...previewSettings.get(), ...patch });
}

export function setPaletteColorEnabled(key: string, enabled: boolean) {
	paletteEnabled.set({ ...paletteEnabled.get(), [key]: enabled });
}

export function setAllPaletteColors(enabled: boolean) {
	paletteEnabled.set(
		Object.fromEntries(Object.keys(paletteEnabled.get()).map((key) => [key, enabled]))
	);
}

export function togglePaletteColors() {
	paletteEnabled.set(
		Object.fromEntries(
			Object.entries(paletteEnabled.get()).map(([key, enabled]) => [key, !enabled])
		)
	);
}

export function clearInMemoryImageState() {
	const url = sourceObjectUrl.get();
	if (url) URL.revokeObjectURL(url);
	sourceMeta.set(undefined);
	sourceObjectUrl.set(undefined);
	sourceImageData.set(undefined);
	processedImage.set(undefined);
	processingProgress.set(undefined);
	processingError.set(undefined);
}
