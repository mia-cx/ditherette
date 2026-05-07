import { computed, atom } from 'nanostores';
import { persistentJSON } from '@nanostores/persistent';
import {
	TRANSPARENT_KEY,
	WPLACE,
	WPLACE_PALETTE_NAME,
	defaultEnabledState,
	enabledPalette,
	paletteEnabledKey
} from '$lib/palette/wplace';
import type {
	ColorSpaceId,
	DitherSettings,
	OutputSettings,
	Palette,
	PaletteColor,
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
	matteKey: '#FFFFFF',
	autoSizeOnUpload: false,
	scaleFactor: 1
});

export const ditherSettings = persistentJSON<DitherSettings>('ditherette:dither', {
	algorithm: 'none',
	strength: 100,
	placement: 'everywhere',
	placementRadius: 3,
	placementThreshold: 12,
	placementSoftness: 8,
	serpentine: true,
	seed: 0xc0ffee42,
	useColorSpace: false
});

export const colorSpace = persistentJSON<ColorSpaceId>('ditherette:color-space', 'oklab');

export type PreviewMode = 'side-by-side' | 'ab-reveal';
export type PreviewSettings = {
	mode?: PreviewMode;
	revealValue?: number;
	desktopPaneLayout?: [number, number];
	zoom?: number;
	panX?: number;
	panY?: number;
	frameWidth?: number;
	frameHeight?: number;
	originX?: number;
	originY?: number;
};

export const previewSettings = persistentJSON<PreviewSettings>('ditherette:preview', {});
export const uiSettings = persistentJSON<{
	desktopDitherFiltersOpen?: boolean;
	controlAccordionSections?: string[];
}>('ditherette:ui', {});
export const activePaletteName = persistentJSON<string>(
	'ditherette:active-palette',
	WPLACE_PALETTE_NAME
);
export const customPalettes = persistentJSON<Palette[]>('ditherette:custom-palettes', []);

export const paletteEnabled = persistentJSON<Record<string, boolean>>(
	'ditherette:palette-enabled',
	defaultEnabledState()
);

const TRANSPARENT_COLOR: PaletteColor = {
	name: 'Transparent',
	key: TRANSPARENT_KEY,
	kind: 'transparent'
};

function withTransparentSwatch(palette: Palette): Palette {
	const visible = palette.colors.filter((color) => color.key !== TRANSPARENT_KEY);
	return { ...palette, colors: [...visible, TRANSPARENT_COLOR] };
}

export const palettes = computed(customPalettes, (custom) => [
	WPLACE,
	...custom.map(withTransparentSwatch)
]);
export const activePalette = computed([activePaletteName, customPalettes], (name, custom) => {
	const palette = custom.find((item) => item.name === name);
	return palette ? withTransparentSwatch(palette) : WPLACE;
});
export const selectedPalette = computed([activePalette, paletteEnabled], (palette, enabled) =>
	enabledPalette(palette, enabled)
);

export type SourceMeta = Omit<SourceImageRecord, 'blob'>;

export const sourceMeta = atom<SourceMeta | undefined>();
export const sourceObjectUrl = atom<string | undefined>();
export const sourceImageData = atom<ImageData | undefined>();
export const processedImage = atom<ProcessedImage | undefined>();
export const processingProgress = atom<{ stage: string; progress: number } | undefined>();
export const processingError = atom<string | undefined>();

export const hasImage = computed(sourceMeta, (source) => Boolean(source));
export const activeColorCount = computed(selectedPalette, (palette) => palette.length);

function shallowEqual<T extends object>(left: T, right: T) {
	const leftKeys = Object.keys(left) as Array<keyof T>;
	const rightKeys = Object.keys(right) as Array<keyof T>;
	return leftKeys.length === rightKeys.length && leftKeys.every((key) => left[key] === right[key]);
}

function normalizeHex(hex: string) {
	const trimmed = hex.trim();
	const expanded = /^#[0-9a-fA-F]{3}$/.test(trimmed)
		? `#${trimmed[1]}${trimmed[1]}${trimmed[2]}${trimmed[2]}${trimmed[3]}${trimmed[3]}`
		: trimmed;
	if (!/^#[0-9a-fA-F]{6}$/.test(expanded)) throw new Error('Use #RGB or #RRGGBB hex.');
	return expanded.toUpperCase();
}

function hexToRgb(hex: string) {
	return {
		r: Number.parseInt(hex.slice(1, 3), 16),
		g: Number.parseInt(hex.slice(3, 5), 16),
		b: Number.parseInt(hex.slice(5, 7), 16)
	};
}

function activeCustomPalette() {
	const palette = activePalette.get();
	if (palette.source !== 'custom')
		throw new Error('Duplicate or create a custom palette before editing.');
	return palette;
}

function setCustomPalette(nextPalette: Palette) {
	customPalettes.set(
		customPalettes
			.get()
			.map((palette) => (palette.name === nextPalette.name ? nextPalette : palette))
	);
}

function uniquePaletteName(baseName: string) {
	const names = new Set(palettes.get().map((palette) => palette.name));
	const name = baseName.trim() || 'Custom palette';
	if (!names.has(name)) return name;
	let suffix = 2;
	while (names.has(`${name} ${suffix}`)) suffix++;
	return `${name} ${suffix}`;
}

function visibleCustomColor(name: string, hex: string): PaletteColor {
	const key = normalizeHex(hex);
	return { name: name.trim() || key, key, rgb: hexToRgb(key), kind: 'custom' };
}

export function updateOutputSettings(patch: Partial<OutputSettings>) {
	const next = { ...outputSettings.get(), ...patch };
	if (!shallowEqual(outputSettings.get(), next)) outputSettings.set(next);
}

export function updateDitherSettings(patch: Partial<DitherSettings>) {
	const next = { ...ditherSettings.get(), ...patch };
	if (!shallowEqual(ditherSettings.get(), next)) ditherSettings.set(next);
}

export function updatePreviewSettings(patch: Partial<PreviewSettings>) {
	const next = { ...previewSettings.get(), ...patch };
	if (!shallowEqual(previewSettings.get(), next)) previewSettings.set(next);
}

export function setPaletteColorEnabled(
	key: string,
	enabled: boolean,
	paletteName = activePalette.get().name
) {
	const stateKey = paletteEnabledKey(paletteName, key);
	if (paletteEnabled.get()[stateKey] === enabled) return;
	paletteEnabled.set({ ...paletteEnabled.get(), [stateKey]: enabled });
}

export function setAllPaletteColors(enabled: boolean) {
	const palette = activePalette.get();
	paletteEnabled.set({
		...paletteEnabled.get(),
		...Object.fromEntries(
			palette.colors.map((color) => [paletteEnabledKey(palette.name, color.key), enabled])
		)
	});
}

export function togglePaletteColors() {
	const palette = activePalette.get();
	const current = paletteEnabled.get();
	paletteEnabled.set({
		...current,
		...Object.fromEntries(
			palette.colors.map((color) => {
				const key = paletteEnabledKey(palette.name, color.key);
				return [key, current[key] === false];
			})
		)
	});
}

export function duplicateActivePalette(name = `${activePalette.get().name} Copy`) {
	const source = activePalette.get();
	const nextName = uniquePaletteName(name);
	const currentEnabled = paletteEnabled.get();
	const nextPalette: Palette = {
		name: nextName,
		source: 'custom',
		colors: source.colors.map((color) => ({
			...color,
			kind: color.kind === 'transparent' ? 'transparent' : 'custom'
		}))
	};
	customPalettes.set([...customPalettes.get(), nextPalette]);
	activePaletteName.set(nextName);
	paletteEnabled.set({
		...currentEnabled,
		...Object.fromEntries(
			nextPalette.colors.map((color) => [
				paletteEnabledKey(nextName, color.key),
				currentEnabled[paletteEnabledKey(source.name, color.key)] !== false
			])
		)
	});
	return nextPalette;
}

export function createCustomPalette(name: string) {
	const nextName = uniquePaletteName(name);
	const nextPalette: Palette = {
		name: nextName,
		source: 'custom',
		colors: [{ name: 'Transparent', key: TRANSPARENT_KEY, kind: 'transparent' }]
	};
	customPalettes.set([...customPalettes.get(), nextPalette]);
	activePaletteName.set(nextName);
	setPaletteColorEnabled(TRANSPARENT_KEY, true, nextName);
	return nextPalette;
}

export function addColorToActivePalette(name: string, hex: string) {
	const palette = activeCustomPalette();
	if (palette.colors.length >= 256) throw new Error('Palettes can contain at most 256 colors.');
	const color = visibleCustomColor(name, hex);
	if (palette.colors.some((existing) => existing.key === color.key))
		throw new Error(`${color.key} already exists in this palette.`);
	setCustomPalette({
		...palette,
		colors: [
			...palette.colors.filter((c) => c.key !== TRANSPARENT_KEY),
			color,
			...palette.colors.filter((c) => c.key === TRANSPARENT_KEY)
		]
	});
	setPaletteColorEnabled(color.key, true, palette.name);
}

export function duplicateActivePaletteColor(key: string, name: string, hex: string) {
	const palette = activeCustomPalette();
	const current = palette.colors.find((color) => color.key === key);
	if (!current || current.kind === 'transparent')
		throw new Error('Only custom visible colors can be duplicated.');
	addColorToActivePalette(name, hex);
}

export function editActivePaletteColor(key: string, name: string, hex: string) {
	const palette = activeCustomPalette();
	const current = palette.colors.find((color) => color.key === key);
	if (!current || current.kind === 'transparent')
		throw new Error('Only custom visible colors can be edited.');
	const nextColor = visibleCustomColor(name, hex);
	if (nextColor.key !== key && palette.colors.some((color) => color.key === nextColor.key))
		throw new Error(`${nextColor.key} already exists in this palette.`);
	setCustomPalette({
		...palette,
		colors: palette.colors.map((color) => (color.key === key ? nextColor : color))
	});
	const enabled = paletteEnabled.get()[paletteEnabledKey(palette.name, key)] !== false;
	setPaletteColorEnabled(nextColor.key, enabled, palette.name);
}

export function deleteActivePaletteColors(keys: string[]) {
	const palette = activeCustomPalette();
	const removable = new Set(keys.filter((key) => key !== TRANSPARENT_KEY));
	setCustomPalette({
		...palette,
		colors: palette.colors.filter((color) => !removable.has(color.key))
	});
}

export function deleteActiveCustomPalette() {
	const palette = activeCustomPalette();
	customPalettes.set(customPalettes.get().filter((item) => item.name !== palette.name));
	activePaletteName.set(WPLACE_PALETTE_NAME);
}

export function previewCustomPaletteImport(value: unknown) {
	const records = Array.isArray(value) ? value : [value];
	const palettes = records.map((record) => parseImportedPalette(record));
	const currentNames = new Set(customPalettes.get().map((palette) => palette.name));
	return {
		palettes,
		overwrites: palettes
			.filter((palette) => currentNames.has(palette.name))
			.map((palette) => palette.name)
	};
}

export function importCustomPaletteData(value: unknown) {
	const { palettes: imported } = previewCustomPaletteImport(value);
	const currentPalettes = customPalettes.get();
	const existing = new Map(currentPalettes.map((palette) => [palette.name, palette]));
	const currentEnabled = paletteEnabled.get();
	const nextEnabled: Record<string, boolean> = { ...currentEnabled };
	for (const palette of imported) {
		const previous = existing.get(palette.name);
		existing.set(palette.name, palette);
		for (const color of palette.colors) {
			nextEnabled[paletteEnabledKey(palette.name, color.key)] = importedEnabledState(
				palette.name,
				color,
				previous,
				currentEnabled
			);
		}
	}
	customPalettes.set([...existing.values()]);
	if (imported[0]) activePaletteName.set(imported[0].name);
	paletteEnabled.set(nextEnabled);
	return imported.length;
}

function importedEnabledState(
	paletteName: string,
	color: PaletteColor,
	previous: Palette | undefined,
	enabled: Record<string, boolean>
) {
	if (!previous) return true;
	const exactKey = paletteEnabledKey(paletteName, color.key);
	if (exactKey in enabled) return enabled[exactKey] !== false;
	if (!color.rgb) return true;
	const nearest = previous.colors
		.filter((candidate) => candidate.rgb)
		.reduce<PaletteColor | undefined>((best, candidate) => {
			if (!candidate.rgb) return best;
			if (!best?.rgb) return candidate;
			return rgbDistanceSquared(color.rgb!, candidate.rgb) <
				rgbDistanceSquared(color.rgb!, best.rgb)
				? candidate
				: best;
		}, undefined);
	return nearest ? enabled[paletteEnabledKey(paletteName, nearest.key)] !== false : true;
}

function rgbDistanceSquared(
	left: { r: number; g: number; b: number },
	right: { r: number; g: number; b: number }
) {
	const dr = left.r - right.r;
	const dg = left.g - right.g;
	const db = left.b - right.b;
	return dr * dr + dg * dg + db * db;
}

function parseImportedPalette(value: unknown): Palette {
	if (!value || typeof value !== 'object') throw new Error('Palette import must be an object.');
	const record = value as { name?: unknown; colors?: unknown };
	if (typeof record.name !== 'string' || !record.name.trim())
		throw new Error('Imported palette needs a name.');
	if (record.name === WPLACE_PALETTE_NAME)
		throw new Error('Imported palette cannot replace Wplace.');
	if (!Array.isArray(record.colors)) throw new Error('Imported palette needs a colors array.');
	const importedColors = record.colors.map((color, index) => parseImportedColor(color, index));
	const colors = withTransparentSwatch({
		name: record.name.trim(),
		source: 'custom',
		colors: importedColors
	}).colors;
	if (colors.length > 256) throw new Error('Imported palettes can contain at most 256 colors.');
	const visibleKeys = colors
		.filter((color) => color.key !== TRANSPARENT_KEY)
		.map((color) => color.key);
	if (new Set(visibleKeys).size !== visibleKeys.length)
		throw new Error('Imported palette has duplicate colors.');
	return { name: record.name.trim(), source: 'custom', colors };
}

function parseImportedColor(value: unknown, index: number): PaletteColor {
	if (!value || typeof value !== 'object') throw new Error(`Color ${index + 1} must be an object.`);
	const record = value as { name?: unknown; key?: unknown; hex?: unknown; kind?: unknown };
	if (record.key === TRANSPARENT_KEY || record.kind === 'transparent') {
		return { name: 'Transparent', key: TRANSPARENT_KEY, kind: 'transparent' };
	}
	const hex = typeof record.key === 'string' ? record.key : record.hex;
	if (typeof hex !== 'string') throw new Error(`Color ${index + 1} needs a hex key.`);
	return visibleCustomColor(typeof record.name === 'string' ? record.name : '', hex);
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
