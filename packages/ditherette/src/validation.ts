import { DitheretteError } from './errors.js';
import type { ErrorCode } from './errors.js';
import type { InitInput, InitOptions } from './types.js';

const maxPixels = 67_108_864;
const anchors = [
	'top-left',
	'top',
	'top-right',
	'left',
	'center',
	'right',
	'bottom-left',
	'bottom',
	'bottom-right'
];
// Private ABI order. Existing algorithm tags remain stable as implementations are added.
const algorithms = ['nearest', 'area', 'bilinear', 'bicubic', 'lanczos2', 'lanczos3'];
const typedArrayPrototype = Object.getPrototypeOf(Uint8Array.prototype);
const arrayTag = Object.getOwnPropertyDescriptor(typedArrayPrototype, Symbol.toStringTag)!.get!;
const arrayBuffer = Object.getOwnPropertyDescriptor(typedArrayPrototype, 'buffer')!.get!;
const arrayOffset = Object.getOwnPropertyDescriptor(typedArrayPrototype, 'byteOffset')!.get!;
const arrayLength = Object.getOwnPropertyDescriptor(typedArrayPrototype, 'byteLength')!.get!;

export function object(
	value: unknown,
	keys: readonly string[],
	code: ErrorCode,
	path: string
): Record<string, unknown> {
	if (typeof value !== 'object' || value === null || Array.isArray(value)) {
		throw new DitheretteError(code, path, 'Expected a settings object.');
	}
	for (const key of Reflect.ownKeys(value)) {
		if (typeof key !== 'string' || !keys.includes(key)) {
			throw new DitheretteError(
				code,
				typeof key === 'string' ? (path ? `${path}.${key}` : key) : path,
				'Unknown field.'
			);
		}
	}
	return value as Record<string, unknown>;
}

export function field(value: Record<string, unknown>, key: string): unknown {
	return Object.hasOwn(value, key) ? value[key] : undefined;
}

function integer(value: unknown, maximum: number, code: ErrorCode, path: string): number {
	if (typeof value !== 'number' || !Number.isInteger(value) || value < 1 || value > maximum) {
		throw new DitheretteError(code, path, 'Expected an integer within the supported range.');
	}
	return value;
}

export function dimensions(
	value: Record<string, unknown>,
	maximum: number,
	code: ErrorCode,
	path: string
) {
	const width = integer(field(value, 'width'), maximum, code, `${path}.width`);
	const height = integer(field(value, 'height'), maximum, code, `${path}.height`);
	if (width * height > maxPixels)
		throw new DitheretteError(code, path, 'Image exceeds the maximum pixel count.');
	return { width, height };
}

/** Validate raw options before loading artifacts or allocating Wasm memory. */
export function validateOptions(
	value: unknown
): Required<Pick<InitOptions, 'threads' | 'memoryLimitBytes'>> & { wasm?: InitInput } {
	try {
		const options = object(
			value === undefined ? {} : value,
			['threads', 'memoryLimitBytes', 'wasm'],
			'invalid-settings',
			''
		);
		const rawThreads = field(options, 'threads');
		const threads = rawThreads === undefined ? 'disabled' : rawThreads;
		if (threads !== 'disabled' && threads !== 'preferred' && threads !== 'required') {
			throw new DitheretteError('invalid-settings', 'threads', 'Unknown thread policy.');
		}
		const rawLimit = field(options, 'memoryLimitBytes');
		const memoryLimitBytes = integer(
			rawLimit === undefined ? 1_610_612_736 : rawLimit,
			2_147_483_648,
			'invalid-settings',
			'memoryLimitBytes'
		);
		const wasm = field(options, 'wasm');
		if (wasm !== undefined && !isInitInput(wasm))
			throw new DitheretteError(
				'invalid-settings',
				'wasm',
				'Unsupported Wasm initialization input.'
			);
		return { threads, memoryLimitBytes, wasm };
	} catch (error) {
		if (error instanceof DitheretteError) throw error;
		throw new DitheretteError('invalid-settings', '', 'Initialization options could not be read.');
	}
}

function isInitInput(value: unknown): value is InitInput {
	return (
		typeof value === 'string' ||
		(typeof URL !== 'undefined' && value instanceof URL) ||
		(typeof Request !== 'undefined' && value instanceof Request) ||
		(typeof Response !== 'undefined' && value instanceof Response) ||
		value instanceof ArrayBuffer ||
		ArrayBuffer.isView(value) ||
		(typeof WebAssembly !== 'undefined' && value instanceof WebAssembly.Module)
	);
}

export function rgbaBytes(value: unknown, expectedBytes: number): Uint8Array {
	if (
		!ArrayBuffer.isView(value) ||
		arrayTag.call(value) !== 'Uint8Array' ||
		arrayLength.call(value) !== expectedBytes
	) {
		throw new DitheretteError(
			'invalid-image',
			'source.data',
			'Expected packed RGBA8 Uint8Array bytes matching the dimensions.'
		);
	}
	try {
		// A plain borrowed view ignores caller-overridden length/getters without copying source bytes.
		return new Uint8Array(arrayBuffer.call(value), arrayOffset.call(value), expectedBytes);
	} catch (error) {
		const code = error instanceof RangeError ? 'wasm-memory-unavailable' : 'invalid-image';
		throw new DitheretteError(code, 'source.data', 'RGBA8 source bytes are unavailable.');
	}
}

/** Read each caller property once and validate canonical tags before entering the private binding. */
export function validateResize(value: unknown) {
	try {
		const request = object(
			value,
			['version', 'source', 'output', 'onProgress'],
			'invalid-request',
			'request'
		);
		if (field(request, 'version') !== 1)
			throw new DitheretteError('invalid-request', 'version', 'Unsupported recipe version.');
		const onProgress = field(request, 'onProgress');
		if (onProgress !== undefined) {
			if (typeof onProgress !== 'function')
				throw new DitheretteError(
					'invalid-settings',
					'onProgress',
					'Expected a progress callback.'
				);
			throw new DitheretteError(
				'unsupported-operation',
				'onProgress',
				'Progress callbacks are not implemented in this package checkpoint.'
			);
		}
		const output = object(
			field(request, 'output'),
			['width', 'height', 'resize'],
			'invalid-settings',
			'output'
		);
		const resize = object(
			field(output, 'resize'),
			['algorithm', 'anchor', 'support'],
			'invalid-settings',
			'output.resize'
		);
		const algorithm = field(resize, 'algorithm');
		const algorithmIndex = typeof algorithm === 'string' ? algorithms.indexOf(algorithm) : -1;
		if (algorithmIndex < 0) {
			const code = algorithm === 'trilinear' ? 'unsupported-operation' : 'invalid-settings';
			throw new DitheretteError(
				code,
				'output.resize.algorithm',
				'This resize algorithm is not implemented in this package checkpoint.'
			);
		}
		const convolution = algorithmIndex >= 3;
		if (!convolution && Object.hasOwn(resize, 'support'))
			throw new DitheretteError(
				'invalid-settings',
				'output.resize.support',
				'This resize algorithm has no support policy.'
			);
		const rawSupport = convolution ? field(resize, 'support') : 'fixed';
		if (rawSupport !== 'fixed' && rawSupport !== 'scale-aware')
			throw new DitheretteError(
				'invalid-settings',
				'output.resize.support',
				'Expected fixed or scale-aware support.'
			);
		if (algorithm === 'area' && Object.hasOwn(resize, 'anchor'))
			throw new DitheretteError(
				'invalid-settings',
				'output.resize.anchor',
				'Area resize has no anchor.'
			);
		const rawAnchor = field(resize, 'anchor');
		const anchor =
			algorithm === 'area' ? 0 : typeof rawAnchor === 'string' ? anchors.indexOf(rawAnchor) : -1;
		if (anchor < 0)
			throw new DitheretteError(
				'invalid-settings',
				'output.resize.anchor',
				'Unknown resize anchor.'
			);
		const source = object(
			field(request, 'source'),
			['width', 'height', 'data'],
			'invalid-image',
			'source'
		);
		const sourceSize = dimensions(source, 32_768, 'invalid-image', 'source');
		const rawData = field(source, 'data');
		const outputSize = dimensions(output, 16_384, 'invalid-settings', 'output');
		const data = rgbaBytes(rawData, sourceSize.width * sourceSize.height * 4);
		return {
			data,
			sourceWidth: sourceSize.width,
			sourceHeight: sourceSize.height,
			outputWidth: outputSize.width,
			outputHeight: outputSize.height,
			algorithm: algorithmIndex,
			anchor,
			support: rawSupport === 'fixed' ? 0 : 1
		};
	} catch (error) {
		if (error instanceof DitheretteError) throw error;
		throw new DitheretteError(
			'invalid-request',
			'request',
			'Request properties could not be read.'
		);
	}
}

const matchingModes = [
	'srgb-euclidean',
	'linear-rgb-euclidean',
	'oklab-euclidean',
	'cielab-euclidean',
	'ycbcr-euclidean',
	'srgb-compuphase',
	'srgb-rec601',
	'srgb-rec709',
	'oklch-euclidean',
	'oklch-circular-hue',
	'oklch-hue-arc',
	'cielab-ciede2000',
	'cielch-euclidean',
	'cielch-circular-hue',
	'cielch-hue-arc'
];

function rgbCode(value: unknown, code: ErrorCode, path: string): number {
	if (
		!Array.isArray(value) ||
		value.length !== 3 ||
		Reflect.ownKeys(value).some(
			(key) => !['0', '1', '2', 'length'].includes(String(key)) || typeof key !== 'string'
		)
	)
		throw new DitheretteError(code, path, 'Expected exactly three RGB bytes.');
	let packed = 0;
	for (let index = 0; index < 3; index++) {
		const channel: unknown = Object.hasOwn(value, index) ? value[index] : undefined;
		if (typeof channel !== 'number' || !Number.isInteger(channel) || channel < 0 || channel > 255)
			throw new DitheretteError(code, `${path}.${index}`, 'Expected an RGB byte.');
		packed = packed * 256 + channel;
	}
	return packed;
}

/** Normalize the frozen quantize contract once, retaining only its bounded palette prefix. */
export function validateQuantize(value: unknown) {
	try {
		const request = object(
			value,
			['version', 'source', 'palette', 'alpha', 'matching', 'onProgress'],
			'invalid-request',
			'request'
		);
		if (field(request, 'version') !== 1)
			throw new DitheretteError('invalid-request', 'version', 'Unsupported recipe version.');
		const onProgress = field(request, 'onProgress');
		if (onProgress !== undefined)
			throw new DitheretteError(
				typeof onProgress === 'function' ? 'unsupported-operation' : 'invalid-settings',
				'onProgress',
				'Progress callbacks are not implemented in this package checkpoint.'
			);
		const rawMatching = field(request, 'matching');
		const matching = typeof rawMatching === 'string' ? matchingModes.indexOf(rawMatching) : -1;
		if (matching < 0)
			throw new DitheretteError('invalid-settings', 'matching', 'Unsupported matching recipe.');
		const alpha = object(
			field(request, 'alpha'),
			['mode', 'threshold', 'rgb'],
			'invalid-settings',
			'alpha'
		);
		const mode = field(alpha, 'mode');
		let alphaMode: number;
		let threshold = 0;
		let matte = 0;
		if (mode === 'preserve') {
			if (Object.hasOwn(alpha, 'rgb'))
				throw new DitheretteError('invalid-settings', 'alpha.rgb', 'Preserve alpha has no matte.');
			const value = field(alpha, 'threshold');
			if (typeof value !== 'number' || !Number.isFinite(value) || value < 0 || value > 255)
				throw new DitheretteError(
					'invalid-settings',
					'alpha.threshold',
					'Expected a finite byte threshold.'
				);
			threshold = value;
			alphaMode = 0;
		} else if (mode === 'premultiplied' || mode === 'matte') {
			if (Object.hasOwn(alpha, 'threshold'))
				throw new DitheretteError(
					'invalid-settings',
					'alpha.threshold',
					'This alpha mode has no threshold.'
				);
			if (mode === 'premultiplied') {
				if (Object.hasOwn(alpha, 'rgb'))
					throw new DitheretteError(
						'invalid-settings',
						'alpha.rgb',
						'Premultiplied alpha has no matte.'
					);
				alphaMode = 1;
			} else {
				matte = rgbCode(field(alpha, 'rgb'), 'invalid-settings', 'alpha.rgb');
				alphaMode = 2;
			}
		} else throw new DitheretteError('invalid-settings', 'alpha.mode', 'Unknown alpha mode.');
		const rawPalette = field(request, 'palette');
		if (!Array.isArray(rawPalette))
			throw new DitheretteError('invalid-palette', 'palette', 'Expected a nonempty palette array.');
		const count = rawPalette.length;
		if (!Number.isInteger(count) || count < 1 || count > 4_294_967_295)
			throw new DitheretteError('invalid-palette', 'palette', 'Expected a nonempty palette array.');
		const palette: number[] = [];
		for (let index = 0; index < count; index++) {
			const path = `palette.${index}`;
			const entry = object(
				Object.hasOwn(rawPalette, index) ? rawPalette[index] : undefined,
				['kind', 'rgb'],
				'invalid-palette',
				path
			);
			const kind = field(entry, 'kind');
			let code: number;
			if (kind === 'transparent') {
				if (Object.hasOwn(entry, 'rgb'))
					throw new DitheretteError(
						'invalid-palette',
						`${path}.rgb`,
						'Transparent has no RGB fields.'
					);
				code = 16_777_216;
			} else if (kind === 'color')
				code = rgbCode(field(entry, 'rgb'), 'invalid-palette', `${path}.rgb`);
			else throw new DitheretteError('invalid-palette', `${path}.kind`, 'Unknown palette entry.');
			if (index < 256) palette.push(code);
		}
		if (count > 256) palette.push(16_777_216); // Tail marker is never retained as a normalized palette entry.
		const source = object(
			field(request, 'source'),
			['width', 'height', 'data'],
			'invalid-image',
			'source'
		);
		const size = dimensions(source, 32_768, 'invalid-image', 'source');
		return {
			data: rgbaBytes(field(source, 'data'), size.width * size.height * 4),
			sourceWidth: size.width,
			sourceHeight: size.height,
			palette,
			matching,
			alphaMode,
			threshold,
			matte
		};
	} catch (error) {
		if (error instanceof DitheretteError) throw error;
		throw new DitheretteError(
			error instanceof RangeError ? 'wasm-memory-unavailable' : 'invalid-request',
			'request',
			'Quantize request could not be normalized.'
		);
	}
}
