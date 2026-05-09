import {
	MAX_OUTPUT_PIXELS,
	MAX_OUTPUT_SIDE,
	MAX_SOURCE_PIXELS,
	MAX_SOURCE_SIDE,
	type AlphaMode,
	type ColorSpaceId,
	type DitherId,
	type DitherPlacement,
	type DitherSettings,
	type EnabledPaletteColor,
	type OutputSettings,
	type ProcessedImage,
	type ProcessingSettings,
	type ResizeId,
	type SourceImageRecord,
	type WorkerRequest,
	type WorkerResponse
} from './types';

const RESIZE_MODES = [
	'nearest',
	'bilinear',
	'lanczos3',
	'area'
] as const satisfies readonly ResizeId[];
const ALPHA_MODES = ['preserve', 'premultiplied', 'matte'] as const satisfies readonly AlphaMode[];
const DITHER_IDS = [
	'none',
	'bayer-2',
	'bayer-4',
	'bayer-8',
	'bayer-16',
	'floyd-steinberg',
	'sierra',
	'sierra-lite',
	'random'
] as const satisfies readonly DitherId[];
const PLACEMENT_MODES = ['everywhere', 'adaptive'] as const satisfies readonly DitherPlacement[];
const COLOR_SPACES = [
	'oklab',
	'srgb',
	'linear-rgb',
	'weighted-rgb',
	'weighted-rgb-601',
	'weighted-rgb-709',
	'cielab',
	'oklch'
] as const satisfies readonly ColorSpaceId[];
const COVERAGE_MODES = ['full', 'transitions', 'edges'] as const;

function isObject(value: unknown): value is Record<string, unknown> {
	return Boolean(value) && typeof value === 'object';
}

function assertFinitePositiveInteger(value: unknown, label: string, max = Number.MAX_SAFE_INTEGER) {
	if (!Number.isInteger(value) || (value as number) < 1 || (value as number) > max) {
		throw new Error(`${label} must be a positive integer.`);
	}
	return value as number;
}

function assertFiniteNumber(value: unknown, label: string) {
	if (typeof value !== 'number' || !Number.isFinite(value)) throw new Error(`${label} is invalid.`);
	return value;
}

function assertFiniteNonNegativeNumber(value: unknown, label: string) {
	const number = assertFiniteNumber(value, label);
	if (number < 0) throw new Error(`${label} cannot be negative.`);
	return number;
}

function assertFiniteTimestamp(value: unknown, label: string) {
	if (!Number.isFinite(value) || (value as number) < 0) throw new Error(`${label} is invalid.`);
	return value as number;
}

function assertString(value: unknown, label: string) {
	if (typeof value !== 'string') throw new Error(`${label} must be a string.`);
	return value;
}

function assertBoolean(value: unknown, label: string) {
	if (typeof value !== 'boolean') throw new Error(`${label} must be a boolean.`);
	return value;
}

function assertOneOf<const T extends readonly string[]>(
	value: unknown,
	allowed: T,
	label: string
): T[number] {
	if (typeof value !== 'string' || !allowed.includes(value)) {
		throw new Error(`${label} is invalid.`);
	}
	return value;
}

export function assertOutputDimensions(width: unknown, height: unknown, label = 'Image') {
	const safeWidth = assertFinitePositiveInteger(width, `${label} width`, MAX_OUTPUT_SIDE);
	const safeHeight = assertFinitePositiveInteger(height, `${label} height`, MAX_OUTPUT_SIDE);
	if (safeWidth * safeHeight > MAX_OUTPUT_PIXELS) {
		throw new Error(`${label} exceeds the maximum output pixel count.`);
	}
	return { width: safeWidth, height: safeHeight };
}

export function assertSourceDimensions(width: unknown, height: unknown, label = 'Source image') {
	const safeWidth = assertFinitePositiveInteger(width, `${label} width`, MAX_SOURCE_SIDE);
	const safeHeight = assertFinitePositiveInteger(height, `${label} height`, MAX_SOURCE_SIDE);
	if (safeWidth * safeHeight > MAX_SOURCE_PIXELS) {
		throw new Error(`${label} exceeds the maximum source pixel count.`);
	}
	return { width: safeWidth, height: safeHeight };
}

function assertCropRect(value: unknown): OutputSettings['crop'] {
	if (value === undefined) return undefined;
	if (!isObject(value)) throw new Error('Worker output crop is invalid.');
	return {
		x: assertFiniteNumber(value.x, 'Worker output crop x'),
		y: assertFiniteNumber(value.y, 'Worker output crop y'),
		width: assertFinitePositiveInteger(value.width, 'Worker output crop width', MAX_SOURCE_SIDE),
		height: assertFinitePositiveInteger(value.height, 'Worker output crop height', MAX_SOURCE_SIDE)
	};
}

function validateOutputSettings(value: unknown): OutputSettings {
	if (!isObject(value)) throw new Error('Worker output settings are invalid.');
	const { width, height } = assertOutputDimensions(value.width, value.height, 'Worker output');
	const alphaThreshold = assertFiniteNumber(value.alphaThreshold, 'Worker output alpha threshold');
	if (alphaThreshold < 0 || alphaThreshold > 255) {
		throw new Error('Worker output alpha threshold must be between 0 and 255.');
	}
	return {
		width,
		height,
		lockAspect: assertBoolean(value.lockAspect, 'Worker output aspect lock'),
		resize: assertOneOf(value.resize, RESIZE_MODES, 'Worker output resize mode'),
		alphaMode: assertOneOf(value.alphaMode, ALPHA_MODES, 'Worker output alpha mode'),
		alphaThreshold,
		matteKey: assertString(value.matteKey, 'Worker output matte key'),
		autoSizeOnUpload: assertBoolean(value.autoSizeOnUpload, 'Worker output auto size flag'),
		scaleFactor: assertFiniteNonNegativeNumber(value.scaleFactor, 'Worker output scale factor'),
		crop: assertCropRect(value.crop)
	};
}

function validateDitherSettings(value: unknown): DitherSettings {
	if (!isObject(value)) throw new Error('Worker dither settings are invalid.');
	const coverage = value.coverage;
	return {
		algorithm: assertOneOf(value.algorithm, DITHER_IDS, 'Worker dither algorithm'),
		strength: assertFiniteNonNegativeNumber(value.strength, 'Worker dither strength'),
		placement: assertOneOf(value.placement, PLACEMENT_MODES, 'Worker dither placement'),
		placementRadius: assertFiniteNonNegativeNumber(
			value.placementRadius,
			'Worker dither placement radius'
		),
		placementThreshold: assertFiniteNonNegativeNumber(
			value.placementThreshold,
			'Worker dither placement threshold'
		),
		placementSoftness: assertFiniteNonNegativeNumber(
			value.placementSoftness,
			'Worker dither placement softness'
		),
		serpentine: assertBoolean(value.serpentine, 'Worker dither serpentine flag'),
		seed: assertFiniteNumber(value.seed, 'Worker dither seed'),
		useColorSpace: assertBoolean(value.useColorSpace, 'Worker dither color-space flag'),
		coverage:
			coverage === undefined
				? undefined
				: assertOneOf(coverage, COVERAGE_MODES, 'Worker dither coverage')
	};
}

function validateProcessingSettings(value: unknown): ProcessingSettings {
	if (!isObject(value)) throw new Error('Worker request settings are invalid.');
	return {
		output: validateOutputSettings(value.output),
		dither: validateDitherSettings(value.dither),
		colorSpace: assertOneOf(value.colorSpace, COLOR_SPACES, 'Worker color space')
	};
}

export function assertPaletteForIndexedOutput(palette: unknown): EnabledPaletteColor[] {
	if (!Array.isArray(palette) || palette.length < 1 || palette.length > 256) {
		throw new Error('Indexed PNG palette must contain 1–256 colors.');
	}
	for (const [index, color] of palette.entries()) {
		if (!isObject(color)) throw new Error(`Palette color ${index + 1} is invalid.`);
		assertString(color.name, `Palette color ${index + 1} name`);
		assertString(color.key, `Palette color ${index + 1} key`);
		if (color.kind !== 'transparent') {
			if (!isObject(color.rgb)) throw new Error(`Palette color ${index + 1} needs RGB.`);
			for (const channel of ['r', 'g', 'b'] as const) {
				const value = color.rgb[channel];
				if (typeof value !== 'number' || !Number.isInteger(value) || value < 0 || value > 255) {
					throw new Error(`Palette color ${index + 1} has invalid RGB.`);
				}
			}
		}
	}
	return palette as EnabledPaletteColor[];
}

export function assertIndexBuffer(
	indices: unknown,
	width: number,
	height: number,
	paletteLength: number
): Uint8Array {
	if (!(indices instanceof Uint8Array)) throw new Error('Processed image indices are invalid.');
	const expectedLength = width * height;
	if (indices.length !== expectedLength) {
		throw new Error('Processed image index buffer does not match dimensions.');
	}
	for (const index of indices) {
		if (index >= paletteLength)
			throw new Error('Processed image references a missing palette entry.');
	}
	return indices;
}

export function validateSourceImageRecord(value: unknown): SourceImageRecord {
	if (!isObject(value)) throw new Error('Saved source image record is invalid.');
	if (!(value.blob instanceof Blob)) throw new Error('Saved source image blob is invalid.');
	const { width, height } = assertSourceDimensions(value.width, value.height, 'Saved source image');
	return {
		blob: value.blob,
		name: assertString(value.name, 'Saved source image name'),
		width,
		height,
		type: assertString(value.type, 'Saved source image type'),
		updatedAt: assertFiniteTimestamp(value.updatedAt, 'Saved source image timestamp')
	};
}

export function validateProcessedImage(value: unknown): ProcessedImage {
	if (!isObject(value)) throw new Error('Saved processed image record is invalid.');
	const { width, height } = assertOutputDimensions(value.width, value.height, 'Processed image');
	const palette = assertPaletteForIndexedOutput(value.palette);
	const indices = assertIndexBuffer(value.indices, width, height, palette.length);
	const transparentIndex = value.transparentIndex;
	if (
		!Number.isInteger(transparentIndex) ||
		(transparentIndex as number) < -1 ||
		(transparentIndex as number) >= palette.length
	) {
		throw new Error('Processed image transparent index is invalid.');
	}
	const warnings = Array.isArray(value.warnings)
		? value.warnings.filter((warning): warning is string => typeof warning === 'string')
		: [];
	return {
		width,
		height,
		indices,
		palette,
		transparentIndex: transparentIndex as number,
		warnings,
		settingsHash: assertString(value.settingsHash, 'Processed image settings hash'),
		updatedAt: assertFiniteTimestamp(value.updatedAt, 'Processed image timestamp')
	};
}

export function validateWorkerRequest(value: unknown): WorkerRequest {
	if (!isObject(value)) throw new Error('Worker received an invalid processing request.');
	if (!Number.isInteger(value.id)) throw new Error('Worker request id is invalid.');
	const id = value.id as number;
	if (!(value.source instanceof ImageData))
		throw new Error('Worker request source image is invalid.');
	const palette = assertPaletteForIndexedOutput(value.palette);
	return {
		id,
		source: value.source,
		settings: validateProcessingSettings(value.settings),
		palette,
		settingsHash: assertString(value.settingsHash, 'Worker settings hash')
	};
}

export function validateWorkerResponse(value: unknown): WorkerResponse {
	if (!isObject(value)) throw new Error('Worker response is invalid.');
	if (!Number.isInteger(value.id)) throw new Error('Worker response id is invalid.');
	const id = value.id as number;
	if (value.type === 'progress') {
		if (typeof value.stage !== 'string' || !Number.isFinite(value.progress)) {
			throw new Error('Worker progress response is invalid.');
		}
		return value as WorkerResponse;
	}
	if (value.type === 'error') {
		if (typeof value.message !== 'string') throw new Error('Worker error response is invalid.');
		return value as WorkerResponse;
	}
	if (value.type === 'complete') {
		return { id, type: 'complete', image: validateProcessedImage(value.image) };
	}
	throw new Error('Worker response type is invalid.');
}

export function validatePngExportImage(image: ProcessedImage): ProcessedImage {
	return validateProcessedImage(image);
}
