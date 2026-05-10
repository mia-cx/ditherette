import { bayerSizeForAlgorithm, normalizedBayerMatrix } from './bayer';
import { clampByte, createPaletteMatcher, vectorForRgb } from './color';
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
	| 'palette vector space'
	| 'color vector image lookup'
	| 'color vector image build'
	| 'direct loop'
	| 'error diffusion init'
	| 'error diffusion loop';

export type QuantizeDiagnosticsSink = {
	recordTiming?(name: QuantizeTimingName, ms: number): void;
	recordCount?(name: QuantizeCounterName, amount?: number): void;
};

type PaletteVectorMatcher = {
	nearest(v0: number, v1: number, v2: number): PaletteVector | undefined;
	nearestIndex(v0: number, v1: number, v2: number): number;
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

export type QuantizeResult = {
	indices: Uint8Array;
	palette: EnabledPaletteColor[];
	transparentIndex: number;
	warnings: string[];
};

function mulberry32(seed: number) {
	let state = seed >>> 0;
	return () => {
		state += 0x6d2b79f5;
		let value = state;
		value = Math.imul(value ^ (value >>> 15), value | 1);
		value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
		return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
	};
}

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
	const timingStart = performance.now();
	const cached = cacheKey ? caches?.getPaletteVectorSpace?.(cacheKey) : undefined;
	if (cached) {
		recordTiming(caches, 'palette vector space', timingStart);
		return cached;
	}

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
	if (cacheKey) caches?.setPaletteVectorSpace?.(cacheKey, space);
	recordTiming(caches, 'palette vector space', timingStart);
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
		const hue = Math.atan2(Math.sin(left2 - right[2]), Math.cos(left2 - right[2]));
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
	let lastV0 = Number.NaN;
	let lastV1 = Number.NaN;
	let lastV2 = Number.NaN;
	let lastMatch: PaletteVector | undefined;

	function nearest(v0: number, v1: number, v2: number) {
		if (v0 === lastV0 && v1 === lastV1 && v2 === lastV2) {
			caches?.recordCount?.('vector memo hit');
			return lastMatch;
		}
		caches?.recordCount?.('vector memo miss');
		caches?.recordCount?.('nearest vector');

		let winner: PaletteVector | undefined;
		let best = Infinity;
		for (const candidate of space.colors) {
			const distance = vectorDistanceValues(v0, v1, v2, candidate.vector, settings);
			if (distance < best) {
				best = distance;
				winner = candidate;
			}
		}

		lastV0 = v0;
		lastV1 = v1;
		lastV2 = v2;
		lastMatch = winner;
		caches?.recordCount?.('vector memo set');
		return winner;
	}

	return {
		nearest,
		nearestIndex(v0, v1, v2) {
			return nearest(v0, v1, v2)?.index ?? -1;
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
	for (let index = 0; index < pixels; index++) {
		const offset = index * 4;
		const rgb =
			kind === 'composited'
				? compositedRgb(
						{ r: source[offset]!, g: source[offset + 1]!, b: source[offset + 2]! },
						source[offset + 3]!,
						settings.output.alphaMode,
						matte
					)
				: { r: source[offset]!, g: source[offset + 1]!, b: source[offset + 2]! };
		const vector = vectorForRgb(rgb.r, rgb.g, rgb.b, settings.colorSpace);
		v0[index] = vector[0];
		v1[index] = vector[1];
		v2[index] = vector[2];
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
	const lookupStart = performance.now();
	const cached = caches?.getColorVectorImage?.(key);
	recordTiming(caches, 'color vector image lookup', lookupStart);
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
	recordTiming(caches, 'color vector image build', buildStart);
	caches.setColorVectorImage(key, vectors, bytes);
	return caches.getColorVectorImage?.(key) ?? vectors;
}

function colorSpaceThresholdIndex(
	rgb: Rgb,
	threshold: number,
	space: PaletteVectorSpace,
	matcher: PaletteVectorMatcher,
	settings: ProcessingSettings,
	strength: number,
	sourceVector?: ColorVector
) {
	if (strength <= 0) return -1;
	const source = sourceVector ?? vectorForRgb(rgb.r, rgb.g, rgb.b, settings.colorSpace);
	const amount = threshold * strength * COLOR_SPACE_THRESHOLD_SCALE;
	return matcher.nearestIndex(
		source[0] + amount * space.ranges[0],
		source[1] + amount * space.ranges[1],
		source[2] + amount * space.ranges[2]
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

export function quantizeImage(
	image: ImageData,
	palette: EnabledPaletteColor[],
	settings: ProcessingSettings,
	caches?: QuantizeCaches
): QuantizeResult {
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
			indices: new Uint8Array(image.width * image.height).fill(tIndex),
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
	const matcher = createPaletteMatcher(nextPalette, settings.colorSpace);
	recordTiming(caches, 'matcher build', matcherStart);
	const paletteCacheKey = paletteVectorCacheKey(nextPalette, settings.colorSpace);
	const pixels = image.width * image.height;
	const indices = new Uint8Array(pixels);
	const strength = settings.dither.strength / 100;
	const fallbackTransparentIndex = fallbackTransparent
		? nextPalette.indexOf(fallbackTransparent)
		: -1;

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
	const random = mulberry32(settings.dither.seed);
	const bayerSize = bayerSizeForAlgorithm(settings.dither.algorithm);
	const bayer = bayerSize ? normalizedBayerMatrix(bayerSize) : undefined;
	const source = image.data;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const useBayer = Boolean(bayer && bayerSize && strength > 0);
	const useRandom = settings.dither.algorithm === 'random' && strength > 0;
	const useVectorDither = supportsVectorDither(settings);
	const noiseScale = RGB_DITHER_NOISE_SCALE * strength;
	const vectorSpace = paletteVectorSpace(matcher, settings, paletteCacheKey, caches);
	const vectorMatcher = createPaletteVectorMatcher(vectorSpace, settings, caches);
	const needsCompositedVectors =
		supportsCachedVectorMatching(settings.colorSpace) &&
		((!useBayer && !useRandom) || useVectorDither);
	const compositedVectors = needsCompositedVectors
		? cachedColorVectorImage(image, settings, matte, 'composited', caches)
		: undefined;
	const sourceVectors =
		(useBayer || useRandom) && usesAdaptivePlacement(settings)
			? cachedColorVectorImage(image, settings, matte, 'source', caches)
			: undefined;
	const useDirectVectorMatch = Boolean(compositedVectors && !useBayer && !useRandom);
	const loopStart = performance.now();

	for (let y = 0; y < image.height; y++) {
		const rowOffset = y * image.width;
		const bayerRow = bayerSize ? (y % bayerSize) * bayerSize : 0;
		for (let x = 0; x < image.width; x++) {
			const index = rowOffset + x;
			const offset = index * 4;
			const alpha = source[offset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			let { r, g, b } = compositedRgb(
				{ r: source[offset]!, g: source[offset + 1]!, b: source[offset + 2]! },
				alpha,
				alphaMode,
				matte
			);

			if (useDirectVectorMatch && compositedVectors) {
				indices[index] = vectorMatcher.nearestIndex(
					compositedVectors.v0[index]!,
					compositedVectors.v1[index]!,
					compositedVectors.v2[index]!
				);
				continue;
			}

			if (useBayer && bayer && bayerSize) {
				const mask = placementMask(
					source,
					image.width,
					image.height,
					x,
					y,
					settings,
					vectorSpace,
					sourceVectors
				);
				const threshold = bayer[bayerRow + (x % bayerSize)]!;
				if (useVectorDither) {
					const match = colorSpaceThresholdIndex(
						{ r, g, b },
						threshold,
						vectorSpace,
						vectorMatcher,
						settings,
						strength * mask,
						compositedVectors ? cachedVectorAt(compositedVectors, index) : undefined
					);
					indices[index] = match === -1 ? matcher.nearestIndexRgb(r, g, b) : match;
					continue;
				}
				r = clampByte(r + threshold * noiseScale * mask);
				g = clampByte(g + threshold * noiseScale * mask);
				b = clampByte(b + threshold * noiseScale * mask);
			} else if (useRandom) {
				const mask = placementMask(
					source,
					image.width,
					image.height,
					x,
					y,
					settings,
					vectorSpace,
					sourceVectors
				);
				const noise = random() - 0.5;
				if (useVectorDither) {
					const match = colorSpaceThresholdIndex(
						{ r, g, b },
						noise,
						vectorSpace,
						vectorMatcher,
						settings,
						strength * mask,
						compositedVectors ? cachedVectorAt(compositedVectors, index) : undefined
					);
					indices[index] = match === -1 ? matcher.nearestIndexRgb(r, g, b) : match;
					continue;
				}
				r = clampByte(r + noise * noiseScale * mask);
				g = clampByte(g + noise * noiseScale * mask);
				b = clampByte(b + noise * noiseScale * mask);
			}
			caches?.recordCount?.('nearest rgb');
			indices[index] = matcher.nearestIndexRgb(r, g, b);
		}
	}
	recordTiming(caches, 'direct loop', loopStart);
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
	const sourceVectors = usesAdaptivePlacement(settings)
		? cachedColorVectorImage(image, settings, matte, 'source', caches)
		: undefined;

	for (let index = 0; index < width * height; index++) {
		const sourceOffset = index * 4;
		const workOffset = index * 3;
		const vector = compositedVectors
			? cachedVectorAt(compositedVectors, index)
			: vectorForCompositedPixel(source, sourceOffset, settings, matte);
		work[workOffset] = vector[0];
		work[workOffset + 1] = vector[1];
		work[workOffset + 2] = vector[2];
	}
	recordTiming(caches, 'error diffusion init', initStart);
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
			const match = vectorMatcher.nearest(current0, current1, current2);
			if (!match) throw new Error('No visible palette colors are enabled');
			indices[index] = match.index;
			const mask = placementMask(source, width, height, x, y, settings, vectorSpace, sourceVectors);
			const error0 = current0 - match.vector[0];
			const error1 = current1 - match.vector[1];
			const error2 = current2 - match.vector[2];
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
	recordTiming(caches, 'error diffusion loop', loopStart);
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
	const sourceVectors = usesAdaptivePlacement(settings)
		? cachedColorVectorImage(image, settings, matte, 'source', caches)
		: undefined;

	for (let index = 0; index < width * height; index++) {
		const sourceOffset = index * 4;
		const workOffset = index * 3;
		const alpha = source[sourceOffset + 3]!;
		const { r, g, b } = compositedRgb(
			{ r: source[sourceOffset]!, g: source[sourceOffset + 1]!, b: source[sourceOffset + 2]! },
			alpha,
			alphaMode,
			matte
		);
		work[workOffset] = r;
		work[workOffset + 1] = g;
		work[workOffset + 2] = b;
	}
	recordTiming(caches, 'error diffusion init', initStart);
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
			caches?.recordCount?.('nearest rgb');
			const match = matcher.nearestIndexRgb(r, g, b);
			indices[index] = match;
			const chosen = matcher.paletteRgbAt(match);
			const mask = placementMask(source, width, height, x, y, settings, vectorSpace, sourceVectors);
			const errorR = r - chosen.r;
			const errorG = g - chosen.g;
			const errorB = b - chosen.b;
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
	recordTiming(caches, 'error diffusion loop', loopStart);
}
