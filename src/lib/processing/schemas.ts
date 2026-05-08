import {
	MAX_OUTPUT_PIXELS,
	MAX_OUTPUT_SIDE,
	MAX_SOURCE_PIXELS,
	MAX_SOURCE_SIDE,
	type EnabledPaletteColor,
	type ProcessedImage,
	type SourceImageRecord,
	type WorkerRequest,
	type WorkerResponse
} from './types';

function isObject(value: unknown): value is Record<string, unknown> {
	return Boolean(value) && typeof value === 'object';
}

function assertFinitePositiveInteger(value: unknown, label: string, max = Number.MAX_SAFE_INTEGER) {
	if (!Number.isInteger(value) || (value as number) < 1 || (value as number) > max) {
		throw new Error(`${label} must be a positive integer.`);
	}
	return value as number;
}

function assertFiniteTimestamp(value: unknown, label: string) {
	if (!Number.isFinite(value) || (value as number) < 0) throw new Error(`${label} is invalid.`);
	return value as number;
}

function assertString(value: unknown, label: string) {
	if (typeof value !== 'string') throw new Error(`${label} must be a string.`);
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
	if (!isObject(value.settings)) throw new Error('Worker request settings are invalid.');
	if (!isObject(value.settings.output)) throw new Error('Worker output settings are invalid.');
	if (!isObject(value.settings.dither)) throw new Error('Worker dither settings are invalid.');
	const palette = assertPaletteForIndexedOutput(value.palette);
	return {
		id,
		source: value.source,
		settings: value.settings as WorkerRequest['settings'],
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
