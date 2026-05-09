export const MAX_OUTPUT_PIXELS = 67_108_864;
export const MAX_OUTPUT_SIDE = 16_384;
export const MAX_SOURCE_PIXELS = MAX_OUTPUT_PIXELS;
export const MAX_SOURCE_SIDE = 32_768;
export const MAX_SOURCE_BYTES = 75 * 1024 * 1024;

export const ACCEPTED_IMAGE_TYPES = ['image/png', 'image/jpeg', 'image/webp', 'image/gif'] as const;
const ACCEPTED_IMAGE_TYPE_SET = new Set<string>(ACCEPTED_IMAGE_TYPES);
export type AcceptedImageType = (typeof ACCEPTED_IMAGE_TYPES)[number];

export function isAcceptedImageType(type: string): type is AcceptedImageType {
	return ACCEPTED_IMAGE_TYPE_SET.has(type);
}

export type ColorSpaceId =
	| 'oklab'
	| 'srgb'
	| 'linear-rgb'
	| 'weighted-rgb'
	| 'weighted-rgb-601'
	| 'weighted-rgb-709'
	| 'cielab'
	| 'oklch';

export type DitherId =
	| 'none'
	| 'bayer-2'
	| 'bayer-4'
	| 'bayer-8'
	| 'bayer-16'
	| 'floyd-steinberg'
	| 'sierra'
	| 'sierra-lite'
	| 'random';

export type ResizeId = 'nearest' | 'bilinear' | 'lanczos3' | 'area';
export type FitMode = 'stretch' | 'contain' | 'cover';
export type AlphaMode = 'preserve' | 'premultiplied' | 'matte';

export type Rgb = { r: number; g: number; b: number };
export type CropRect = { x: number; y: number; width: number; height: number };

export type PaletteColor = {
	name: string;
	key: string;
	rgb?: Rgb;
	kind: 'free' | 'premium' | 'transparent' | 'custom';
	tags?: string[];
};

export type Palette = {
	name: string;
	source: 'wplace' | 'custom';
	colors: PaletteColor[];
};

export type EnabledPaletteColor = PaletteColor & { enabled: true };

export type OutputSettings = {
	width: number;
	height: number;
	lockAspect: boolean;
	fit: FitMode;
	resize: ResizeId;
	alphaMode: AlphaMode;
	alphaThreshold: number;
	matteKey: string;
	autoSizeOnUpload: boolean;
	scaleFactor: number;
	crop?: CropRect;
};

export type DitherPlacement = 'everywhere' | 'adaptive';

export type DitherSettings = {
	algorithm: DitherId;
	strength: number;
	placement: DitherPlacement;
	placementRadius: number;
	placementThreshold: number;
	placementSoftness: number;
	serpentine: boolean;
	seed: number;
	useColorSpace: boolean;
	coverage?: 'full' | 'transitions' | 'edges';
};

export type ProcessingSettings = {
	output: OutputSettings;
	dither: DitherSettings;
	colorSpace: ColorSpaceId;
};

export type SourceImageRecord = {
	blob: Blob;
	name: string;
	width: number;
	height: number;
	type: string;
	updatedAt: number;
};

export type ProcessedImage = {
	width: number;
	height: number;
	indices: Uint8Array;
	palette: EnabledPaletteColor[];
	transparentIndex: number;
	warnings: string[];
	settingsHash: string;
	updatedAt: number;
};

export type WorkerRequest = {
	id: number;
	source: ImageData;
	settings: ProcessingSettings;
	palette: EnabledPaletteColor[];
	settingsHash: string;
};

export type WorkerProgress = {
	id: number;
	type: 'progress';
	stage: string;
	progress: number;
};

export type WorkerComplete = {
	id: number;
	type: 'complete';
	image: ProcessedImage;
};

export type WorkerFailure = {
	id: number;
	type: 'error';
	message: string;
};

export type WorkerResponse = WorkerProgress | WorkerComplete | WorkerFailure;

export function clampOutputDimension(value: number): number {
	if (!Number.isFinite(value)) return 1;
	return Math.min(MAX_OUTPUT_SIDE, Math.max(1, Math.round(value)));
}

export function clampOutputSize(
	width: number,
	height: number
): { width: number; height: number; warning?: string } {
	let nextWidth = clampOutputDimension(width);
	let nextHeight = clampOutputDimension(height);
	const pixels = nextWidth * nextHeight;
	if (pixels <= MAX_OUTPUT_PIXELS) return { width: nextWidth, height: nextHeight };

	const scale = Math.sqrt(MAX_OUTPUT_PIXELS / pixels);
	nextWidth = Math.max(1, Math.floor(nextWidth * scale));
	nextHeight = Math.max(1, Math.floor(nextHeight * scale));
	return {
		width: nextWidth,
		height: nextHeight,
		warning: `Output was clamped to ${nextWidth}×${nextHeight} to stay under ${MAX_OUTPUT_PIXELS.toLocaleString()} pixels.`
	};
}

export function fitOutputSizeToBounds(width: number, height: number) {
	const safeWidth = Math.max(1, width);
	const safeHeight = Math.max(1, height);
	const sideScale = Math.min(1, MAX_OUTPUT_SIDE / safeWidth, MAX_OUTPUT_SIDE / safeHeight);
	const nextWidth = Math.max(1, Math.floor(safeWidth * sideScale));
	const nextHeight = Math.max(1, Math.floor(safeHeight * sideScale));
	return clampOutputSize(nextWidth, nextHeight);
}

export function validateSourceImageSize(width: number, height: number) {
	if (!Number.isFinite(width) || !Number.isFinite(height) || width < 1 || height < 1) {
		throw new Error('Image dimensions could not be read.');
	}
	if (width > MAX_SOURCE_SIDE || height > MAX_SOURCE_SIDE) {
		throw new Error(
			`Image is too large to decode safely. Maximum source side is ${MAX_SOURCE_SIDE.toLocaleString()} pixels.`
		);
	}
	if (width * height > MAX_SOURCE_PIXELS) {
		throw new Error(
			`Image is too large to decode safely. Maximum source size is ${MAX_SOURCE_PIXELS.toLocaleString()} pixels.`
		);
	}
}
