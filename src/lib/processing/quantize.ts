import { bayerSizeForAlgorithm, normalizedBayerMatrix } from './bayer';
import { clampByte, createPaletteMatcher, srgbByteToLinear, vectorForRgb } from './color';
import { compositedRgb } from './compositing';
import type {
	AlphaMode,
	ColorSpaceId,
	DitherId,
	EnabledPaletteColor,
	ProcessingSettings,
	Rgb
} from './types';

const COLOR_SPACE_THRESHOLD_SCALE = 0.25;
const RGB_DITHER_NOISE_SCALE = 96;
const REF_X = 0.95047;
const REF_Y = 1;
const REF_Z = 1.08883;
const WEIGHTED_RGB_601_R = Math.sqrt(0.299);
const WEIGHTED_RGB_601_G = Math.sqrt(0.587);
const WEIGHTED_RGB_601_B = Math.sqrt(0.114);
const WEIGHTED_RGB_709_R = Math.sqrt(0.2126);
const WEIGHTED_RGB_709_G = Math.sqrt(0.7152);
const WEIGHTED_RGB_709_B = Math.sqrt(0.0722);

type ColorVector = ReturnType<typeof vectorForRgb>;

type PaletteVector = { index: number; vector: ColorVector };

export type PaletteVectorSpace = {
	colors: PaletteVector[];
	ranges: ColorVector;
};

export type QuantizeCounterName =
	| 'nearest rgb'
	| 'nearest vector'
	| 'rgb memo hit'
	| 'rgb memo miss'
	| 'rgb memo set'
	| 'rgb memo eviction'
	| 'vector memo hit'
	| 'vector memo miss'
	| 'vector memo set'
	| 'vector memo eviction';

export type QuantizeTimingName =
	| 'palette prepare'
	| 'matcher build'
	| 'color space convert palette cache lookup'
	| 'color space convert palette'
	| 'color space convert palette cache write'
	| 'color space convert composited image cache lookup'
	| 'color space convert composited image'
	| 'color space convert composited image cache write'
	| 'color space convert source image cache lookup'
	| 'color space convert source image'
	| 'color space convert source image cache write'
	| 'quantize direct dither+match loop'
	| 'quantize vector diffusion work init'
	| 'quantize vector diffusion dither+match loop'
	| 'quantize rgb diffusion work init'
	| 'quantize rgb diffusion dither+match loop';

export type QuantizeDiagnosticsSink = {
	recordTiming?(name: QuantizeTimingName, ms: number): void;
	recordCount?(name: QuantizeCounterName, amount?: number): void;
};

type PaletteVectorMatcher = {
	nearest(v0: number, v1: number, v2: number): PaletteVector | undefined;
	nearestIndex(v0: number, v1: number, v2: number): number;
	nearestOrdinal(v0: number, v1: number, v2: number): number;
	indexForOrdinal(ordinal: number): number;
	vector0ForOrdinal(ordinal: number): number;
	vector1ForOrdinal(ordinal: number): number;
	vector2ForOrdinal(ordinal: number): number;
	flushCounts(): void;
};

type ThresholdByteVectorMatcher = {
	nearestIndexRgb(r: number, g: number, b: number, thresholdIndex: number): number;
	flushCounts(): void;
};

export type ColorVectorImage = {
	width: number;
	height: number;
	colorSpace: ColorSpaceId;
	v0: Float32Array;
	v1: Float32Array;
	v2: Float32Array;
};

export type QuantizeCaches = QuantizeDiagnosticsSink & {
	getPaletteVectorSpace?(key: string): PaletteVectorSpace | undefined;
	setPaletteVectorSpace?(key: string, value: PaletteVectorSpace): void;
	colorVectorImageScope?: string;
	getColorVectorImage?(key: string): ColorVectorImage | undefined;
	canStoreColorVectorImage?(key: string, bytes: number): boolean;
	setColorVectorImage?(key: string, value: ColorVectorImage, bytes: number): void;
};

const MIN_THRESHOLD_VECTOR_CACHE_BITS = 18;
const MAX_THRESHOLD_VECTOR_CACHE_BITS = 22;

const ERROR_KERNELS = {
	'floyd-steinberg': [
		[1, 0, 7 / 16],
		[-1, 1, 3 / 16],
		[0, 1, 5 / 16],
		[1, 1, 1 / 16]
	],
	sierra: [
		[1, 0, 5 / 32],
		[2, 0, 3 / 32],
		[-2, 1, 2 / 32],
		[-1, 1, 4 / 32],
		[0, 1, 5 / 32],
		[1, 1, 4 / 32],
		[2, 1, 2 / 32],
		[-1, 2, 2 / 32],
		[0, 2, 3 / 32],
		[1, 2, 2 / 32]
	],
	'sierra-lite': [
		[1, 0, 2 / 4],
		[-1, 1, 1 / 4],
		[0, 1, 1 / 4]
	]
} as const satisfies Partial<Record<DitherId, readonly (readonly [number, number, number])[]>>;

type ErrorDiffusionAlgorithm = keyof typeof ERROR_KERNELS;

function errorKernelForAlgorithm(algorithm: DitherId) {
	return Object.hasOwn(ERROR_KERNELS, algorithm)
		? ERROR_KERNELS[algorithm as ErrorDiffusionAlgorithm]
		: undefined;
}

function supportsVectorDither(settings: ProcessingSettings) {
	return settings.dither.useColorSpace && settings.colorSpace !== 'weighted-rgb';
}

function supportsCachedVectorMatching(colorSpace: ColorSpaceId) {
	return (
		colorSpace === 'linear-rgb' ||
		colorSpace === 'oklab' ||
		colorSpace === 'cielab' ||
		colorSpace === 'oklch'
	);
}

function usesAdaptivePlacement(settings: ProcessingSettings) {
	const placement =
		settings.dither.placement ?? (settings.dither.coverage === 'full' ? 'everywhere' : 'adaptive');
	return placement !== 'everywhere';
}

function usesByteRgbMatcher(settings: ProcessingSettings) {
	const hasDither = settings.dither.algorithm !== 'none' && settings.dither.strength > 0;
	if (!hasDither) return true;
	return !supportsVectorDither(settings);
}

export type QuantizeResult = {
	indices: Uint8Array;
	palette: EnabledPaletteColor[];
	transparentIndex: number;
	warnings: string[];
};

function transparentIndex(palette: EnabledPaletteColor[]) {
	return palette.findIndex((color) => color.kind === 'transparent');
}

function paletteColorKey(color: EnabledPaletteColor) {
	const rgb = color.rgb ? `${color.rgb.r},${color.rgb.g},${color.rgb.b}` : 'transparent';
	return `${color.key}:${color.kind}:${rgb}`;
}

function paletteVectorCacheKey(palette: EnabledPaletteColor[], colorSpace: ColorSpaceId) {
	return `${colorSpace}|${palette.map(paletteColorKey).join(';')}`;
}

function colorVectorImageBytes(width: number, height: number) {
	return width * height * 3 * Float32Array.BYTES_PER_ELEMENT;
}

function labPivot(value: number) {
	return value > 0.008856 ? Math.cbrt(value) : 7.787 * value + 16 / 116;
}

function compositedVectorCacheKey(colorSpace: ColorSpaceId, alphaMode: AlphaMode, matte: Rgb) {
	return `composited|${colorSpace}|alpha:${alphaMode}|matte:${matte.r},${matte.g},${matte.b}`;
}

function sourceVectorCacheKey(colorSpace: ColorSpaceId) {
	return `source|${colorSpace}`;
}

function darkestVisible(colors: EnabledPaletteColor[]): EnabledPaletteColor | undefined {
	return colors.reduce<EnabledPaletteColor | undefined>((darkest, color) => {
		if (!color.rgb) return darkest;
		if (!darkest?.rgb) return color;
		const sum = color.rgb.r + color.rgb.g + color.rgb.b;
		const darkestSum = darkest.rgb.r + darkest.rgb.g + darkest.rgb.b;
		return sum < darkestSum ? color : darkest;
	}, undefined);
}

function paletteVectorSpace(
	matcher: ReturnType<typeof createPaletteMatcher>,
	settings: ProcessingSettings,
	cacheKey?: string,
	caches?: QuantizeCaches
): PaletteVectorSpace {
	const cacheLookupStart = performance.now();
	const cached = cacheKey ? caches?.getPaletteVectorSpace?.(cacheKey) : undefined;
	recordTiming(caches, 'color space convert palette cache lookup', cacheLookupStart);
	if (cached) return cached;

	const convertStart = performance.now();
	const colors: PaletteVector[] = [];
	let min: ColorVector = [Infinity, Infinity, Infinity];
	let max: ColorVector = [-Infinity, -Infinity, -Infinity];
	for (let index = 0; index < matcher.colors.length; index++) {
		const color = matcher.colors[index]!;
		if (!color.rgb || color.kind === 'transparent') continue;
		const vector = vectorForRgb(color.rgb.r, color.rgb.g, color.rgb.b, settings.colorSpace);
		colors.push({ index, vector });
		min = [Math.min(min[0], vector[0]), Math.min(min[1], vector[1]), Math.min(min[2], vector[2])];
		max = [Math.max(max[0], vector[0]), Math.max(max[1], vector[1]), Math.max(max[2], vector[2])];
	}
	const space = {
		colors,
		ranges: [
			Math.max(max[0] - min[0], Number.EPSILON),
			Math.max(max[1] - min[1], Number.EPSILON),
			Math.max(max[2] - min[2], Number.EPSILON)
		]
	} satisfies PaletteVectorSpace;
	recordTiming(caches, 'color space convert palette', convertStart);
	if (cacheKey) {
		const cacheWriteStart = performance.now();
		caches?.setPaletteVectorSpace?.(cacheKey, space);
		recordTiming(caches, 'color space convert palette cache write', cacheWriteStart);
	}
	return space;
}

function recordTiming(caches: QuantizeCaches | undefined, name: QuantizeTimingName, start: number) {
	caches?.recordTiming?.(name, performance.now() - start);
}

function vectorDistance(left: ColorVector, right: ColorVector, settings: ProcessingSettings) {
	return vectorDistanceValues(left[0], left[1], left[2], right, settings);
}

function vectorDistanceValues(
	left0: number,
	left1: number,
	left2: number,
	right: ColorVector,
	settings: ProcessingSettings
) {
	const dx = left0 - right[0];
	const dy = left1 - right[1];
	if (settings.colorSpace === 'oklch') {
		let hue = Math.abs(left2 - right[2]);
		if (hue > Math.PI) hue = Math.PI * 2 - hue;
		const dh = Math.min(left1, right[1]) * hue;
		return dx * dx + dy * dy + dh * dh;
	}
	const dz = left2 - right[2];
	return dx * dx + dy * dy + dz * dz;
}

function createPaletteVectorMatcher(
	space: PaletteVectorSpace,
	settings: ProcessingSettings,
	caches?: QuantizeCaches
): PaletteVectorMatcher {
	const count = space.colors.length;
	const paletteIndices = new Int16Array(count);
	const paletteV0 = new Float64Array(count);
	const paletteV1 = new Float64Array(count);
	const paletteV2 = new Float64Array(count);
	for (let ordinal = 0; ordinal < count; ordinal++) {
		const color = space.colors[ordinal]!;
		paletteIndices[ordinal] = color.index;
		paletteV0[ordinal] = color.vector[0];
		paletteV1[ordinal] = color.vector[1];
		paletteV2[ordinal] = color.vector[2];
	}

	let lastV0 = Number.NaN;
	let lastV1 = Number.NaN;
	let lastV2 = Number.NaN;
	let lastOrdinal = -1;
	let memoHits = 0;
	let memoMisses = 0;
	let memoSets = 0;
	let nearestCount = 0;
	let candidateEvaluations = 0;

	function findNearestOrdinal(v0: number, v1: number, v2: number) {
		if (v0 === lastV0 && v1 === lastV1 && v2 === lastV2) {
			memoHits++;
			return lastOrdinal;
		}
		memoMisses++;
		nearestCount++;

		let winner = -1;
		let best = Infinity;
		if (settings.colorSpace === 'oklch') {
			candidateEvaluations += count;
			for (let ordinal = 0; ordinal < count; ordinal++) {
				const dl = v0 - paletteV0[ordinal]!;
				const dc = v1 - paletteV1[ordinal]!;
				let hue = Math.abs(v2 - paletteV2[ordinal]!);
				if (hue > Math.PI) hue = Math.PI * 2 - hue;
				const dh = Math.min(v1, paletteV1[ordinal]!) * hue;
				const distance = dl * dl + dc * dc + dh * dh;
				if (distance < best) {
					best = distance;
					winner = ordinal;
				}
			}
		} else {
			candidateEvaluations += count;
			for (let ordinal = 0; ordinal < count; ordinal++) {
				const dx = v0 - paletteV0[ordinal]!;
				const dy = v1 - paletteV1[ordinal]!;
				const dz = v2 - paletteV2[ordinal]!;
				const distance = dx * dx + dy * dy + dz * dz;
				if (distance < best) {
					best = distance;
					winner = ordinal;
				}
			}
		}

		lastV0 = v0;
		lastV1 = v1;
		lastV2 = v2;
		lastOrdinal = winner;
		memoSets++;
		return winner;
	}

	return {
		nearest(v0, v1, v2) {
			const ordinal = findNearestOrdinal(v0, v1, v2);
			return ordinal === -1 ? undefined : space.colors[ordinal];
		},
		nearestIndex(v0, v1, v2) {
			const ordinal = findNearestOrdinal(v0, v1, v2);
			return ordinal === -1 ? -1 : paletteIndices[ordinal]!;
		},
		nearestOrdinal: findNearestOrdinal,
		indexForOrdinal(ordinal) {
			return ordinal === -1 ? -1 : paletteIndices[ordinal]!;
		},
		vector0ForOrdinal(ordinal) {
			return paletteV0[ordinal] ?? 0;
		},
		vector1ForOrdinal(ordinal) {
			return paletteV1[ordinal] ?? 0;
		},
		vector2ForOrdinal(ordinal) {
			return paletteV2[ordinal] ?? 0;
		},
		flushCounts() {
			caches?.recordCount?.('vector memo hit', memoHits);
			caches?.recordCount?.('vector memo miss', memoMisses);
			caches?.recordCount?.('vector memo set', memoSets);
			caches?.recordCount?.('nearest vector', nearestCount);
			caches?.recordCount?.('candidate evaluations', candidateEvaluations);
		}
	};
}

function supportsThresholdByteVectorMatching(colorSpace: ColorSpaceId) {
	return colorSpace === 'weighted-rgb-601' || colorSpace === 'weighted-rgb-709';
}

function byteVectorComponent(value: number, colorSpace: ColorSpaceId, channel: 0 | 1 | 2) {
	switch (colorSpace) {
		case 'linear-rgb':
			return srgbByteToLinear(value);
		case 'weighted-rgb-601':
			if (channel === 0) return value * WEIGHTED_RGB_601_R;
			if (channel === 1) return value * WEIGHTED_RGB_601_G;
			return value * WEIGHTED_RGB_601_B;
		case 'weighted-rgb-709':
			if (channel === 0) return value * WEIGHTED_RGB_709_R;
			if (channel === 1) return value * WEIGHTED_RGB_709_G;
			return value * WEIGHTED_RGB_709_B;
		case 'srgb':
		default:
			return value;
	}
}

function createThresholdByteVectorMatcher(
	space: PaletteVectorSpace,
	colorSpace: ColorSpaceId,
	thresholds: readonly number[],
	strength: number,
	pixels: number,
	caches?: QuantizeCaches
): ThresholdByteVectorMatcher | undefined {
	if (!supportsThresholdByteVectorMatching(colorSpace)) return undefined;
	const count = space.colors.length;
	const paletteIndices = new Int16Array(count);
	const termsPerThreshold = count * 256;
	const term0 = new Float64Array(thresholds.length * termsPerThreshold);
	const term1 = new Float64Array(thresholds.length * termsPerThreshold);
	const term2 = new Float64Array(thresholds.length * termsPerThreshold);
	const thresholdKeyBases = new Uint32Array(thresholds.length);
	const thresholdTermBases = new Uint32Array(thresholds.length);
	const tableBytes =
		term0.byteLength +
		term1.byteLength +
		term2.byteLength +
		thresholdKeyBases.byteLength +
		thresholdTermBases.byteLength;
	for (let thresholdIndex = 0; thresholdIndex < thresholds.length; thresholdIndex++) {
		const amount = thresholds[thresholdIndex]! * strength * COLOR_SPACE_THRESHOLD_SCALE;
		const offset0 = amount * space.ranges[0];
		const offset1 = amount * space.ranges[1];
		const offset2 = amount * space.ranges[2];
		const thresholdBase = thresholdIndex * termsPerThreshold;
		thresholdKeyBases[thresholdIndex] = (thresholdIndex * 0x1000000) >>> 0;
		thresholdTermBases[thresholdIndex] = thresholdBase;
		for (let ordinal = 0; ordinal < count; ordinal++) {
			paletteIndices[ordinal] = space.colors[ordinal]!.index;
		}
		for (let value = 0; value < 256; value++) {
			const base = thresholdBase + value * count;
			const v0 = byteVectorComponent(value, colorSpace, 0) + offset0;
			const v1 = byteVectorComponent(value, colorSpace, 1) + offset1;
			const v2 = byteVectorComponent(value, colorSpace, 2) + offset2;
			for (let ordinal = 0; ordinal < count; ordinal++) {
				const color = space.colors[ordinal]!;
				const d0 = v0 - color.vector[0];
				const d1 = v1 - color.vector[1];
				const d2 = v2 - color.vector[2];
				term0[base + ordinal] = d0 * d0;
				term1[base + ordinal] = d1 * d1;
				term2[base + ordinal] = d2 * d2;
			}
		}
	}
	const cacheBits = Math.min(
		MAX_THRESHOLD_VECTOR_CACHE_BITS,
		Math.max(MIN_THRESHOLD_VECTOR_CACHE_BITS, Math.ceil(Math.log2(pixels * 2)))
	);
	const cacheSize = 1 << cacheBits;
	const cacheMask = cacheSize - 1;
	const cacheKeys = new Uint32Array(cacheSize);
	const cacheValues = new Uint16Array(cacheSize);
	const cacheValid = new Uint8Array(cacheSize);
	const cacheBytes = cacheKeys.byteLength + cacheValues.byteLength + cacheValid.byteLength;
	let nearestCount = 0;
	let memoHits = 0;
	let memoMisses = 0;
	let memoSets = 0;
	let memoCollisions = 0;
	let candidateEvaluations = 0;
	return {
		nearestIndexRgb(r, g, b, thresholdIndex) {
			const rgbKey = ((r << 16) | (g << 8) | b) >>> 0;
			const key = (thresholdKeyBases[thresholdIndex]! | rgbKey) >>> 0;
			const slot = (Math.imul(key ^ (key >>> 16), 0x45d9f3b) >>> 0) & cacheMask;
			if (cacheValid[slot]) {
				if (cacheKeys[slot] === key) {
					memoHits++;
					return cacheValues[slot]!;
				}
				memoCollisions++;
			}
			memoMisses++;
			nearestCount++;
			candidateEvaluations += count;
			let winner = -1;
			let best = Infinity;
			const thresholdBase = thresholdTermBases[thresholdIndex]!;
			const base0 = thresholdBase + r * count;
			const base1 = thresholdBase + g * count;
			const base2 = thresholdBase + b * count;
			for (let ordinal = 0; ordinal < count; ordinal++) {
				const distance =
					term0[base0 + ordinal]! + term1[base1 + ordinal]! + term2[base2 + ordinal]!;
				if (distance < best) {
					best = distance;
					winner = ordinal;
				}
			}
			const index = winner === -1 ? -1 : paletteIndices[winner]!;
			cacheKeys[slot] = key;
			cacheValues[slot] = index;
			cacheValid[slot] = 1;
			memoSets++;
			return index;
		},
		flushCounts() {
			caches?.recordCount?.('nearest vector', nearestCount);
			caches?.recordCount?.('vector memo hit', memoHits);
			caches?.recordCount?.('vector memo miss', memoMisses);
			caches?.recordCount?.('vector memo set', memoSets);
			caches?.recordCount?.('threshold cache collision', memoCollisions);
			caches?.recordCount?.('candidate evaluations', candidateEvaluations);
			caches?.recordCount?.('threshold table bytes', tableBytes);
			caches?.recordCount?.('threshold cache bytes', cacheBytes);
		}
	};
}

function cachedVectorAt(vectors: ColorVectorImage, index: number): ColorVector {
	return [vectors.v0[index]!, vectors.v1[index]!, vectors.v2[index]!];
}

function vectorForCompositedPixel(
	source: Uint8ClampedArray,
	offset: number,
	settings: ProcessingSettings,
	matte: Rgb
) {
	const rgb = compositedRgb(
		{ r: source[offset]!, g: source[offset + 1]!, b: source[offset + 2]! },
		source[offset + 3]!,
		settings.output.alphaMode,
		matte
	);
	return vectorForRgb(rgb.r, rgb.g, rgb.b, settings.colorSpace);
}

function buildColorVectorImage(
	image: ImageData,
	settings: ProcessingSettings,
	matte: Rgb,
	kind: 'composited' | 'source'
): ColorVectorImage {
	const pixels = image.width * image.height;
	const v0 = new Float32Array(pixels);
	const v1 = new Float32Array(pixels);
	const v2 = new Float32Array(pixels);
	const source = image.data;
	const alphaMode = kind === 'source' ? 'preserve' : settings.output.alphaMode;

	if (alphaMode === 'preserve') {
		switch (settings.colorSpace) {
			case 'srgb':
			case 'weighted-rgb':
				for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
					v0[index] = source[offset]!;
					v1[index] = source[offset + 1]!;
					v2[index] = source[offset + 2]!;
				}
				break;
			case 'weighted-rgb-601':
				for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
					v0[index] = source[offset]! * WEIGHTED_RGB_601_R;
					v1[index] = source[offset + 1]! * WEIGHTED_RGB_601_G;
					v2[index] = source[offset + 2]! * WEIGHTED_RGB_601_B;
				}
				break;
			case 'weighted-rgb-709':
				for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
					v0[index] = source[offset]! * WEIGHTED_RGB_709_R;
					v1[index] = source[offset + 1]! * WEIGHTED_RGB_709_G;
					v2[index] = source[offset + 2]! * WEIGHTED_RGB_709_B;
				}
				break;
			case 'linear-rgb': {
				let lastKey = -1;
				let last0 = 0;
				let last1 = 0;
				let last2 = 0;
				for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
					const r = source[offset]!;
					const g = source[offset + 1]!;
					const b = source[offset + 2]!;
					const key = (r << 16) | (g << 8) | b;
					if (key !== lastKey) {
						last0 = srgbByteToLinear(r);
						last1 = srgbByteToLinear(g);
						last2 = srgbByteToLinear(b);
						lastKey = key;
					}
					v0[index] = last0;
					v1[index] = last1;
					v2[index] = last2;
				}
				break;
			}
			case 'cielab': {
				let lastKey = -1;
				let last0 = 0;
				let last1 = 0;
				let last2 = 0;
				for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
					const r = source[offset]!;
					const g = source[offset + 1]!;
					const b = source[offset + 2]!;
					const key = (r << 16) | (g << 8) | b;
					if (key !== lastKey) {
						const rr = srgbByteToLinear(r);
						const gg = srgbByteToLinear(g);
						const bb = srgbByteToLinear(b);
						const x = rr * 0.4124564 + gg * 0.3575761 + bb * 0.1804375;
						const y = rr * 0.2126729 + gg * 0.7151522 + bb * 0.072175;
						const z = rr * 0.0193339 + gg * 0.119192 + bb * 0.9503041;
						const fx = labPivot(x / REF_X);
						const fy = labPivot(y / REF_Y);
						const fz = labPivot(z / REF_Z);
						last0 = 116 * fy - 16;
						last1 = 500 * (fx - fy);
						last2 = 200 * (fy - fz);
						lastKey = key;
					}
					v0[index] = last0;
					v1[index] = last1;
					v2[index] = last2;
				}
				break;
			}
			case 'oklch': {
				let lastKey = -1;
				let last0 = 0;
				let last1 = 0;
				let last2 = 0;
				for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
					const r = source[offset]!;
					const g = source[offset + 1]!;
					const b = source[offset + 2]!;
					const key = (r << 16) | (g << 8) | b;
					if (key !== lastKey) {
						const rr = srgbByteToLinear(r);
						const gg = srgbByteToLinear(g);
						const bb = srgbByteToLinear(b);
						const l = 0.4122214708 * rr + 0.5363325363 * gg + 0.0514459929 * bb;
						const m = 0.2119034982 * rr + 0.6806995451 * gg + 0.1073969566 * bb;
						const s = 0.0883024619 * rr + 0.2817188376 * gg + 0.6299787005 * bb;
						const lRoot = Math.cbrt(l);
						const mRoot = Math.cbrt(m);
						const sRoot = Math.cbrt(s);
						const labA = 1.9779984951 * lRoot - 2.428592205 * mRoot + 0.4505937099 * sRoot;
						const labB = 0.0259040371 * lRoot + 0.7827717662 * mRoot - 0.808675766 * sRoot;
						last0 = 0.2104542553 * lRoot + 0.793617785 * mRoot - 0.0040720468 * sRoot;
						last1 = Math.hypot(labA, labB);
						last2 = Math.atan2(labB, labA);
						lastKey = key;
					}
					v0[index] = last0;
					v1[index] = last1;
					v2[index] = last2;
				}
				break;
			}
			case 'oklab':
			default: {
				let lastKey = -1;
				let last0 = 0;
				let last1 = 0;
				let last2 = 0;
				for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
					const r = source[offset]!;
					const g = source[offset + 1]!;
					const b = source[offset + 2]!;
					const key = (r << 16) | (g << 8) | b;
					if (key !== lastKey) {
						const rr = srgbByteToLinear(r);
						const gg = srgbByteToLinear(g);
						const bb = srgbByteToLinear(b);
						const l = 0.4122214708 * rr + 0.5363325363 * gg + 0.0514459929 * bb;
						const m = 0.2119034982 * rr + 0.6806995451 * gg + 0.1073969566 * bb;
						const s = 0.0883024619 * rr + 0.2817188376 * gg + 0.6299787005 * bb;
						const lRoot = Math.cbrt(l);
						const mRoot = Math.cbrt(m);
						const sRoot = Math.cbrt(s);
						last0 = 0.2104542553 * lRoot + 0.793617785 * mRoot - 0.0040720468 * sRoot;
						last1 = 1.9779984951 * lRoot - 2.428592205 * mRoot + 0.4505937099 * sRoot;
						last2 = 0.0259040371 * lRoot + 0.7827717662 * mRoot - 0.808675766 * sRoot;
						lastKey = key;
					}
					v0[index] = last0;
					v1[index] = last1;
					v2[index] = last2;
				}
				break;
			}
		}
		return {
			width: image.width,
			height: image.height,
			colorSpace: settings.colorSpace,
			v0,
			v1,
			v2
		};
	}

	switch (settings.colorSpace) {
		case 'srgb':
		case 'weighted-rgb':
			for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
				let r = source[offset]!;
				let g = source[offset + 1]!;
				let b = source[offset + 2]!;
				const alpha = source[offset + 3]!;
				if (alpha !== 255) {
					const opacity = alpha / 255;
					if (alphaMode === 'premultiplied') {
						r = Math.round(r * opacity);
						g = Math.round(g * opacity);
						b = Math.round(b * opacity);
					} else {
						const background = 1 - opacity;
						r = Math.round(r * opacity + matte.r * background);
						g = Math.round(g * opacity + matte.g * background);
						b = Math.round(b * opacity + matte.b * background);
					}
				}
				v0[index] = r;
				v1[index] = g;
				v2[index] = b;
			}
			break;
		case 'weighted-rgb-601':
			for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
				let r = source[offset]!;
				let g = source[offset + 1]!;
				let b = source[offset + 2]!;
				const alpha = source[offset + 3]!;
				if (alpha !== 255) {
					const opacity = alpha / 255;
					if (alphaMode === 'premultiplied') {
						r = Math.round(r * opacity);
						g = Math.round(g * opacity);
						b = Math.round(b * opacity);
					} else {
						const background = 1 - opacity;
						r = Math.round(r * opacity + matte.r * background);
						g = Math.round(g * opacity + matte.g * background);
						b = Math.round(b * opacity + matte.b * background);
					}
				}
				v0[index] = r * WEIGHTED_RGB_601_R;
				v1[index] = g * WEIGHTED_RGB_601_G;
				v2[index] = b * WEIGHTED_RGB_601_B;
			}
			break;
		case 'weighted-rgb-709':
			for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
				let r = source[offset]!;
				let g = source[offset + 1]!;
				let b = source[offset + 2]!;
				const alpha = source[offset + 3]!;
				if (alpha !== 255) {
					const opacity = alpha / 255;
					if (alphaMode === 'premultiplied') {
						r = Math.round(r * opacity);
						g = Math.round(g * opacity);
						b = Math.round(b * opacity);
					} else {
						const background = 1 - opacity;
						r = Math.round(r * opacity + matte.r * background);
						g = Math.round(g * opacity + matte.g * background);
						b = Math.round(b * opacity + matte.b * background);
					}
				}
				v0[index] = r * WEIGHTED_RGB_709_R;
				v1[index] = g * WEIGHTED_RGB_709_G;
				v2[index] = b * WEIGHTED_RGB_709_B;
			}
			break;
		case 'linear-rgb': {
			let lastKey = -1;
			let last0 = 0;
			let last1 = 0;
			let last2 = 0;
			for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
				let r = source[offset]!;
				let g = source[offset + 1]!;
				let b = source[offset + 2]!;
				const alpha = source[offset + 3]!;
				if (alpha !== 255) {
					const opacity = alpha / 255;
					if (alphaMode === 'premultiplied') {
						r = Math.round(r * opacity);
						g = Math.round(g * opacity);
						b = Math.round(b * opacity);
					} else {
						const background = 1 - opacity;
						r = Math.round(r * opacity + matte.r * background);
						g = Math.round(g * opacity + matte.g * background);
						b = Math.round(b * opacity + matte.b * background);
					}
				}
				const key = (r << 16) | (g << 8) | b;
				if (key !== lastKey) {
					last0 = srgbByteToLinear(r);
					last1 = srgbByteToLinear(g);
					last2 = srgbByteToLinear(b);
					lastKey = key;
				}
				v0[index] = last0;
				v1[index] = last1;
				v2[index] = last2;
			}
			break;
		}
		case 'cielab': {
			let lastKey = -1;
			let last0 = 0;
			let last1 = 0;
			let last2 = 0;
			for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
				let r = source[offset]!;
				let g = source[offset + 1]!;
				let b = source[offset + 2]!;
				const alpha = source[offset + 3]!;
				if (alpha !== 255) {
					const opacity = alpha / 255;
					if (alphaMode === 'premultiplied') {
						r = Math.round(r * opacity);
						g = Math.round(g * opacity);
						b = Math.round(b * opacity);
					} else {
						const background = 1 - opacity;
						r = Math.round(r * opacity + matte.r * background);
						g = Math.round(g * opacity + matte.g * background);
						b = Math.round(b * opacity + matte.b * background);
					}
				}
				const key = (r << 16) | (g << 8) | b;
				if (key !== lastKey) {
					const rr = srgbByteToLinear(r);
					const gg = srgbByteToLinear(g);
					const bb = srgbByteToLinear(b);
					const x = rr * 0.4124564 + gg * 0.3575761 + bb * 0.1804375;
					const y = rr * 0.2126729 + gg * 0.7151522 + bb * 0.072175;
					const z = rr * 0.0193339 + gg * 0.119192 + bb * 0.9503041;
					const fx = labPivot(x / REF_X);
					const fy = labPivot(y / REF_Y);
					const fz = labPivot(z / REF_Z);
					last0 = 116 * fy - 16;
					last1 = 500 * (fx - fy);
					last2 = 200 * (fy - fz);
					lastKey = key;
				}
				v0[index] = last0;
				v1[index] = last1;
				v2[index] = last2;
			}
			break;
		}
		case 'oklch': {
			let lastKey = -1;
			let last0 = 0;
			let last1 = 0;
			let last2 = 0;
			for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
				let r = source[offset]!;
				let g = source[offset + 1]!;
				let b = source[offset + 2]!;
				const alpha = source[offset + 3]!;
				if (alpha !== 255) {
					const opacity = alpha / 255;
					if (alphaMode === 'premultiplied') {
						r = Math.round(r * opacity);
						g = Math.round(g * opacity);
						b = Math.round(b * opacity);
					} else {
						const background = 1 - opacity;
						r = Math.round(r * opacity + matte.r * background);
						g = Math.round(g * opacity + matte.g * background);
						b = Math.round(b * opacity + matte.b * background);
					}
				}
				const key = (r << 16) | (g << 8) | b;
				if (key !== lastKey) {
					const rr = srgbByteToLinear(r);
					const gg = srgbByteToLinear(g);
					const bb = srgbByteToLinear(b);
					const l = 0.4122214708 * rr + 0.5363325363 * gg + 0.0514459929 * bb;
					const m = 0.2119034982 * rr + 0.6806995451 * gg + 0.1073969566 * bb;
					const s = 0.0883024619 * rr + 0.2817188376 * gg + 0.6299787005 * bb;
					const lRoot = Math.cbrt(l);
					const mRoot = Math.cbrt(m);
					const sRoot = Math.cbrt(s);
					const labA = 1.9779984951 * lRoot - 2.428592205 * mRoot + 0.4505937099 * sRoot;
					const labB = 0.0259040371 * lRoot + 0.7827717662 * mRoot - 0.808675766 * sRoot;
					last0 = 0.2104542553 * lRoot + 0.793617785 * mRoot - 0.0040720468 * sRoot;
					last1 = Math.hypot(labA, labB);
					last2 = Math.atan2(labB, labA);
					lastKey = key;
				}
				v0[index] = last0;
				v1[index] = last1;
				v2[index] = last2;
			}
			break;
		}
		case 'oklab':
		default: {
			let lastKey = -1;
			let last0 = 0;
			let last1 = 0;
			let last2 = 0;
			for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
				let r = source[offset]!;
				let g = source[offset + 1]!;
				let b = source[offset + 2]!;
				const alpha = source[offset + 3]!;
				if (alpha !== 255) {
					const opacity = alpha / 255;
					if (alphaMode === 'premultiplied') {
						r = Math.round(r * opacity);
						g = Math.round(g * opacity);
						b = Math.round(b * opacity);
					} else {
						const background = 1 - opacity;
						r = Math.round(r * opacity + matte.r * background);
						g = Math.round(g * opacity + matte.g * background);
						b = Math.round(b * opacity + matte.b * background);
					}
				}
				const key = (r << 16) | (g << 8) | b;
				if (key !== lastKey) {
					const rr = srgbByteToLinear(r);
					const gg = srgbByteToLinear(g);
					const bb = srgbByteToLinear(b);
					const l = 0.4122214708 * rr + 0.5363325363 * gg + 0.0514459929 * bb;
					const m = 0.2119034982 * rr + 0.6806995451 * gg + 0.1073969566 * bb;
					const s = 0.0883024619 * rr + 0.2817188376 * gg + 0.6299787005 * bb;
					const lRoot = Math.cbrt(l);
					const mRoot = Math.cbrt(m);
					const sRoot = Math.cbrt(s);
					last0 = 0.2104542553 * lRoot + 0.793617785 * mRoot - 0.0040720468 * sRoot;
					last1 = 1.9779984951 * lRoot - 2.428592205 * mRoot + 0.4505937099 * sRoot;
					last2 = 0.0259040371 * lRoot + 0.7827717662 * mRoot - 0.808675766 * sRoot;
					lastKey = key;
				}
				v0[index] = last0;
				v1[index] = last1;
				v2[index] = last2;
			}
			break;
		}
	}
	return { width: image.width, height: image.height, colorSpace: settings.colorSpace, v0, v1, v2 };
}

function cachedColorVectorImage(
	image: ImageData,
	settings: ProcessingSettings,
	matte: Rgb,
	kind: 'composited' | 'source',
	caches?: QuantizeCaches
) {
	if (!supportsCachedVectorMatching(settings.colorSpace)) return undefined;
	const scope = caches?.colorVectorImageScope;
	if (!scope) return undefined;
	const key = `${scope}|${
		kind === 'composited'
			? compositedVectorCacheKey(settings.colorSpace, settings.output.alphaMode, matte)
			: sourceVectorCacheKey(settings.colorSpace)
	}`;
	const label = `color space convert ${kind} image` as const;
	const lookupStart = performance.now();
	const cached = caches?.getColorVectorImage?.(key);
	recordTiming(caches, `${label} cache lookup`, lookupStart);
	if (
		cached?.width === image.width &&
		cached.height === image.height &&
		cached.colorSpace === settings.colorSpace
	) {
		return cached;
	}
	const bytes = colorVectorImageBytes(image.width, image.height);
	if (!caches?.setColorVectorImage || caches.canStoreColorVectorImage?.(key, bytes) === false) {
		return undefined;
	}
	const buildStart = performance.now();
	const vectors = buildColorVectorImage(image, settings, matte, kind);
	recordTiming(caches, label, buildStart);
	const cacheWriteStart = performance.now();
	caches.setColorVectorImage(key, vectors, bytes);
	recordTiming(caches, `${label} cache write`, cacheWriteStart);
	return caches.getColorVectorImage?.(key) ?? vectors;
}

function colorSpaceThresholdIndexRgb(
	r: number,
	g: number,
	b: number,
	threshold: number,
	space: PaletteVectorSpace,
	matcher: PaletteVectorMatcher,
	settings: ProcessingSettings,
	strength: number
) {
	switch (settings.colorSpace) {
		case 'srgb':
		case 'weighted-rgb':
			return colorSpaceThresholdIndexValues(r, g, b, threshold, space, matcher, strength);
		case 'weighted-rgb-601':
			return colorSpaceThresholdIndexValues(
				r * WEIGHTED_RGB_601_R,
				g * WEIGHTED_RGB_601_G,
				b * WEIGHTED_RGB_601_B,
				threshold,
				space,
				matcher,
				strength
			);
		case 'weighted-rgb-709':
			return colorSpaceThresholdIndexValues(
				r * WEIGHTED_RGB_709_R,
				g * WEIGHTED_RGB_709_G,
				b * WEIGHTED_RGB_709_B,
				threshold,
				space,
				matcher,
				strength
			);
		case 'linear-rgb':
			return colorSpaceThresholdIndexValues(
				srgbByteToLinear(r),
				srgbByteToLinear(g),
				srgbByteToLinear(b),
				threshold,
				space,
				matcher,
				strength
			);
		default: {
			const source = vectorForRgb(r, g, b, settings.colorSpace);
			return colorSpaceThresholdIndexValues(
				source[0],
				source[1],
				source[2],
				threshold,
				space,
				matcher,
				strength
			);
		}
	}
}

function colorSpaceThresholdIndexValues(
	source0: number,
	source1: number,
	source2: number,
	threshold: number,
	space: PaletteVectorSpace,
	matcher: PaletteVectorMatcher,
	strength: number
) {
	if (strength <= 0) return -1;
	const amount = threshold * strength * COLOR_SPACE_THRESHOLD_SCALE;
	return matcher.nearestIndex(
		source0 + amount * space.ranges[0],
		source1 + amount * space.ranges[1],
		source2 + amount * space.ranges[2]
	);
}

function smoothstep(edge0: number, edge1: number, value: number) {
	if (edge0 === edge1) return value >= edge1 ? 1 : 0;
	const t = Math.max(0, Math.min(1, (value - edge0) / (edge1 - edge0)));
	return t * t * (3 - 2 * t);
}

function sourceVectorAt(
	source: Uint8ClampedArray,
	width: number,
	height: number,
	x: number,
	y: number,
	settings: ProcessingSettings,
	vectors?: ColorVectorImage
) {
	const xx = Math.min(width - 1, Math.max(0, x));
	const yy = Math.min(height - 1, Math.max(0, y));
	const index = yy * width + xx;
	if (vectors) return cachedVectorAt(vectors, index);
	const offset = index * 4;
	return vectorForRgb(
		source[offset]!,
		source[offset + 1]!,
		source[offset + 2]!,
		settings.colorSpace
	);
}

function placementMask(
	source: Uint8ClampedArray,
	width: number,
	height: number,
	x: number,
	y: number,
	settings: ProcessingSettings,
	space: PaletteVectorSpace,
	vectors?: ColorVectorImage
) {
	if (!usesAdaptivePlacement(settings)) return 1;

	const radius = Math.max(1, Math.round(settings.dither.placementRadius ?? 3));
	const center = sourceVectorAt(source, width, height, x, y, settings, vectors);
	const offsets = [
		[-radius, 0],
		[radius, 0],
		[0, -radius],
		[0, radius],
		[-radius, -radius],
		[radius, -radius],
		[-radius, radius],
		[radius, radius]
	] as const;
	let total = 0;
	for (const [dx, dy] of offsets) {
		total += Math.sqrt(
			vectorDistance(
				center,
				sourceVectorAt(source, width, height, x + dx, y + dy, settings, vectors),
				settings
			)
		);
	}
	const diagonal = Math.max(
		Math.hypot(space.ranges[0], space.ranges[1], space.ranges[2]),
		Number.EPSILON
	);
	const variance = (total / offsets.length / diagonal) * 100;
	const threshold = settings.dither.placementThreshold ?? 12;
	const softness = settings.dither.placementSoftness ?? 8;
	return smoothstep(threshold - softness, threshold + softness, variance);
}

type PreparedQuantize = {
	warnings: string[];
	nextPalette: EnabledPaletteColor[];
	transparentIndexValue: number;
	matcher: ReturnType<typeof createPaletteMatcher>;
	matte: Rgb;
	paletteCacheKey: string;
	fallbackTransparentIndex: number;
	strength: number;
};

function prepareQuantize(
	palette: EnabledPaletteColor[],
	settings: ProcessingSettings,
	caches?: QuantizeCaches
): PreparedQuantize | QuantizeResult {
	const prepareStart = performance.now();
	const warnings: string[] = [];
	const nextPalette = palette.slice(0, 256);
	if (palette.length > 256)
		warnings.push('Palette was truncated to 256 entries for indexed PNG export.');

	const tIndex = transparentIndex(nextPalette);
	const visible = nextPalette.filter((color) => color.rgb && color.kind !== 'transparent');
	if (visible.length === 0) {
		if (tIndex === -1) throw new Error('Enable at least one visible color or Transparent.');
		warnings.push('Only Transparent is enabled; every output pixel is transparent.');
		return {
			indices: new Uint8Array(0),
			palette: nextPalette,
			transparentIndex: tIndex,
			warnings
		};
	}

	const fallbackTransparent = tIndex === -1 ? darkestVisible(visible) : undefined;
	if (settings.output.alphaMode === 'preserve' && tIndex === -1) {
		warnings.push(
			'Transparent is disabled; alpha-thresholded pixels use the darkest enabled visible color.'
		);
	}

	const { matte, warning } = resolveMatteRgb(nextPalette, visible, settings.output.matteKey);
	if (warning) warnings.push(warning);
	recordTiming(caches, 'palette prepare', prepareStart);
	const matcherStart = performance.now();
	const matcher = createPaletteMatcher(nextPalette, settings.colorSpace, {
		denseRgbMemo: usesByteRgbMatcher(settings),
		distanceTables: true
	});
	recordTiming(caches, 'matcher build', matcherStart);
	return {
		warnings,
		nextPalette,
		transparentIndexValue: tIndex,
		matcher,
		matte,
		paletteCacheKey: paletteVectorCacheKey(nextPalette, settings.colorSpace),
		fallbackTransparentIndex: fallbackTransparent ? nextPalette.indexOf(fallbackTransparent) : -1,
		strength: settings.dither.strength / 100
	};
}

export function prepareQuantizeColorSpace(
	image: ImageData,
	palette: EnabledPaletteColor[],
	settings: ProcessingSettings,
	caches?: QuantizeCaches
) {
	const prepared = prepareQuantize(palette, settings, caches);
	if ('indices' in prepared) return;
	const { matcher, matte, paletteCacheKey, strength } = prepared;
	const bayerSize = bayerSizeForAlgorithm(settings.dither.algorithm);
	const useBayer = Boolean(bayerSize && strength > 0);
	const useRandom = settings.dither.algorithm === 'random' && strength > 0;
	const vectorSpace = paletteVectorSpace(matcher, settings, paletteCacheKey, caches);
	if (errorKernelForAlgorithm(settings.dither.algorithm) && strength > 0) {
		cachedColorVectorImage(image, settings, matte, 'composited', caches);
		if (usesAdaptivePlacement(settings))
			cachedColorVectorImage(image, settings, matte, 'source', caches);
		return;
	}
	if (
		supportsCachedVectorMatching(settings.colorSpace) &&
		((!useBayer && !useRandom) || supportsVectorDither(settings))
	) {
		cachedColorVectorImage(image, settings, matte, 'composited', caches);
	}
	if ((useBayer || useRandom) && usesAdaptivePlacement(settings)) {
		cachedColorVectorImage(image, settings, matte, 'source', caches);
	}
	void vectorSpace;
}

export function quantizeImage(
	image: ImageData,
	palette: EnabledPaletteColor[],
	settings: ProcessingSettings,
	caches?: QuantizeCaches
): QuantizeResult {
	const prepared = prepareQuantize(palette, settings, caches);
	if ('indices' in prepared) {
		return {
			...prepared,
			indices: new Uint8Array(image.width * image.height).fill(prepared.transparentIndex)
		};
	}
	const {
		warnings,
		nextPalette,
		transparentIndexValue: tIndex,
		matcher,
		matte,
		paletteCacheKey,
		fallbackTransparentIndex,
		strength
	} = prepared;
	const pixels = image.width * image.height;
	const indices = new Uint8Array(pixels);

	if (errorKernelForAlgorithm(settings.dither.algorithm) && strength > 0) {
		quantizeErrorDiffusion(
			image,
			indices,
			matcher,
			settings,
			matte,
			tIndex,
			fallbackTransparentIndex,
			strength,
			paletteCacheKey,
			caches
		);
	} else {
		quantizeDirect(
			image,
			indices,
			matcher,
			settings,
			matte,
			tIndex,
			fallbackTransparentIndex,
			strength,
			paletteCacheKey,
			caches
		);
	}
	recordMatcherMemoStats(matcher, caches);

	return { indices, palette: nextPalette, transparentIndex: tIndex, warnings };
}

function recordMatcherMemoStats(
	matcher: ReturnType<typeof createPaletteMatcher>,
	caches: QuantizeCaches | undefined
) {
	const stats = matcher.memoStats();
	caches?.recordCount?.('rgb memo hit', stats.rgbHits);
	caches?.recordCount?.('rgb memo miss', stats.rgbMisses);
	caches?.recordCount?.('rgb memo set', stats.rgbSets);
	caches?.recordCount?.('rgb memo eviction', stats.rgbEvictions);
	caches?.recordCount?.('candidate evaluations', stats.candidateEvaluations);
	caches?.recordCount?.('dense cache bytes', stats.denseRgbMemoBytes);
	caches?.recordCount?.('distance table bytes', stats.distanceTableBytes);
}

function resolveMatteRgb(
	palette: EnabledPaletteColor[],
	visible: EnabledPaletteColor[],
	matteKey: string
): { matte: Rgb; warning?: string } {
	const selected = palette.find((color) => color.key === matteKey)?.rgb;
	if (selected) return { matte: selected };
	const matteRgb = rgbFromHexKey(matteKey);
	if (matteRgb) {
		return {
			matte: nearestVisibleColor(matteRgb, visible).rgb!,
			warning: 'Matte color is disabled; using the nearest enabled visible color for alpha matte.'
		};
	}
	return {
		matte: visible[0]!.rgb!,
		warning: 'Matte color is unavailable; using the first enabled visible color for alpha matte.'
	};
}

function rgbFromHexKey(key: string): Rgb | undefined {
	if (!/^#[0-9a-fA-F]{6}$/.test(key)) return undefined;
	return {
		r: Number.parseInt(key.slice(1, 3), 16),
		g: Number.parseInt(key.slice(3, 5), 16),
		b: Number.parseInt(key.slice(5, 7), 16)
	};
}

function nearestVisibleColor(rgb: Rgb, visible: EnabledPaletteColor[]) {
	let best = visible[0]!;
	let bestDistance = Number.POSITIVE_INFINITY;
	for (const color of visible) {
		const candidate = color.rgb!;
		const distance =
			(candidate.r - rgb.r) ** 2 + (candidate.g - rgb.g) ** 2 + (candidate.b - rgb.b) ** 2;
		if (distance < bestDistance) {
			best = color;
			bestDistance = distance;
		}
	}
	return best;
}

function quantizeDirect(
	image: ImageData,
	indices: Uint8Array,
	matcher: ReturnType<typeof createPaletteMatcher>,
	settings: ProcessingSettings,
	matte: Rgb,
	transparentIndexValue: number,
	fallbackTransparentIndex: number,
	strength: number,
	paletteCacheKey: string,
	caches?: QuantizeCaches
) {
	let randomState = settings.dither.seed >>> 0;
	const bayerSize = bayerSizeForAlgorithm(settings.dither.algorithm);
	const bayer = bayerSize ? normalizedBayerMatrix(bayerSize) : undefined;
	const bayerMask = bayerSize ? bayerSize - 1 : 0;
	const source = image.data;
	const pixels = image.width * image.height;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const useBayer = Boolean(bayer && bayerSize && strength > 0);
	const useRandom = settings.dither.algorithm === 'random' && strength > 0;
	const useVectorDither = (useBayer || useRandom) && supportsVectorDither(settings);
	const noiseScale = RGB_DITHER_NOISE_SCALE * strength;
	const vectorSpace = paletteVectorSpace(matcher, settings, paletteCacheKey, caches);
	const vectorMatcher = createPaletteVectorMatcher(vectorSpace, settings, caches);
	const useAdaptivePlacement = usesAdaptivePlacement(settings);
	const thresholdVectorMatcher =
		useBayer && bayer && useVectorDither && !useAdaptivePlacement
			? createThresholdByteVectorMatcher(
					vectorSpace,
					settings.colorSpace,
					bayer,
					strength,
					pixels,
					caches
				)
			: undefined;
	const needsCompositedVectors =
		supportsCachedVectorMatching(settings.colorSpace) && useVectorDither && !thresholdVectorMatcher;
	const compositedVectors = needsCompositedVectors
		? cachedColorVectorImage(image, settings, matte, 'composited', caches)
		: undefined;
	const sourceVectors =
		(useBayer || useRandom) && useAdaptivePlacement
			? cachedColorVectorImage(image, settings, matte, 'source', caches)
			: undefined;
	const useDirectVectorMatch = Boolean(compositedVectors && !useBayer && !useRandom);
	let nearestRgbCount = 0;
	const loopStart = performance.now();

	if (!useBayer && !useRandom) {
		if (useDirectVectorMatch && compositedVectors) {
			for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
				const alpha = source[offset + 3]!;
				if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
					indices[index] =
						transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
					continue;
				}
				indices[index] = vectorMatcher.nearestIndex(
					compositedVectors.v0[index]!,
					compositedVectors.v1[index]!,
					compositedVectors.v2[index]!
				);
			}
			vectorMatcher.flushCounts();
			recordTiming(caches, 'quantize direct dither+match loop', loopStart);
			return;
		}

		for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
			const alpha = source[offset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			let r = source[offset]!;
			let g = source[offset + 1]!;
			let b = source[offset + 2]!;
			if (alphaMode !== 'preserve' && alpha !== 255) {
				const opacity = alpha / 255;
				if (alphaMode === 'premultiplied') {
					r = Math.round(r * opacity);
					g = Math.round(g * opacity);
					b = Math.round(b * opacity);
				} else {
					const background = 1 - opacity;
					r = Math.round(r * opacity + matte.r * background);
					g = Math.round(g * opacity + matte.g * background);
					b = Math.round(b * opacity + matte.b * background);
				}
			}
			nearestRgbCount++;
			indices[index] = matcher.nearestIndexByteRgb(r, g, b);
		}
		caches?.recordCount?.('nearest rgb', nearestRgbCount);
		recordTiming(caches, 'quantize direct dither+match loop', loopStart);
		return;
	}

	for (let y = 0; y < image.height; y++) {
		const rowOffset = y * image.width;
		const bayerRow = bayerSize ? (y & bayerMask) * bayerSize : 0;
		for (let x = 0; x < image.width; x++) {
			const index = rowOffset + x;
			const offset = index * 4;
			const alpha = source[offset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			let r = source[offset]!;
			let g = source[offset + 1]!;
			let b = source[offset + 2]!;
			if (alphaMode !== 'preserve' && alpha !== 255) {
				const opacity = alpha / 255;
				if (alphaMode === 'premultiplied') {
					r = Math.round(r * opacity);
					g = Math.round(g * opacity);
					b = Math.round(b * opacity);
				} else {
					const background = 1 - opacity;
					r = Math.round(r * opacity + matte.r * background);
					g = Math.round(g * opacity + matte.g * background);
					b = Math.round(b * opacity + matte.b * background);
				}
			}

			if (useDirectVectorMatch && compositedVectors) {
				indices[index] = vectorMatcher.nearestIndex(
					compositedVectors.v0[index]!,
					compositedVectors.v1[index]!,
					compositedVectors.v2[index]!
				);
				continue;
			}

			if (useBayer && bayer && bayerSize) {
				const thresholdIndex = bayerRow + (x & bayerMask);
				if (useVectorDither && thresholdVectorMatcher) {
					indices[index] = thresholdVectorMatcher.nearestIndexRgb(r, g, b, thresholdIndex);
					continue;
				}
				const threshold = bayer[thresholdIndex]!;
				const mask = useAdaptivePlacement
					? placementMask(
							source,
							image.width,
							image.height,
							x,
							y,
							settings,
							vectorSpace,
							sourceVectors
						)
					: 1;
				if (useVectorDither) {
					const ditherStrength = strength * mask;
					const match = compositedVectors
						? colorSpaceThresholdIndexValues(
								compositedVectors.v0[index]!,
								compositedVectors.v1[index]!,
								compositedVectors.v2[index]!,
								threshold,
								vectorSpace,
								vectorMatcher,
								ditherStrength
							)
						: colorSpaceThresholdIndexRgb(
								r,
								g,
								b,
								threshold,
								vectorSpace,
								vectorMatcher,
								settings,
								ditherStrength
							);
					indices[index] = match === -1 ? matcher.nearestIndexByteRgb(r, g, b) : match;
					continue;
				}
				r = clampByte(r + threshold * noiseScale * mask);
				g = clampByte(g + threshold * noiseScale * mask);
				b = clampByte(b + threshold * noiseScale * mask);
			} else if (useRandom) {
				const mask = useAdaptivePlacement
					? placementMask(
							source,
							image.width,
							image.height,
							x,
							y,
							settings,
							vectorSpace,
							sourceVectors
						)
					: 1;
				randomState = (randomState + 0x6d2b79f5) >>> 0;
				let randomValue = randomState;
				randomValue = Math.imul(randomValue ^ (randomValue >>> 15), randomValue | 1);
				randomValue ^= randomValue + Math.imul(randomValue ^ (randomValue >>> 7), randomValue | 61);
				const noise = ((randomValue ^ (randomValue >>> 14)) >>> 0) / 4294967296 - 0.5;
				if (useVectorDither) {
					const ditherStrength = strength * mask;
					const match = compositedVectors
						? colorSpaceThresholdIndexValues(
								compositedVectors.v0[index]!,
								compositedVectors.v1[index]!,
								compositedVectors.v2[index]!,
								noise,
								vectorSpace,
								vectorMatcher,
								ditherStrength
							)
						: colorSpaceThresholdIndexRgb(
								r,
								g,
								b,
								noise,
								vectorSpace,
								vectorMatcher,
								settings,
								ditherStrength
							);
					indices[index] = match === -1 ? matcher.nearestIndexByteRgb(r, g, b) : match;
					continue;
				}
				r = clampByte(r + noise * noiseScale * mask);
				g = clampByte(g + noise * noiseScale * mask);
				b = clampByte(b + noise * noiseScale * mask);
			}
			nearestRgbCount++;
			indices[index] = matcher.nearestIndexByteRgb(r, g, b);
		}
	}
	caches?.recordCount?.('nearest rgb', nearestRgbCount);
	vectorMatcher.flushCounts();
	thresholdVectorMatcher?.flushCounts();
	recordTiming(caches, 'quantize direct dither+match loop', loopStart);
}

function quantizeVectorErrorDiffusion(
	image: ImageData,
	indices: Uint8Array,
	matcher: ReturnType<typeof createPaletteMatcher>,
	settings: ProcessingSettings,
	matte: Rgb,
	transparentIndexValue: number,
	fallbackTransparentIndex: number,
	strength: number,
	paletteCacheKey: string,
	caches?: QuantizeCaches
) {
	const kernel = errorKernelForAlgorithm(settings.dither.algorithm);
	if (!kernel)
		throw new Error(`Unsupported error diffusion algorithm: ${settings.dither.algorithm}`);
	const width = image.width;
	const height = image.height;
	const source = image.data;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const initStart = performance.now();
	const work = new Float32Array(width * height * 3);
	const vectorSpace = paletteVectorSpace(matcher, settings, paletteCacheKey, caches);
	const vectorMatcher = createPaletteVectorMatcher(vectorSpace, settings, caches);
	const compositedVectors = cachedColorVectorImage(image, settings, matte, 'composited', caches);
	const useAdaptivePlacement = usesAdaptivePlacement(settings);
	const sourceVectors = useAdaptivePlacement
		? cachedColorVectorImage(image, settings, matte, 'source', caches)
		: undefined;

	for (let index = 0; index < width * height; index++) {
		const sourceOffset = index * 4;
		const workOffset = index * 3;
		if (compositedVectors) {
			work[workOffset] = compositedVectors.v0[index]!;
			work[workOffset + 1] = compositedVectors.v1[index]!;
			work[workOffset + 2] = compositedVectors.v2[index]!;
		} else {
			const vector = vectorForCompositedPixel(source, sourceOffset, settings, matte);
			work[workOffset] = vector[0];
			work[workOffset + 1] = vector[1];
			work[workOffset + 2] = vector[2];
		}
	}
	recordTiming(caches, 'quantize vector diffusion work init', initStart);
	const loopStart = performance.now();

	for (let y = 0; y < height; y++) {
		const reverse = settings.dither.serpentine && y % 2 === 1;
		const start = reverse ? width - 1 : 0;
		const end = reverse ? -1 : width;
		const step = reverse ? -1 : 1;
		for (let x = start; x !== end; x += step) {
			const index = y * width + x;
			const sourceOffset = index * 4;
			const alpha = source[sourceOffset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			const workOffset = index * 3;
			const current0 = work[workOffset]!;
			const current1 = work[workOffset + 1]!;
			const current2 = work[workOffset + 2]!;
			const ordinal = vectorMatcher.nearestOrdinal(current0, current1, current2);
			if (ordinal === -1) throw new Error('No visible palette colors are enabled');
			indices[index] = vectorMatcher.indexForOrdinal(ordinal);
			const mask = useAdaptivePlacement
				? placementMask(source, width, height, x, y, settings, vectorSpace, sourceVectors)
				: 1;
			const error0 = current0 - vectorMatcher.vector0ForOrdinal(ordinal);
			const error1 = current1 - vectorMatcher.vector1ForOrdinal(ordinal);
			const error2 = current2 - vectorMatcher.vector2ForOrdinal(ordinal);
			if (settings.dither.algorithm === 'floyd-steinberg') {
				const strengthMask = strength * mask;
				if (reverse) {
					if (x > 0) {
						const target = workOffset - 3;
						const scaledWeight = (7 / 16) * strengthMask;
						work[target] += error0 * scaledWeight;
						work[target + 1] += error1 * scaledWeight;
						work[target + 2] += error2 * scaledWeight;
					}
					if (y + 1 < height) {
						const nextRow = workOffset + width * 3;
						if (x + 1 < width) {
							const target = nextRow + 3;
							const scaledWeight = (3 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
						{
							const target = nextRow;
							const scaledWeight = (5 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
						if (x > 0) {
							const target = nextRow - 3;
							const scaledWeight = (1 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
					}
				} else {
					if (x + 1 < width) {
						const target = workOffset + 3;
						const scaledWeight = (7 / 16) * strengthMask;
						work[target] += error0 * scaledWeight;
						work[target + 1] += error1 * scaledWeight;
						work[target + 2] += error2 * scaledWeight;
					}
					if (y + 1 < height) {
						const nextRow = workOffset + width * 3;
						if (x > 0) {
							const target = nextRow - 3;
							const scaledWeight = (3 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
						{
							const target = nextRow;
							const scaledWeight = (5 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
						if (x + 1 < width) {
							const target = nextRow + 3;
							const scaledWeight = (1 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
					}
				}
			} else {
				for (const [dxBase, dy, weight] of kernel) {
					const dx = reverse ? -dxBase : dxBase;
					const xx = x + dx;
					const yy = y + dy;
					if (xx < 0 || xx >= width || yy < 0 || yy >= height) continue;
					const target = (yy * width + xx) * 3;
					const scaledWeight = weight * strength * mask;
					work[target] += error0 * scaledWeight;
					work[target + 1] += error1 * scaledWeight;
					work[target + 2] += error2 * scaledWeight;
				}
			}
		}
	}
	vectorMatcher.flushCounts();
	recordTiming(caches, 'quantize vector diffusion dither+match loop', loopStart);
}

function quantizeErrorDiffusion(
	image: ImageData,
	indices: Uint8Array,
	matcher: ReturnType<typeof createPaletteMatcher>,
	settings: ProcessingSettings,
	matte: Rgb,
	transparentIndexValue: number,
	fallbackTransparentIndex: number,
	strength: number,
	paletteCacheKey: string,
	caches?: QuantizeCaches
) {
	if (supportsVectorDither(settings)) {
		quantizeVectorErrorDiffusion(
			image,
			indices,
			matcher,
			settings,
			matte,
			transparentIndexValue,
			fallbackTransparentIndex,
			strength,
			paletteCacheKey,
			caches
		);
		return;
	}

	const kernel = errorKernelForAlgorithm(settings.dither.algorithm);
	if (!kernel)
		throw new Error(`Unsupported error diffusion algorithm: ${settings.dither.algorithm}`);
	const width = image.width;
	const height = image.height;
	const source = image.data;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const initStart = performance.now();
	const work = new Float32Array(width * height * 3);
	const vectorSpace = paletteVectorSpace(matcher, settings, paletteCacheKey, caches);
	const useAdaptivePlacement = usesAdaptivePlacement(settings);
	const sourceVectors = useAdaptivePlacement
		? cachedColorVectorImage(image, settings, matte, 'source', caches)
		: undefined;

	for (
		let index = 0, sourceOffset = 0, workOffset = 0;
		index < width * height;
		index++, sourceOffset += 4, workOffset += 3
	) {
		const alpha = source[sourceOffset + 3]!;
		let r = source[sourceOffset]!;
		let g = source[sourceOffset + 1]!;
		let b = source[sourceOffset + 2]!;
		if (alphaMode !== 'preserve' && alpha !== 255) {
			const opacity = alpha / 255;
			if (alphaMode === 'premultiplied') {
				r = Math.round(r * opacity);
				g = Math.round(g * opacity);
				b = Math.round(b * opacity);
			} else {
				const background = 1 - opacity;
				r = Math.round(r * opacity + matte.r * background);
				g = Math.round(g * opacity + matte.g * background);
				b = Math.round(b * opacity + matte.b * background);
			}
		}
		work[workOffset] = r;
		work[workOffset + 1] = g;
		work[workOffset + 2] = b;
	}
	recordTiming(caches, 'quantize rgb diffusion work init', initStart);
	let nearestRgbCount = 0;
	const loopStart = performance.now();

	for (let y = 0; y < height; y++) {
		const reverse = settings.dither.serpentine && y % 2 === 1;
		const start = reverse ? width - 1 : 0;
		const end = reverse ? -1 : width;
		const step = reverse ? -1 : 1;
		for (let x = start; x !== end; x += step) {
			const index = y * width + x;
			const sourceOffset = index * 4;
			const alpha = source[sourceOffset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			const workOffset = index * 3;
			const r = clampByte(work[workOffset]!);
			const g = clampByte(work[workOffset + 1]!);
			const b = clampByte(work[workOffset + 2]!);
			nearestRgbCount++;
			const match = matcher.nearestIndexByteRgb(r, g, b);
			indices[index] = match;
			const mask = useAdaptivePlacement
				? placementMask(source, width, height, x, y, settings, vectorSpace, sourceVectors)
				: 1;
			const errorR = r - matcher.paletteRedAt(match);
			const errorG = g - matcher.paletteGreenAt(match);
			const errorB = b - matcher.paletteBlueAt(match);
			if (settings.dither.algorithm === 'floyd-steinberg') {
				const strengthMask = strength * mask;
				if (reverse) {
					if (x > 0) {
						const target = workOffset - 3;
						const scaledWeight = (7 / 16) * strengthMask;
						work[target] += errorR * scaledWeight;
						work[target + 1] += errorG * scaledWeight;
						work[target + 2] += errorB * scaledWeight;
					}
					if (y + 1 < height) {
						const nextRow = workOffset + width * 3;
						if (x + 1 < width) {
							const target = nextRow + 3;
							const scaledWeight = (3 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
						{
							const target = nextRow;
							const scaledWeight = (5 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
						if (x > 0) {
							const target = nextRow - 3;
							const scaledWeight = (1 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
					}
				} else {
					if (x + 1 < width) {
						const target = workOffset + 3;
						const scaledWeight = (7 / 16) * strengthMask;
						work[target] += errorR * scaledWeight;
						work[target + 1] += errorG * scaledWeight;
						work[target + 2] += errorB * scaledWeight;
					}
					if (y + 1 < height) {
						const nextRow = workOffset + width * 3;
						if (x > 0) {
							const target = nextRow - 3;
							const scaledWeight = (3 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
						{
							const target = nextRow;
							const scaledWeight = (5 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
						if (x + 1 < width) {
							const target = nextRow + 3;
							const scaledWeight = (1 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
					}
				}
			} else {
				for (const [dxBase, dy, weight] of kernel) {
					const dx = reverse ? -dxBase : dxBase;
					const xx = x + dx;
					const yy = y + dy;
					if (xx < 0 || xx >= width || yy < 0 || yy >= height) continue;
					const target = (yy * width + xx) * 3;
					const scaledWeight = weight * strength * mask;
					work[target] += errorR * scaledWeight;
					work[target + 1] += errorG * scaledWeight;
					work[target + 2] += errorB * scaledWeight;
				}
			}
		}
	}
	caches?.recordCount?.('nearest rgb', nearestRgbCount);
	recordTiming(caches, 'quantize rgb diffusion dither+match loop', loopStart);
}
