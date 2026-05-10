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
import type { ProcessingMetricsSample } from './metrics';

const RESIZE_MODES = [
	'nearest',
	'bilinear',
	'lanczos2',
	'lanczos2-scale-aware',
	'lanczos3',
	'lanczos3-scale-aware',
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

function assertInteger(value: unknown, label: string) {
	if (!Number.isInteger(value)) throw new Error(`${label} is invalid.`);
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
	if (value.type === 'cancel') return { id, type: 'cancel' };
	if (value.type === 'load-source') {
		if (!(value.source instanceof ImageData))
			throw new Error('Worker request source image is invalid.');
		return {
			id,
			type: 'load-source',
			sourceId: assertString(value.sourceId, 'Worker source id'),
			source: value.source
		};
	}
	if (value.type === 'process') {
		const palette = assertPaletteForIndexedOutput(value.palette);
		return {
			id,
			type: 'process',
			sourceId: assertString(value.sourceId, 'Worker source id'),
			settings: validateProcessingSettings(value.settings),
			palette,
			settingsHash: assertString(value.settingsHash, 'Worker settings hash')
		};
	}
	throw new Error('Worker request type is invalid.');
}

function validateTiming(value: unknown) {
	if (!isObject(value)) throw new Error('Worker metrics timing is invalid.');
	const replayed = value.replayed;
	return {
		name: assertString(value.name, 'Worker metrics timing name'),
		ms: assertFiniteNonNegativeNumber(value.ms, 'Worker metrics timing duration'),
		replayed:
			replayed === undefined ? undefined : assertBoolean(replayed, 'Worker metrics replay flag')
	};
}

function validateCacheSnapshot(value: unknown) {
	if (!isObject(value)) throw new Error('Worker metrics cache snapshot is invalid.');
	return {
		sourceLoaded: assertBoolean(value.sourceLoaded, 'Worker metrics source loaded'),
		sourceBytes: assertFiniteNonNegativeNumber(value.sourceBytes, 'Worker metrics source bytes'),
		branchCount: assertFiniteNonNegativeNumber(value.branchCount, 'Worker metrics branch count'),
		branchBytes: assertFiniteNonNegativeNumber(value.branchBytes, 'Worker metrics branch bytes'),
		branchMaxBytes: assertFiniteNonNegativeNumber(
			value.branchMaxBytes,
			'Worker metrics branch max bytes'
		),
		resizedHits: assertFiniteNonNegativeNumber(value.resizedHits, 'Worker metrics resized hits'),
		resizedMisses: assertFiniteNonNegativeNumber(
			value.resizedMisses,
			'Worker metrics resized misses'
		),
		resizedSets: assertFiniteNonNegativeNumber(value.resizedSets, 'Worker metrics resized sets'),
		resizedSkips: assertFiniteNonNegativeNumber(value.resizedSkips, 'Worker metrics resized skips'),
		resizedEvictions: assertFiniteNonNegativeNumber(
			value.resizedEvictions,
			'Worker metrics resized evictions'
		),
		derivedHits: assertFiniteNonNegativeNumber(value.derivedHits, 'Worker metrics derived hits'),
		derivedMisses: assertFiniteNonNegativeNumber(
			value.derivedMisses,
			'Worker metrics derived misses'
		),
		derivedSets: assertFiniteNonNegativeNumber(value.derivedSets, 'Worker metrics derived sets'),
		derivedSkips: assertFiniteNonNegativeNumber(value.derivedSkips, 'Worker metrics derived skips'),
		derivedEvictions: assertFiniteNonNegativeNumber(
			value.derivedEvictions,
			'Worker metrics derived evictions'
		),
		paletteVectorEntries: assertFiniteNonNegativeNumber(
			value.paletteVectorEntries,
			'Worker metrics palette entries'
		),
		paletteVectorMaxEntries: assertFiniteNonNegativeNumber(
			value.paletteVectorMaxEntries,
			'Worker metrics palette max entries'
		),
		paletteVectorHits: assertFiniteNonNegativeNumber(
			value.paletteVectorHits,
			'Worker metrics palette hits'
		),
		paletteVectorMisses: assertFiniteNonNegativeNumber(
			value.paletteVectorMisses,
			'Worker metrics palette misses'
		),
		paletteVectorSets: assertFiniteNonNegativeNumber(
			value.paletteVectorSets,
			'Worker metrics palette sets'
		),
		paletteVectorEvictions: assertFiniteNonNegativeNumber(
			value.paletteVectorEvictions,
			'Worker metrics palette evictions'
		)
	};
}

function validateMemoryShape(value: unknown) {
	if (!isObject(value)) throw new Error('Worker metrics memory shape is invalid.');
	return {
		sourceBytes: assertFiniteNonNegativeNumber(value.sourceBytes, 'Worker metrics source memory'),
		resizedBytes: assertFiniteNonNegativeNumber(
			value.resizedBytes,
			'Worker metrics resized memory'
		),
		indexBytes: assertFiniteNonNegativeNumber(value.indexBytes, 'Worker metrics index memory'),
		vectorBytes: assertFiniteNonNegativeNumber(value.vectorBytes, 'Worker metrics vector memory'),
		ditherWorkBytes: assertFiniteNonNegativeNumber(
			value.ditherWorkBytes,
			'Worker metrics dither memory'
		),
		branchCacheBytes: assertFiniteNonNegativeNumber(
			value.branchCacheBytes,
			'Worker metrics branch memory'
		),
		branchCacheMaxBytes: assertFiniteNonNegativeNumber(
			value.branchCacheMaxBytes,
			'Worker metrics branch max memory'
		)
	};
}

function validateProcessingMetrics(value: unknown): ProcessingMetricsSample | undefined {
	if (value === undefined) return undefined;
	if (!isObject(value)) throw new Error('Worker metrics are invalid.');
	const cache = isObject(value.cache) ? value.cache : undefined;
	if (!cache) throw new Error('Worker metrics cache is invalid.');
	const timings = Array.isArray(value.timings) ? value.timings.map(validateTiming) : [];
	const warnings = Array.isArray(value.warnings)
		? value.warnings.filter((warning): warning is string => typeof warning === 'string')
		: [];
	return {
		id: assertInteger(value.id, 'Worker metrics id'),
		settingsHash: assertString(value.settingsHash, 'Worker metrics settings hash'),
		sourceId: assertString(value.sourceId, 'Worker metrics source id'),
		scopeKey: assertString(value.scopeKey, 'Worker metrics scope key'),
		startedAt: assertFiniteTimestamp(value.startedAt, 'Worker metrics start'),
		completedAt: assertFiniteTimestamp(value.completedAt, 'Worker metrics completion'),
		totalMs: assertFiniteNonNegativeNumber(value.totalMs, 'Worker metrics total'),
		timings,
		cache: {
			delta: validateCacheSnapshot(cache.delta),
			lifetime: validateCacheSnapshot(cache.lifetime)
		},
		memory: validateMemoryShape(value.memory),
		outputPixels: assertFiniteNonNegativeNumber(value.outputPixels, 'Worker metrics output pixels'),
		colorSpace: assertOneOf(value.colorSpace, COLOR_SPACES, 'Worker metrics color space'),
		dither: assertOneOf(value.dither, DITHER_IDS, 'Worker metrics dither'),
		resize: assertOneOf(value.resize, RESIZE_MODES, 'Worker metrics resize'),
		warnings
	};
}

function safeProcessingMetrics(value: unknown) {
	try {
		return validateProcessingMetrics(value);
	} catch {
		return undefined;
	}
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
	if (value.type === 'source-loaded') {
		return {
			id,
			type: 'source-loaded',
			sourceId: assertString(value.sourceId, 'Worker source id')
		};
	}
	if (value.type === 'complete') {
		return {
			id,
			type: 'complete',
			image: validateProcessedImage(value.image),
			metrics: safeProcessingMetrics(value.metrics)
		};
	}
	throw new Error('Worker response type is invalid.');
}

export function validatePngExportImage(image: ProcessedImage): ProcessedImage {
	return validateProcessedImage(image);
}
