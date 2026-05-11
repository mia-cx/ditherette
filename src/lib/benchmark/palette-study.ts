import { WPLACE_PALETTE } from '$lib/palette/wplace';
import { bayerSizeForAlgorithm, normalizedBayerMatrix } from '$lib/processing/bayer';
import {
	WEIGHTED_RGB_601_B,
	WEIGHTED_RGB_601_G,
	WEIGHTED_RGB_601_R,
	WEIGHTED_RGB_709_B,
	WEIGHTED_RGB_709_G,
	WEIGHTED_RGB_709_R
} from '$lib/processing/quantize-shared';
import {
	createPaletteMatcher,
	vectorForRgb,
	type PaletteMatcherOptions
} from '$lib/processing/color';
import type { ColorSpaceId, DitherId, EnabledPaletteColor } from '$lib/processing/types';

export type PaletteStudyId =
	| 'direct-byte-rgb'
	| 'bayer-additive-byte-rgb'
	| 'bayer-threshold-vector'
	| 'random-vector'
	| 'diffusion-trace'
	| 'diffusion-kernel-only'
	| 'cache-study'
	| 'matcher';

export type PaletteStudyVariant =
	| 'scan'
	| 'distance-tables'
	| 'dense-rgb-distance-tables'
	| 'generic-kernel'
	| 'unrolled-kernel'
	| 'rolling-row-kernel'
	| 'vp-tree'
	| 'ball-tree'
	| 'previous-verify'
	| 'threshold-scan'
	| 'threshold-direct-cache'
	| 'threshold-direct-cache-23'
	| 'threshold-direct-cache-24'
	| 'threshold-probe-4'
	| 'threshold-unique-map'
	| 'threshold-unique-map-hot';

export type PaletteStudySource = {
	id: string;
	label: string;
	imageData: ImageData;
	path?: string;
	decodeMs?: number;
};

export type PaletteStudyOptions = {
	sources: readonly PaletteStudySource[];
	iterations?: number;
	warmups?: number;
	studies?: readonly PaletteStudyId[];
	variants?: readonly PaletteStudyVariant[];
	colorSpaces?: readonly ColorSpaceId[];
	dithers?: readonly DitherId[];
};

export type PaletteStudyRow = {
	study: PaletteStudyId;
	source: string;
	pixels: number;
	paletteColors: number;
	dither: DitherId | 'none';
	colorSpace: ColorSpaceId;
	variant: PaletteStudyVariant;
	buildMs: number;
	loopMs: number;
	totalMs: number;
	candidateEvaluations: number;
	queries: number;
	uniqueKeys: number;
	cacheHits: number;
	cacheMisses: number;
	cacheCollisions: number;
	cacheSets: number;
	cacheBytes: number;
	tableBytes: number;
	workBytes: number;
	checksum: string;
	matchesBaseline: boolean;
	notes: string;
};

export type PaletteStudyResult = {
	version: 1;
	runAt: string;
	iterations: number;
	warmups: number;
	rows: PaletteStudyRow[];
};

const COLOR_SPACES: readonly ColorSpaceId[] = [
	'oklab',
	'srgb',
	'linear-rgb',
	'weighted-rgb',
	'weighted-rgb-601',
	'weighted-rgb-709',
	'cielab',
	'oklch'
];

const DEFAULT_STUDIES: readonly PaletteStudyId[] = ['direct-byte-rgb'];
const DEFAULT_VARIANTS: readonly PaletteStudyVariant[] = [
	'scan',
	'distance-tables',
	'dense-rgb-distance-tables'
];
const DEFAULT_DIFFUSION_DITHERS: readonly DitherId[] = ['floyd-steinberg', 'sierra', 'sierra-lite'];
const DIFFUSION_KERNEL_VARIANTS: readonly PaletteStudyVariant[] = [
	'generic-kernel',
	'unrolled-kernel',
	'rolling-row-kernel'
];
const DIFFUSION_TRACE_VARIANTS: readonly PaletteStudyVariant[] = [
	'scan',
	'vp-tree',
	'ball-tree',
	'previous-verify'
];
const BAYER_THRESHOLD_VARIANTS: readonly PaletteStudyVariant[] = [
	'threshold-scan',
	'threshold-direct-cache',
	'threshold-direct-cache-23',
	'threshold-direct-cache-24',
	'threshold-probe-4',
	'threshold-unique-map',
	'threshold-unique-map-hot'
];
const RGB24_SIZE = 256 * 256 * 256;
const FNV_OFFSET = 0x811c9dc5;
const FNV_PRIME = 0x01000193;
const STUDY_DITHER_STRENGTH = 100;
const COLOR_SPACE_THRESHOLD_SCALE = 0.25;

export function runPaletteStudy(options: PaletteStudyOptions): PaletteStudyResult {
	const iterations = Math.max(1, Math.floor(options.iterations ?? 3));
	const warmups = Math.max(0, Math.floor(options.warmups ?? 1));
	const studies = options.studies?.length ? options.studies : DEFAULT_STUDIES;
	const variants = options.variants?.length ? options.variants : DEFAULT_VARIANTS;
	const colorSpaces = options.colorSpaces?.length ? options.colorSpaces : COLOR_SPACES;
	const rows: PaletteStudyRow[] = [];

	for (const source of options.sources) {
		for (const study of studies) {
			if (study === 'bayer-threshold-vector') {
				const dithers: readonly DitherId[] = options.dithers?.length
					? options.dithers
					: ['bayer-8', 'bayer-16'];
				const thresholdVariants = options.variants?.length
					? options.variants
					: BAYER_THRESHOLD_VARIANTS;
				for (const dither of dithers) {
					if (!bayerSizeForAlgorithm(dither)) continue;
					for (const colorSpace of colorSpaces) {
						if (!supportsBayerThresholdStudy(colorSpace)) continue;
						const dataset = bayerThresholdDataset(source.imageData, dither);
						const baselines = new Map<ColorSpaceId, string>();
						for (const variant of thresholdVariants) {
							if (!BAYER_THRESHOLD_VARIANTS.includes(variant)) continue;
							const runner = isPersistentThresholdVariant(variant)
								? runPersistentBayerThresholdStudy
								: runBayerThresholdStudy;
							for (let warmup = 0; warmup < warmups; warmup++) {
								runner(source, dataset, colorSpace, dither, variant);
							}
							const runs = Array.from({ length: iterations }, () =>
								runner(source, dataset, colorSpace, dither, variant)
							);
							const row = meanDirectRows(runs);
							const baseline = baselines.get(colorSpace) ?? row.checksum;
							baselines.set(colorSpace, baseline);
							rows.push({ ...row, matchesBaseline: row.checksum === baseline });
						}
					}
				}
				continue;
			}
			if (study === 'diffusion-kernel-only') {
				const dithers = options.dithers?.length ? options.dithers : DEFAULT_DIFFUSION_DITHERS;
				const kernelVariants = options.variants?.length
					? options.variants
					: DIFFUSION_KERNEL_VARIANTS;
				const baselines = new Map<DitherId, string>();
				for (const dither of dithers) {
					if (!DEFAULT_DIFFUSION_DITHERS.includes(dither)) continue;
					for (const variant of kernelVariants) {
						if (
							variant !== 'generic-kernel' &&
							variant !== 'unrolled-kernel' &&
							variant !== 'rolling-row-kernel'
						)
							continue;
						for (let warmup = 0; warmup < warmups; warmup++) {
							runDiffusionKernelStudy(source, dither, variant);
						}
						const runs = Array.from({ length: iterations }, () =>
							runDiffusionKernelStudy(source, dither, variant)
						);
						const row = meanDirectRows(runs);
						const baseline = baselines.get(dither) ?? row.checksum;
						baselines.set(dither, baseline);
						rows.push({ ...row, matchesBaseline: row.checksum === baseline });
					}
				}
				continue;
			}
			if (study === 'diffusion-trace' || study === 'matcher') {
				const dithers = options.dithers?.length ? options.dithers : DEFAULT_DIFFUSION_DITHERS;
				const traceVariants = options.variants?.length
					? options.variants
					: DIFFUSION_TRACE_VARIANTS;
				for (const dither of dithers) {
					if (!DEFAULT_DIFFUSION_DITHERS.includes(dither)) continue;
					for (const colorSpace of colorSpaces) {
						const trace = buildDiffusionTrace(source, colorSpace, dither);
						for (const variant of traceVariants) {
							if (!DIFFUSION_TRACE_VARIANTS.includes(variant)) continue;
							for (let warmup = 0; warmup < warmups; warmup++) {
								runDiffusionTraceStudy(source, trace, colorSpace, dither, variant, study);
							}
							const runs = Array.from({ length: iterations }, () =>
								runDiffusionTraceStudy(source, trace, colorSpace, dither, variant, study)
							);
							rows.push(meanDirectRows(runs));
						}
					}
				}
				continue;
			}
			if (study !== 'direct-byte-rgb' && study !== 'cache-study') {
				rows.push(...unsupportedRows(source, study, variants, colorSpaces));
				continue;
			}
			const dataset = directByteRgbDataset(source.imageData);
			const baselineChecksums = new Map<ColorSpaceId, string>();
			for (const colorSpace of colorSpaces) {
				for (const variant of variants) {
					for (let warmup = 0; warmup < warmups; warmup++) {
						runDirectByteRgbStudy(source, dataset, colorSpace, variant);
					}
					const runs = Array.from({ length: iterations }, () =>
						runDirectByteRgbStudy(source, dataset, colorSpace, variant)
					);
					const row = meanDirectRows(runs);
					const baseline = baselineChecksums.get(colorSpace) ?? row.checksum;
					baselineChecksums.set(colorSpace, baseline);
					rows.push({
						...row,
						study,
						matchesBaseline: row.checksum === baseline
					});
				}
			}
		}
	}

	return { version: 1, runAt: new Date().toISOString(), iterations, warmups, rows };
}

type DirectByteRgbDataset = {
	pixels: number;
	keys: Uint32Array;
	uniqueKeys: number;
};

function directByteRgbDataset(image: ImageData): DirectByteRgbDataset {
	const pixels = image.width * image.height;
	const keys = new Uint32Array(pixels);
	const seen = new Uint8Array(RGB24_SIZE);
	let uniqueKeys = 0;
	const source = image.data;
	for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
		const key = ((source[offset]! << 16) | (source[offset + 1]! << 8) | source[offset + 2]!) >>> 0;
		keys[index] = key;
		if (!seen[key]) {
			seen[key] = 1;
			uniqueKeys++;
		}
	}
	return { pixels, keys, uniqueKeys };
}

function runDirectByteRgbStudy(
	source: PaletteStudySource,
	dataset: DirectByteRgbDataset,
	colorSpace: ColorSpaceId,
	variant: PaletteStudyVariant
): PaletteStudyRow {
	const palette = enabledBenchmarkPalette();
	const visiblePaletteColors = palette.filter(
		(color) => color.rgb && color.kind !== 'transparent'
	).length;
	const matcherOptions = matcherOptionsForVariant(variant);
	const totalStart = performance.now();
	const buildStart = performance.now();
	const matcher = createPaletteMatcher(palette, colorSpace, matcherOptions);
	const buildMs = performance.now() - buildStart;
	let checksum = 0x811c9dc5;
	const loopStart = performance.now();
	for (let index = 0; index < dataset.keys.length; index++) {
		const key = dataset.keys[index]!;
		const match = matcher.nearestIndexByteRgb(key >>> 16, (key >>> 8) & 255, key & 255);
		checksum ^= match + 1;
		checksum = Math.imul(checksum, 0x01000193) >>> 0;
	}
	const loopMs = performance.now() - loopStart;
	const stats = matcher.memoStats();
	const cacheBytes = variant === 'dense-rgb-distance-tables' ? RGB24_SIZE : 0;
	return {
		study: 'direct-byte-rgb',
		source: source.id,
		pixels: dataset.pixels,
		paletteColors: visiblePaletteColors,
		dither: 'none',
		colorSpace,
		variant,
		buildMs,
		loopMs,
		totalMs: performance.now() - totalStart,
		candidateEvaluations: stats.rgbMisses * visiblePaletteColors,
		queries: dataset.pixels,
		uniqueKeys: dataset.uniqueKeys,
		cacheHits: stats.rgbHits,
		cacheMisses: stats.rgbMisses,
		cacheCollisions: 0,
		cacheSets: stats.rgbSets,
		cacheBytes,
		tableBytes: distanceTableBytes(variant, colorSpace, visiblePaletteColors),
		workBytes: dataset.keys.byteLength,
		checksum: checksum.toString(16).padStart(8, '0'),
		matchesBaseline: true,
		notes: 'native source direct byte-RGB matcher replay'
	};
}

function runDiffusionKernelStudy(
	source: PaletteStudySource,
	dither: DitherId,
	variant: PaletteStudyVariant
): PaletteStudyRow {
	const image = source.imageData;
	const pixels = image.width * image.height;
	const totalStart = performance.now();
	const buildStart = performance.now();
	if (variant === 'rolling-row-kernel') {
		return runRollingRowKernelStudy(source, dither, buildStart, totalStart);
	}
	const work = new Float32Array(pixels * 3);
	for (
		let index = 0, sourceOffset = 0, workOffset = 0;
		index < pixels;
		index++, sourceOffset += 4, workOffset += 3
	) {
		work[workOffset] = image.data[sourceOffset]!;
		work[workOffset + 1] = image.data[sourceOffset + 1]!;
		work[workOffset + 2] = image.data[sourceOffset + 2]!;
	}
	const buildMs = performance.now() - buildStart;
	let checksum = 0x811c9dc5;
	const loopStart = performance.now();
	for (let y = 0; y < image.height; y++) {
		const reverse = y % 2 === 1;
		const start = reverse ? image.width - 1 : 0;
		const end = reverse ? -1 : image.width;
		const step = reverse ? -1 : 1;
		for (let x = start; x !== end; x += step) {
			const workOffset = (y * image.width + x) * 3;
			const current0 = work[workOffset]!;
			const current1 = work[workOffset + 1]!;
			const current2 = work[workOffset + 2]!;
			const error0 = current0 - 96;
			const error1 = current1 - 96;
			const error2 = current2 - 96;
			checksum ^= ((current0 + current1 + current2) | 0) & 255;
			checksum = Math.imul(checksum, 0x01000193) >>> 0;
			if (variant === 'unrolled-kernel') {
				scatterKernelUnrolled(
					work,
					image.width,
					image.height,
					x,
					y,
					workOffset,
					reverse,
					dither,
					error0,
					error1,
					error2
				);
			} else {
				scatterKernelGeneric(
					work,
					image.width,
					image.height,
					x,
					y,
					reverse,
					dither,
					error0,
					error1,
					error2
				);
			}
		}
	}
	const loopMs = performance.now() - loopStart;
	return {
		study: 'diffusion-kernel-only',
		source: source.id,
		pixels,
		paletteColors: 0,
		dither,
		colorSpace: 'srgb',
		variant,
		buildMs,
		loopMs,
		totalMs: performance.now() - totalStart,
		candidateEvaluations: 0,
		queries: pixels,
		uniqueKeys: 0,
		cacheHits: 0,
		cacheMisses: 0,
		cacheCollisions: 0,
		cacheSets: 0,
		cacheBytes: 0,
		tableBytes: 0,
		workBytes: work.byteLength,
		checksum: checksum.toString(16).padStart(8, '0'),
		matchesBaseline: true,
		notes: 'fake nearest-color diffusion scatter/update only'
	};
}

type BayerThresholdDataset = {
	pixels: number;
	thresholds: readonly number[];
	keys: Uint32Array;
	uniqueKeys: number;
};

type ThresholdTerms = {
	palette: VectorPalette;
	term0: Float64Array;
	term1: Float64Array;
	term2: Float64Array;
	thresholdTermBases: Uint32Array;
	thresholdKeyBases: Uint32Array;
	tableBytes: number;
};

type ThresholdStudyMatcher = {
	nearestIndex(key: number): number;
	candidateEvaluations(): number;
	cacheHits(): number;
	cacheMisses(): number;
	cacheCollisions(): number;
	cacheSets(): number;
	cacheBytes(): number;
	notes(): string;
};

function supportsBayerThresholdStudy(colorSpace: ColorSpaceId) {
	return colorSpace === 'weighted-rgb-601' || colorSpace === 'weighted-rgb-709';
}

function isPersistentThresholdVariant(variant: PaletteStudyVariant) {
	return variant === 'threshold-unique-map-hot';
}

function bayerThresholdDataset(image: ImageData, dither: DitherId): BayerThresholdDataset {
	const bayerSize = bayerSizeForAlgorithm(dither);
	if (!bayerSize) throw new Error(`Unsupported Bayer threshold study dither: ${dither}`);
	const thresholds = normalizedBayerMatrix(bayerSize);
	const bayerMask = bayerSize - 1;
	const pixels = image.width * image.height;
	const keys = new Uint32Array(pixels);
	const seen = new Set<number>();
	for (let y = 0; y < image.height; y++) {
		const rowOffset = y * image.width;
		const thresholdRow = (y & bayerMask) * bayerSize;
		for (let x = 0; x < image.width; x++) {
			const index = rowOffset + x;
			const offset = index * 4;
			const thresholdIndex = thresholdRow + (x & bayerMask);
			const key =
				((thresholdIndex << 24) |
					(image.data[offset]! << 16) |
					(image.data[offset + 1]! << 8) |
					image.data[offset + 2]!) >>>
				0;
			keys[index] = key;
			seen.add(key);
		}
	}
	return { pixels, thresholds, keys, uniqueKeys: seen.size };
}

function runBayerThresholdStudy(
	source: PaletteStudySource,
	dataset: BayerThresholdDataset,
	colorSpace: ColorSpaceId,
	dither: DitherId,
	variant: PaletteStudyVariant
): PaletteStudyRow {
	const totalStart = performance.now();
	const buildStart = performance.now();
	const terms = buildThresholdTerms(colorSpace, dataset.thresholds);
	const matcher = createThresholdStudyMatcher(terms, dataset, variant);
	const buildMs = performance.now() - buildStart;
	let checksum = FNV_OFFSET;
	const loopStart = performance.now();
	for (let index = 0; index < dataset.keys.length; index++) {
		const match = matcher.nearestIndex(dataset.keys[index]!);
		checksum ^= match + 1;
		checksum = Math.imul(checksum, FNV_PRIME) >>> 0;
	}
	const loopMs = performance.now() - loopStart;
	return {
		study: 'bayer-threshold-vector',
		source: source.id,
		pixels: dataset.pixels,
		paletteColors: terms.palette.count,
		dither,
		colorSpace,
		variant,
		buildMs,
		loopMs,
		totalMs: performance.now() - totalStart,
		candidateEvaluations: matcher.candidateEvaluations(),
		queries: dataset.pixels,
		uniqueKeys: dataset.uniqueKeys,
		cacheHits: matcher.cacheHits(),
		cacheMisses: matcher.cacheMisses(),
		cacheCollisions: matcher.cacheCollisions(),
		cacheSets: matcher.cacheSets(),
		cacheBytes: matcher.cacheBytes(),
		tableBytes: terms.tableBytes,
		workBytes: dataset.keys.byteLength,
		checksum: checksum.toString(16).padStart(8, '0'),
		matchesBaseline: true,
		notes: matcher.notes()
	};
}

function runPersistentBayerThresholdStudy(
	source: PaletteStudySource,
	dataset: BayerThresholdDataset,
	colorSpace: ColorSpaceId,
	dither: DitherId,
	variant: PaletteStudyVariant
): PaletteStudyRow {
	if (variant !== 'threshold-unique-map-hot') {
		return runBayerThresholdStudy(source, dataset, colorSpace, dither, variant);
	}
	const totalStart = performance.now();
	const buildStart = performance.now();
	const terms = buildThresholdTerms(colorSpace, dataset.thresholds);
	const cache = buildUniqueThresholdCache(terms, dataset.keys);
	const buildMs = performance.now() - buildStart;
	let checksum = FNV_OFFSET;
	const loopStart = performance.now();
	for (let index = 0; index < dataset.keys.length; index++) {
		const match = cache.results.get(dataset.keys[index]!)!;
		checksum ^= match + 1;
		checksum = Math.imul(checksum, FNV_PRIME) >>> 0;
	}
	const loopMs = performance.now() - loopStart;
	return {
		study: 'bayer-threshold-vector',
		source: source.id,
		pixels: dataset.pixels,
		paletteColors: terms.palette.count,
		dither,
		colorSpace,
		variant,
		buildMs,
		loopMs,
		totalMs: performance.now() - totalStart,
		candidateEvaluations: cache.candidateEvaluations,
		queries: dataset.pixels,
		uniqueKeys: dataset.uniqueKeys,
		cacheHits: dataset.pixels,
		cacheMisses: 0,
		cacheCollisions: 0,
		cacheSets: cache.results.size,
		cacheBytes: cache.results.size * 8,
		tableBytes: terms.tableBytes,
		workBytes: dataset.keys.byteLength,
		checksum: checksum.toString(16).padStart(8, '0'),
		matchesBaseline: true,
		notes: 'persistent JS Map threshold+RGB LUT hot replay; build cost is reusable'
	};
}

function buildThresholdTerms(
	colorSpace: ColorSpaceId,
	thresholds: readonly number[]
): ThresholdTerms {
	const palette = vectorPalette(enabledBenchmarkPalette(), colorSpace);
	const count = palette.count;
	const ranges = paletteRanges(palette);
	const termsPerThreshold = count * 256;
	const term0 = new Float64Array(thresholds.length * termsPerThreshold);
	const term1 = new Float64Array(thresholds.length * termsPerThreshold);
	const term2 = new Float64Array(thresholds.length * termsPerThreshold);
	const thresholdTermBases = new Uint32Array(thresholds.length);
	const thresholdKeyBases = new Uint32Array(thresholds.length);
	for (let thresholdIndex = 0; thresholdIndex < thresholds.length; thresholdIndex++) {
		const amount =
			thresholds[thresholdIndex]! * STUDY_DITHER_STRENGTH * COLOR_SPACE_THRESHOLD_SCALE;
		const offset0 = amount * ranges[0];
		const offset1 = amount * ranges[1];
		const offset2 = amount * ranges[2];
		const thresholdBase = thresholdIndex * termsPerThreshold;
		thresholdTermBases[thresholdIndex] = thresholdBase;
		thresholdKeyBases[thresholdIndex] = (thresholdIndex << 24) >>> 0;
		for (let value = 0; value < 256; value++) {
			const base = thresholdBase + value * count;
			const v0 = thresholdByteComponent(value, colorSpace, 0) + offset0;
			const v1 = thresholdByteComponent(value, colorSpace, 1) + offset1;
			const v2 = thresholdByteComponent(value, colorSpace, 2) + offset2;
			for (let ordinal = 0; ordinal < count; ordinal++) {
				const d0 = v0 - palette.v0[ordinal]!;
				const d1 = v1 - palette.v1[ordinal]!;
				const d2 = v2 - palette.v2[ordinal]!;
				term0[base + ordinal] = d0 * d0;
				term1[base + ordinal] = d1 * d1;
				term2[base + ordinal] = d2 * d2;
			}
		}
	}
	return {
		palette,
		term0,
		term1,
		term2,
		thresholdTermBases,
		thresholdKeyBases,
		tableBytes:
			term0.byteLength +
			term1.byteLength +
			term2.byteLength +
			thresholdTermBases.byteLength +
			thresholdKeyBases.byteLength
	};
}

function createThresholdStudyMatcher(
	terms: ThresholdTerms,
	dataset: BayerThresholdDataset,
	variant: PaletteStudyVariant
): ThresholdStudyMatcher {
	switch (variant) {
		case 'threshold-direct-cache':
			return createDirectThresholdMatcher(terms, dataset.pixels, 0);
		case 'threshold-direct-cache-23':
			return createDirectThresholdMatcher(terms, dataset.pixels, 0, 23);
		case 'threshold-direct-cache-24':
			return createDirectThresholdMatcher(terms, dataset.pixels, 0, 24);
		case 'threshold-probe-4':
			return createDirectThresholdMatcher(terms, dataset.pixels, 4);
		case 'threshold-unique-map':
		case 'threshold-unique-map-hot':
			return createUniqueThresholdMatcher(terms, dataset.keys);
		case 'threshold-scan':
		default:
			return createScanThresholdMatcher(terms);
	}
}

function createScanThresholdMatcher(terms: ThresholdTerms): ThresholdStudyMatcher {
	let candidates = 0;
	return {
		nearestIndex(key) {
			candidates += terms.palette.count;
			return thresholdScanIndex(terms, key);
		},
		candidateEvaluations: () => candidates,
		cacheHits: () => 0,
		cacheMisses: () => 0,
		cacheCollisions: () => 0,
		cacheSets: () => 0,
		cacheBytes: () => 0,
		notes: () => 'threshold table scan without memo cache'
	};
}

function createDirectThresholdMatcher(
	terms: ThresholdTerms,
	pixels: number,
	probeLimit: number,
	cacheBitsOverride?: number
): ThresholdStudyMatcher {
	const cacheBits =
		cacheBitsOverride ?? Math.min(22, Math.max(18, Math.ceil(Math.log2(pixels * 2))));
	const cacheSize = 1 << cacheBits;
	const cacheMask = cacheSize - 1;
	const cacheKeys = new Uint32Array(cacheSize);
	const cacheValues = new Uint16Array(cacheSize);
	const cacheValid = new Uint8Array(cacheSize);
	let candidates = 0;
	let hits = 0;
	let misses = 0;
	let collisions = 0;
	let sets = 0;
	return {
		nearestIndex(key) {
			let slot = thresholdCacheSlot(key, cacheMask);
			const probes = probeLimit || 1;
			for (let probe = 0; probe < probes; probe++) {
				if (!cacheValid[slot]) break;
				if (cacheKeys[slot] === key) {
					hits++;
					return cacheValues[slot]!;
				}
				collisions++;
				if (!probeLimit) break;
				slot = (slot + 1) & cacheMask;
			}
			misses++;
			candidates += terms.palette.count;
			const value = thresholdScanIndex(terms, key);
			cacheKeys[slot] = key;
			cacheValues[slot] = value;
			cacheValid[slot] = 1;
			sets++;
			return value;
		},
		candidateEvaluations: () => candidates,
		cacheHits: () => hits,
		cacheMisses: () => misses,
		cacheCollisions: () => collisions,
		cacheSets: () => sets,
		cacheBytes: () => cacheKeys.byteLength + cacheValues.byteLength + cacheValid.byteLength,
		notes: () =>
			probeLimit
				? `threshold direct cache with ${probeLimit} linear probes`
				: 'threshold direct-mapped cache'
	};
}

function createUniqueThresholdMatcher(
	terms: ThresholdTerms,
	keys: Uint32Array
): ThresholdStudyMatcher {
	const { candidateEvaluations, results } = buildUniqueThresholdCache(terms, keys);
	return {
		nearestIndex(key) {
			return results.get(key)!;
		},
		candidateEvaluations: () => candidateEvaluations,
		cacheHits: () => keys.length - results.size,
		cacheMisses: () => results.size,
		cacheCollisions: () => 0,
		cacheSets: () => results.size,
		cacheBytes: () => results.size * 8,
		notes: () => 'JS Map unique-key prepass for threshold+RGB queries'
	};
}

function buildUniqueThresholdCache(terms: ThresholdTerms, keys: Uint32Array) {
	const results = new Map<number, number>();
	let candidateEvaluations = 0;
	for (let index = 0; index < keys.length; index++) {
		const key = keys[index]!;
		if (results.has(key)) continue;
		candidateEvaluations += terms.palette.count;
		results.set(key, thresholdScanIndex(terms, key));
	}
	return { candidateEvaluations, results };
}

function thresholdScanIndex(terms: ThresholdTerms, key: number) {
	const thresholdIndex = key >>> 24;
	const r = (key >>> 16) & 255;
	const g = (key >>> 8) & 255;
	const b = key & 255;
	const count = terms.palette.count;
	const thresholdBase = terms.thresholdTermBases[thresholdIndex]!;
	const base0 = thresholdBase + r * count;
	const base1 = thresholdBase + g * count;
	const base2 = thresholdBase + b * count;
	let winner = -1;
	let best = Infinity;
	for (let ordinal = 0; ordinal < count; ordinal++) {
		const distance =
			terms.term0[base0 + ordinal]! + terms.term1[base1 + ordinal]! + terms.term2[base2 + ordinal]!;
		if (distance < best) {
			best = distance;
			winner = ordinal;
		}
	}
	return terms.palette.indices[winner]!;
}

function thresholdCacheSlot(key: number, mask: number) {
	return (Math.imul(key ^ (key >>> 16), 0x45d9f3b) >>> 0) & mask;
}

function thresholdByteComponent(value: number, colorSpace: ColorSpaceId, channel: 0 | 1 | 2) {
	if (colorSpace === 'weighted-rgb-601') {
		if (channel === 0) return value * WEIGHTED_RGB_601_R;
		if (channel === 1) return value * WEIGHTED_RGB_601_G;
		return value * WEIGHTED_RGB_601_B;
	}
	if (channel === 0) return value * WEIGHTED_RGB_709_R;
	if (channel === 1) return value * WEIGHTED_RGB_709_G;
	return value * WEIGHTED_RGB_709_B;
}

function paletteRanges(palette: VectorPalette): [number, number, number] {
	let min0 = Infinity;
	let min1 = Infinity;
	let min2 = Infinity;
	let max0 = -Infinity;
	let max1 = -Infinity;
	let max2 = -Infinity;
	for (let ordinal = 0; ordinal < palette.count; ordinal++) {
		min0 = Math.min(min0, palette.v0[ordinal]!);
		min1 = Math.min(min1, palette.v1[ordinal]!);
		min2 = Math.min(min2, palette.v2[ordinal]!);
		max0 = Math.max(max0, palette.v0[ordinal]!);
		max1 = Math.max(max1, palette.v1[ordinal]!);
		max2 = Math.max(max2, palette.v2[ordinal]!);
	}
	return [
		Math.max(max0 - min0, Number.EPSILON),
		Math.max(max1 - min1, Number.EPSILON),
		Math.max(max2 - min2, Number.EPSILON)
	];
}

type VectorPalette = {
	indices: Uint8Array;
	v0: Float64Array;
	v1: Float64Array;
	v2: Float64Array;
	count: number;
};

type DiffusionTrace = {
	pixels: number;
	width: number;
	height: number;
	v0: Float32Array;
	v1: Float32Array;
	v2: Float32Array;
	baselineChecksum: string;
	traceBuildMs: number;
	palette: VectorPalette;
};

type TraceMatcher = {
	nearest(v0: number, v1: number, v2: number): number;
	candidateEvaluations(): number;
	cacheHits(): number;
	cacheMisses(): number;
	bytes(): number;
	notes(): string;
};

type BallNode = {
	start: number;
	end: number;
	left: number;
	right: number;
	center0: number;
	center1: number;
	center2: number;
	radius: number;
};

type VpNode = {
	ordinal: number;
	left: number;
	right: number;
	threshold: number;
};

function buildDiffusionTrace(
	source: PaletteStudySource,
	colorSpace: ColorSpaceId,
	dither: DitherId
): DiffusionTrace {
	const start = performance.now();
	const image = source.imageData;
	const pixels = image.width * image.height;
	const palette = vectorPalette(enabledBenchmarkPalette(), colorSpace);
	const work0 = new Float32Array(pixels);
	const work1 = new Float32Array(pixels);
	const work2 = new Float32Array(pixels);
	const trace0 = new Float32Array(pixels);
	const trace1 = new Float32Array(pixels);
	const trace2 = new Float32Array(pixels);
	for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
		const vector = vectorForRgb(
			image.data[offset]!,
			image.data[offset + 1]!,
			image.data[offset + 2]!,
			colorSpace
		);
		work0[index] = vector[0];
		work1[index] = vector[1];
		work2[index] = vector[2];
	}
	let checksum = FNV_OFFSET;
	for (let y = 0; y < image.height; y++) {
		const reverse = y % 2 === 1;
		const rowOffset = y * image.width;
		const startX = reverse ? image.width - 1 : 0;
		const end = reverse ? -1 : image.width;
		const step = reverse ? -1 : 1;
		for (let x = startX; x !== end; x += step) {
			const index = rowOffset + x;
			const current0 = work0[index]!;
			const current1 = work1[index]!;
			const current2 = work2[index]!;
			trace0[index] = current0;
			trace1[index] = current1;
			trace2[index] = current2;
			const ordinal = nearestScanOrdinal(palette, colorSpace, current0, current1, current2);
			checksum ^= palette.indices[ordinal]! + 1;
			checksum = Math.imul(checksum, FNV_PRIME) >>> 0;
			scatterTraceError(
				work0,
				work1,
				work2,
				image.width,
				image.height,
				x,
				y,
				reverse,
				dither,
				current0 - palette.v0[ordinal]!,
				current1 - palette.v1[ordinal]!,
				current2 - palette.v2[ordinal]!
			);
		}
	}
	return {
		pixels,
		width: image.width,
		height: image.height,
		v0: trace0,
		v1: trace1,
		v2: trace2,
		baselineChecksum: checksum.toString(16).padStart(8, '0'),
		traceBuildMs: performance.now() - start,
		palette
	};
}

function runDiffusionTraceStudy(
	source: PaletteStudySource,
	trace: DiffusionTrace,
	colorSpace: ColorSpaceId,
	dither: DitherId,
	variant: PaletteStudyVariant,
	study: PaletteStudyId
): PaletteStudyRow {
	const totalStart = performance.now();
	const buildStart = performance.now();
	const matcher = createTraceMatcher(trace.palette, colorSpace, variant);
	const buildMs = performance.now() - buildStart;
	let checksum = FNV_OFFSET;
	const loopStart = performance.now();
	for (let y = 0; y < trace.height; y++) {
		const reverse = y % 2 === 1;
		const start = reverse ? trace.width - 1 : 0;
		const end = reverse ? -1 : trace.width;
		const step = reverse ? -1 : 1;
		const rowOffset = y * trace.width;
		for (let x = start; x !== end; x += step) {
			const index = rowOffset + x;
			const ordinal = matcher.nearest(trace.v0[index]!, trace.v1[index]!, trace.v2[index]!);
			checksum ^= trace.palette.indices[ordinal]! + 1;
			checksum = Math.imul(checksum, FNV_PRIME) >>> 0;
		}
	}
	const loopMs = performance.now() - loopStart;
	const checksumText = checksum.toString(16).padStart(8, '0');
	return {
		study,
		source: source.id,
		pixels: trace.pixels,
		paletteColors: trace.palette.count,
		dither,
		colorSpace,
		variant,
		buildMs,
		loopMs,
		totalMs: performance.now() - totalStart,
		candidateEvaluations: matcher.candidateEvaluations(),
		queries: trace.pixels,
		uniqueKeys: 0,
		cacheHits: matcher.cacheHits(),
		cacheMisses: matcher.cacheMisses(),
		cacheCollisions: 0,
		cacheSets: 0,
		cacheBytes: matcher.bytes(),
		tableBytes: 0,
		workBytes: trace.v0.byteLength + trace.v1.byteLength + trace.v2.byteLength,
		checksum: checksumText,
		matchesBaseline: checksumText === trace.baselineChecksum,
		notes: `${matcher.notes()}; trace build ${trace.traceBuildMs.toFixed(1)}ms`
	};
}

function createTraceMatcher(
	palette: VectorPalette,
	colorSpace: ColorSpaceId,
	variant: PaletteStudyVariant
): TraceMatcher {
	switch (variant) {
		case 'vp-tree':
			return createVpTraceMatcher(palette, colorSpace);
		case 'ball-tree':
			return createBallTraceMatcher(palette, colorSpace);
		case 'previous-verify':
			return createPreviousVerifyMatcher(palette, colorSpace);
		case 'scan':
		default:
			return createScanTraceMatcher(palette, colorSpace);
	}
}

function vectorPalette(colors: EnabledPaletteColor[], colorSpace: ColorSpaceId): VectorPalette {
	const visible = colors.filter((color) => color.rgb && color.kind !== 'transparent');
	const indices = new Uint8Array(visible.length);
	const v0 = new Float64Array(visible.length);
	const v1 = new Float64Array(visible.length);
	const v2 = new Float64Array(visible.length);
	for (let ordinal = 0; ordinal < visible.length; ordinal++) {
		const color = visible[ordinal]!;
		const rgb = color.rgb!;
		const vector = vectorForRgb(rgb.r, rgb.g, rgb.b, colorSpace);
		indices[ordinal] = colors.indexOf(color);
		v0[ordinal] = vector[0];
		v1[ordinal] = vector[1];
		v2[ordinal] = vector[2];
	}
	return { indices, v0, v1, v2, count: visible.length };
}

function createScanTraceMatcher(palette: VectorPalette, colorSpace: ColorSpaceId): TraceMatcher {
	let candidates = 0;
	return {
		nearest(v0, v1, v2) {
			candidates += palette.count;
			return nearestScanOrdinal(palette, colorSpace, v0, v1, v2);
		},
		candidateEvaluations: () => candidates,
		cacheHits: () => 0,
		cacheMisses: () => 0,
		bytes: () => 0,
		notes: () => 'brute vector scan replay'
	};
}

function createPreviousVerifyMatcher(
	palette: VectorPalette,
	colorSpace: ColorSpaceId
): TraceMatcher {
	if (colorSpace === 'oklch') return createScanTraceMatcher(palette, colorSpace);
	let previous = 0;
	let candidates = 0;
	let hits = 0;
	let misses = 0;
	return {
		nearest(v0, v1, v2) {
			candidates += palette.count;
			if (verifyEuclideanOrdinal(palette, previous, v0, v1, v2)) {
				hits++;
				return previous;
			}
			misses++;
			candidates += palette.count;
			previous = nearestScanOrdinal(palette, colorSpace, v0, v1, v2);
			return previous;
		},
		candidateEvaluations: () => candidates,
		cacheHits: () => hits,
		cacheMisses: () => misses,
		bytes: () => 0,
		notes: () => 'previous winner Voronoi half-space verification with scan fallback'
	};
}

function createVpTraceMatcher(palette: VectorPalette, colorSpace: ColorSpaceId): TraceMatcher {
	if (colorSpace === 'oklch') return createScanTraceMatcher(palette, colorSpace);
	const ordinals = Array.from({ length: palette.count }, (_, ordinal) => ordinal);
	const nodes: VpNode[] = [];
	buildVpNode(palette, ordinals, nodes);
	let candidates = 0;
	return {
		nearest(v0, v1, v2) {
			let bestOrdinal = -1;
			let bestDistance = Infinity;
			const visit = (nodeIndex: number) => {
				if (nodeIndex === -1) return;
				const node = nodes[nodeIndex]!;
				const squaredDistance = euclideanDistanceToOrdinal(palette, node.ordinal, v0, v1, v2);
				const distance = Math.sqrt(squaredDistance);
				candidates++;
				if (isBetterOrdinal(squaredDistance, node.ordinal, bestDistance, bestOrdinal)) {
					bestDistance = squaredDistance;
					bestOrdinal = node.ordinal;
				}
				const leftFirst = distance < node.threshold;
				const near = leftFirst ? node.left : node.right;
				const far = leftFirst ? node.right : node.left;
				visit(near);
				if (Math.abs(distance - node.threshold) <= Math.sqrt(bestDistance)) visit(far);
			};
			visit(0);
			return bestOrdinal;
		},
		candidateEvaluations: () => candidates,
		cacheHits: () => 0,
		cacheMisses: () => 0,
		bytes: () => nodes.length * (4 * 3 + 8),
		notes: () => `exact VP-tree replay (${nodes.length} nodes)`
	};
}

function createBallTraceMatcher(palette: VectorPalette, colorSpace: ColorSpaceId): TraceMatcher {
	if (colorSpace === 'oklch') return createScanTraceMatcher(palette, colorSpace);
	const ordinals = Array.from({ length: palette.count }, (_, ordinal) => ordinal);
	const nodes: BallNode[] = [];
	const ordered: number[] = [];
	buildBallNode(palette, ordinals, nodes, ordered);
	let candidates = 0;
	return {
		nearest(v0, v1, v2) {
			let bestOrdinal = -1;
			let bestDistance = Infinity;
			const visit = (nodeIndex: number) => {
				if (nodeIndex === -1) return;
				const node = nodes[nodeIndex]!;
				const centerDistance = Math.sqrt(
					(v0 - node.center0) ** 2 + (v1 - node.center1) ** 2 + (v2 - node.center2) ** 2
				);
				const lowerBound = Math.max(0, centerDistance - node.radius);
				if (lowerBound * lowerBound > bestDistance) return;
				if (node.left === -1 && node.right === -1) {
					for (let index = node.start; index < node.end; index++) {
						const ordinal = ordered[index]!;
						const distance = euclideanDistanceToOrdinal(palette, ordinal, v0, v1, v2);
						candidates++;
						if (isBetterOrdinal(distance, ordinal, bestDistance, bestOrdinal)) {
							bestDistance = distance;
							bestOrdinal = ordinal;
						}
					}
					return;
				}
				const left = node.left === -1 ? undefined : nodes[node.left]!;
				const right = node.right === -1 ? undefined : nodes[node.right]!;
				const leftBound = left
					? Math.max(
							0,
							Math.sqrt(
								(v0 - left.center0) ** 2 + (v1 - left.center1) ** 2 + (v2 - left.center2) ** 2
							) - left.radius
						)
					: Infinity;
				const rightBound = right
					? Math.max(
							0,
							Math.sqrt(
								(v0 - right.center0) ** 2 + (v1 - right.center1) ** 2 + (v2 - right.center2) ** 2
							) - right.radius
						)
					: Infinity;
				if (leftBound <= rightBound) {
					visit(node.left);
					visit(node.right);
				} else {
					visit(node.right);
					visit(node.left);
				}
			};
			visit(0);
			return bestOrdinal;
		},
		candidateEvaluations: () => candidates,
		cacheHits: () => 0,
		cacheMisses: () => 0,
		bytes: () => nodes.length * 56 + ordered.length * 4,
		notes: () => `exact ball-tree replay (${nodes.length} nodes, leaf<=4)`
	};
}

function buildVpNode(palette: VectorPalette, ordinals: number[], nodes: VpNode[]): number {
	if (ordinals.length === 0) return -1;
	const ordinal = ordinals[0]!;
	const nodeIndex = nodes.length;
	nodes.push({ ordinal, left: -1, right: -1, threshold: 0 });
	if (ordinals.length === 1) return nodeIndex;
	const rest = ordinals.slice(1).sort((left, right) => {
		const dl = euclideanDistanceBetweenOrdinals(palette, ordinal, left);
		const dr = euclideanDistanceBetweenOrdinals(palette, ordinal, right);
		return dl - dr;
	});
	const mid = Math.floor(rest.length / 2);
	const threshold = Math.sqrt(euclideanDistanceBetweenOrdinals(palette, ordinal, rest[mid]!));
	nodes[nodeIndex]!.threshold = threshold;
	nodes[nodeIndex]!.left = buildVpNode(palette, rest.slice(0, mid), nodes);
	nodes[nodeIndex]!.right = buildVpNode(palette, rest.slice(mid), nodes);
	return nodeIndex;
}

function buildBallNode(
	palette: VectorPalette,
	ordinals: number[],
	nodes: BallNode[],
	ordered: number[]
): number {
	const nodeIndex = nodes.length;
	const center0 = mean(ordinals.map((ordinal) => palette.v0[ordinal]!));
	const center1 = mean(ordinals.map((ordinal) => palette.v1[ordinal]!));
	const center2 = mean(ordinals.map((ordinal) => palette.v2[ordinal]!));
	const radius = Math.max(
		...ordinals.map((ordinal) =>
			Math.sqrt(
				(palette.v0[ordinal]! - center0) ** 2 +
					(palette.v1[ordinal]! - center1) ** 2 +
					(palette.v2[ordinal]! - center2) ** 2
			)
		)
	);
	const node: BallNode = {
		start: ordered.length,
		end: ordered.length,
		left: -1,
		right: -1,
		center0,
		center1,
		center2,
		radius
	};
	nodes.push(node);
	if (ordinals.length <= 4) {
		node.start = ordered.length;
		ordered.push(...ordinals);
		node.end = ordered.length;
		return nodeIndex;
	}
	const ranges = [
		Math.max(...ordinals.map((ordinal) => palette.v0[ordinal]!)) -
			Math.min(...ordinals.map((ordinal) => palette.v0[ordinal]!)),
		Math.max(...ordinals.map((ordinal) => palette.v1[ordinal]!)) -
			Math.min(...ordinals.map((ordinal) => palette.v1[ordinal]!)),
		Math.max(...ordinals.map((ordinal) => palette.v2[ordinal]!)) -
			Math.min(...ordinals.map((ordinal) => palette.v2[ordinal]!))
	];
	const axis =
		ranges[0]! >= ranges[1]! && ranges[0]! >= ranges[2]! ? 0 : ranges[1]! >= ranges[2]! ? 1 : 2;
	const sorted = ordinals
		.slice()
		.sort((left, right) => paletteValue(palette, left, axis) - paletteValue(palette, right, axis));
	const mid = Math.floor(sorted.length / 2);
	node.left = buildBallNode(palette, sorted.slice(0, mid), nodes, ordered);
	node.right = buildBallNode(palette, sorted.slice(mid), nodes, ordered);
	return nodeIndex;
}

function nearestScanOrdinal(
	palette: VectorPalette,
	colorSpace: ColorSpaceId,
	v0: number,
	v1: number,
	v2: number
) {
	let winner = -1;
	let best = Infinity;
	for (let ordinal = 0; ordinal < palette.count; ordinal++) {
		const distance = vectorDistanceToOrdinal(palette, colorSpace, ordinal, v0, v1, v2);
		if (isBetterOrdinal(distance, ordinal, best, winner)) {
			best = distance;
			winner = ordinal;
		}
	}
	return winner;
}

function vectorDistanceToOrdinal(
	palette: VectorPalette,
	colorSpace: ColorSpaceId,
	ordinal: number,
	v0: number,
	v1: number,
	v2: number
) {
	if (colorSpace === 'oklch') {
		const dl = v0 - palette.v0[ordinal]!;
		const dc = v1 - palette.v1[ordinal]!;
		let hue = Math.abs(v2 - palette.v2[ordinal]!);
		if (hue > Math.PI) hue = Math.PI * 2 - hue;
		const dh = Math.min(v1, palette.v1[ordinal]!) * hue;
		return dl * dl + dc * dc + dh * dh;
	}
	return euclideanDistanceToOrdinal(palette, ordinal, v0, v1, v2);
}

function euclideanDistanceToOrdinal(
	palette: VectorPalette,
	ordinal: number,
	v0: number,
	v1: number,
	v2: number
) {
	const dx = v0 - palette.v0[ordinal]!;
	const dy = v1 - palette.v1[ordinal]!;
	const dz = v2 - palette.v2[ordinal]!;
	return dx * dx + dy * dy + dz * dz;
}

function euclideanDistanceBetweenOrdinals(palette: VectorPalette, left: number, right: number) {
	return euclideanDistanceToOrdinal(
		palette,
		left,
		palette.v0[right]!,
		palette.v1[right]!,
		palette.v2[right]!
	);
}

function verifyEuclideanOrdinal(
	palette: VectorPalette,
	ordinal: number,
	v0: number,
	v1: number,
	v2: number
) {
	const candidate0 = palette.v0[ordinal]!;
	const candidate1 = palette.v1[ordinal]!;
	const candidate2 = palette.v2[ordinal]!;
	const candidateNorm = candidate0 * candidate0 + candidate1 * candidate1 + candidate2 * candidate2;
	const epsilon = 1e-10;
	for (let other = 0; other < palette.count; other++) {
		if (other === ordinal) continue;
		const other0 = palette.v0[other]!;
		const other1 = palette.v1[other]!;
		const other2 = palette.v2[other]!;
		const otherNorm = other0 * other0 + other1 * other1 + other2 * other2;
		const delta =
			2 * (v0 * (other0 - candidate0) + v1 * (other1 - candidate1) + v2 * (other2 - candidate2)) -
			(otherNorm - candidateNorm);
		if (delta < -epsilon) continue;
		if (Math.abs(delta) <= epsilon && ordinal < other) continue;
		return false;
	}
	return true;
}

function isBetterOrdinal(distance: number, ordinal: number, best: number, winner: number) {
	return distance < best || (distance === best && (winner === -1 || ordinal < winner));
}

function paletteValue(palette: VectorPalette, ordinal: number, axis: number) {
	if (axis === 0) return palette.v0[ordinal]!;
	if (axis === 1) return palette.v1[ordinal]!;
	return palette.v2[ordinal]!;
}

function scatterTraceError(
	work0: Float32Array,
	work1: Float32Array,
	work2: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	reverse: boolean,
	dither: DitherId,
	error0: number,
	error1: number,
	error2: number
) {
	for (const [dxBase, dy, weight] of diffusionKernel(dither)) {
		const dx = reverse ? -dxBase : dxBase;
		const xx = x + dx;
		const yy = y + dy;
		if (xx < 0 || xx >= width || yy < 0 || yy >= height) continue;
		const index = yy * width + xx;
		work0[index] += error0 * weight;
		work1[index] += error1 * weight;
		work2[index] += error2 * weight;
	}
}

function runRollingRowKernelStudy(
	source: PaletteStudySource,
	dither: DitherId,
	buildStart: number,
	totalStart: number
): PaletteStudyRow {
	const image = source.imageData;
	const pixels = image.width * image.height;
	const rows = dither === 'sierra' ? 3 : 2;
	const work = new Float32Array(image.width * rows * 3);
	const buildMs = performance.now() - buildStart;
	let checksum = FNV_OFFSET;
	const loopStart = performance.now();
	for (let y = 0; y < image.height; y++) {
		const rowSlot = y % rows;
		const rowBase = rowSlot * image.width * 3;
		for (let x = 0; x < image.width; x++) {
			const sourceOffset = (y * image.width + x) * 4;
			const offset = rowBase + x * 3;
			work[offset] += image.data[sourceOffset]!;
			work[offset + 1] += image.data[sourceOffset + 1]!;
			work[offset + 2] += image.data[sourceOffset + 2]!;
		}
		const reverse = y % 2 === 1;
		const start = reverse ? image.width - 1 : 0;
		const end = reverse ? -1 : image.width;
		const step = reverse ? -1 : 1;
		for (let x = start; x !== end; x += step) {
			const offset = rowBase + x * 3;
			const current0 = work[offset]!;
			const current1 = work[offset + 1]!;
			const current2 = work[offset + 2]!;
			const error0 = current0 - 96;
			const error1 = current1 - 96;
			const error2 = current2 - 96;
			checksum ^= ((current0 + current1 + current2) | 0) & 255;
			checksum = Math.imul(checksum, FNV_PRIME) >>> 0;
			scatterRollingKernel(
				work,
				image.width,
				image.height,
				rows,
				x,
				y,
				reverse,
				dither,
				error0,
				error1,
				error2
			);
			work[offset] = 0;
			work[offset + 1] = 0;
			work[offset + 2] = 0;
		}
	}
	const loopMs = performance.now() - loopStart;
	return {
		study: 'diffusion-kernel-only',
		source: source.id,
		pixels,
		paletteColors: 0,
		dither,
		colorSpace: 'srgb',
		variant: 'rolling-row-kernel',
		buildMs,
		loopMs,
		totalMs: performance.now() - totalStart,
		candidateEvaluations: 0,
		queries: pixels,
		uniqueKeys: 0,
		cacheHits: 0,
		cacheMisses: 0,
		cacheCollisions: 0,
		cacheSets: 0,
		cacheBytes: 0,
		tableBytes: 0,
		workBytes: work.byteLength,
		checksum: checksum.toString(16).padStart(8, '0'),
		matchesBaseline: true,
		notes: `rolling ${rows}-row fake nearest-color diffusion scatter/update only`
	};
}

function scatterRollingKernel(
	work: Float32Array,
	width: number,
	height: number,
	rows: number,
	x: number,
	y: number,
	reverse: boolean,
	dither: DitherId,
	error0: number,
	error1: number,
	error2: number
) {
	for (const [dxBase, dy, weight] of diffusionKernel(dither)) {
		const dx = reverse ? -dxBase : dxBase;
		const xx = x + dx;
		const yy = y + dy;
		if (xx < 0 || xx >= width || yy < 0 || yy >= height) continue;
		const target = ((yy % rows) * width + xx) * 3;
		addKernelError(work, target, error0, error1, error2, weight);
	}
}

function scatterKernelGeneric(
	work: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	reverse: boolean,
	dither: DitherId,
	error0: number,
	error1: number,
	error2: number
) {
	for (const [dxBase, dy, weight] of diffusionKernel(dither)) {
		const dx = reverse ? -dxBase : dxBase;
		const xx = x + dx;
		const yy = y + dy;
		if (xx < 0 || xx >= width || yy < 0 || yy >= height) continue;
		addKernelError(work, (yy * width + xx) * 3, error0, error1, error2, weight);
	}
}

function scatterKernelUnrolled(
	work: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	workOffset: number,
	reverse: boolean,
	dither: DitherId,
	error0: number,
	error1: number,
	error2: number
) {
	switch (dither) {
		case 'floyd-steinberg':
			scatterKernelFloydUnrolled(
				work,
				width,
				height,
				x,
				y,
				workOffset,
				reverse,
				error0,
				error1,
				error2
			);
			return;
		case 'sierra':
			scatterKernelSierraUnrolled(
				work,
				width,
				height,
				x,
				y,
				workOffset,
				reverse,
				error0,
				error1,
				error2
			);
			return;
		case 'sierra-lite':
			scatterKernelSierraLiteUnrolled(
				work,
				width,
				height,
				x,
				y,
				workOffset,
				reverse,
				error0,
				error1,
				error2
			);
			return;
	}
}

function scatterKernelFloydUnrolled(
	work: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	workOffset: number,
	reverse: boolean,
	error0: number,
	error1: number,
	error2: number
) {
	if (reverse) {
		if (x > 0) addKernelError(work, workOffset - 3, error0, error1, error2, 7 / 16);
		if (y + 1 < height) {
			const nextRow = workOffset + width * 3;
			if (x + 1 < width) addKernelError(work, nextRow + 3, error0, error1, error2, 3 / 16);
			addKernelError(work, nextRow, error0, error1, error2, 5 / 16);
			if (x > 0) addKernelError(work, nextRow - 3, error0, error1, error2, 1 / 16);
		}
		return;
	}
	if (x + 1 < width) addKernelError(work, workOffset + 3, error0, error1, error2, 7 / 16);
	if (y + 1 < height) {
		const nextRow = workOffset + width * 3;
		if (x > 0) addKernelError(work, nextRow - 3, error0, error1, error2, 3 / 16);
		addKernelError(work, nextRow, error0, error1, error2, 5 / 16);
		if (x + 1 < width) addKernelError(work, nextRow + 3, error0, error1, error2, 1 / 16);
	}
}

function scatterKernelSierraUnrolled(
	work: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	workOffset: number,
	reverse: boolean,
	error0: number,
	error1: number,
	error2: number
) {
	if (reverse) {
		if (x > 0) addKernelError(work, workOffset - 3, error0, error1, error2, 5 / 32);
		if (x > 1) addKernelError(work, workOffset - 6, error0, error1, error2, 3 / 32);
		if (y + 1 < height) {
			const nextRow = workOffset + width * 3;
			if (x + 2 < width) addKernelError(work, nextRow + 6, error0, error1, error2, 2 / 32);
			if (x + 1 < width) addKernelError(work, nextRow + 3, error0, error1, error2, 4 / 32);
			addKernelError(work, nextRow, error0, error1, error2, 5 / 32);
			if (x > 0) addKernelError(work, nextRow - 3, error0, error1, error2, 4 / 32);
			if (x > 1) addKernelError(work, nextRow - 6, error0, error1, error2, 2 / 32);
		}
		if (y + 2 < height) {
			const next2Row = workOffset + width * 6;
			if (x + 1 < width) addKernelError(work, next2Row + 3, error0, error1, error2, 2 / 32);
			addKernelError(work, next2Row, error0, error1, error2, 3 / 32);
			if (x > 0) addKernelError(work, next2Row - 3, error0, error1, error2, 2 / 32);
		}
		return;
	}
	if (x + 1 < width) addKernelError(work, workOffset + 3, error0, error1, error2, 5 / 32);
	if (x + 2 < width) addKernelError(work, workOffset + 6, error0, error1, error2, 3 / 32);
	if (y + 1 < height) {
		const nextRow = workOffset + width * 3;
		if (x > 1) addKernelError(work, nextRow - 6, error0, error1, error2, 2 / 32);
		if (x > 0) addKernelError(work, nextRow - 3, error0, error1, error2, 4 / 32);
		addKernelError(work, nextRow, error0, error1, error2, 5 / 32);
		if (x + 1 < width) addKernelError(work, nextRow + 3, error0, error1, error2, 4 / 32);
		if (x + 2 < width) addKernelError(work, nextRow + 6, error0, error1, error2, 2 / 32);
	}
	if (y + 2 < height) {
		const next2Row = workOffset + width * 6;
		if (x > 0) addKernelError(work, next2Row - 3, error0, error1, error2, 2 / 32);
		addKernelError(work, next2Row, error0, error1, error2, 3 / 32);
		if (x + 1 < width) addKernelError(work, next2Row + 3, error0, error1, error2, 2 / 32);
	}
}

function scatterKernelSierraLiteUnrolled(
	work: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	workOffset: number,
	reverse: boolean,
	error0: number,
	error1: number,
	error2: number
) {
	if (reverse) {
		if (x > 0) addKernelError(work, workOffset - 3, error0, error1, error2, 2 / 4);
		if (y + 1 < height) {
			const nextRow = workOffset + width * 3;
			if (x + 1 < width) addKernelError(work, nextRow + 3, error0, error1, error2, 1 / 4);
			addKernelError(work, nextRow, error0, error1, error2, 1 / 4);
		}
		return;
	}
	if (x + 1 < width) addKernelError(work, workOffset + 3, error0, error1, error2, 2 / 4);
	if (y + 1 < height) {
		const nextRow = workOffset + width * 3;
		if (x > 0) addKernelError(work, nextRow - 3, error0, error1, error2, 1 / 4);
		addKernelError(work, nextRow, error0, error1, error2, 1 / 4);
	}
}

function diffusionKernel(dither: DitherId): readonly (readonly [number, number, number])[] {
	switch (dither) {
		case 'floyd-steinberg':
			return [
				[1, 0, 7 / 16],
				[-1, 1, 3 / 16],
				[0, 1, 5 / 16],
				[1, 1, 1 / 16]
			];
		case 'sierra':
			return [
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
			];
		case 'sierra-lite':
			return [
				[1, 0, 2 / 4],
				[-1, 1, 1 / 4],
				[0, 1, 1 / 4]
			];
		default:
			return [];
	}
}

function addKernelError(
	work: Float32Array,
	target: number,
	error0: number,
	error1: number,
	error2: number,
	weight: number
) {
	work[target] += error0 * weight;
	work[target + 1] += error1 * weight;
	work[target + 2] += error2 * weight;
}

function matcherOptionsForVariant(variant: PaletteStudyVariant): PaletteMatcherOptions {
	switch (variant) {
		case 'distance-tables':
			return { denseRgbMemo: false, distanceTables: true };
		case 'dense-rgb-distance-tables':
			return { denseRgbMemo: true, distanceTables: true };
		case 'scan':
		case 'generic-kernel':
		case 'unrolled-kernel':
		case 'rolling-row-kernel':
		case 'vp-tree':
		case 'ball-tree':
		case 'previous-verify':
		case 'threshold-scan':
		case 'threshold-direct-cache':
		case 'threshold-direct-cache-23':
		case 'threshold-direct-cache-24':
		case 'threshold-probe-4':
		case 'threshold-unique-map':
		case 'threshold-unique-map-hot':
			return { denseRgbMemo: false, distanceTables: false };
	}
}

function distanceTableBytes(
	variant: PaletteStudyVariant,
	colorSpace: ColorSpaceId,
	paletteColors: number
) {
	if (variant === 'scan') return 0;
	if (
		colorSpace !== 'srgb' &&
		colorSpace !== 'linear-rgb' &&
		colorSpace !== 'weighted-rgb-601' &&
		colorSpace !== 'weighted-rgb-709'
	) {
		return 0;
	}
	return paletteColors * 256 * 3 * Float64Array.BYTES_PER_ELEMENT;
}

function meanDirectRows(rows: PaletteStudyRow[]): PaletteStudyRow {
	const first = rows[0]!;
	return {
		...first,
		buildMs: mean(rows.map((row) => row.buildMs)),
		loopMs: mean(rows.map((row) => row.loopMs)),
		totalMs: mean(rows.map((row) => row.totalMs)),
		candidateEvaluations: mean(rows.map((row) => row.candidateEvaluations)),
		cacheHits: mean(rows.map((row) => row.cacheHits)),
		cacheMisses: mean(rows.map((row) => row.cacheMisses)),
		cacheCollisions: mean(rows.map((row) => row.cacheCollisions)),
		cacheSets: mean(rows.map((row) => row.cacheSets))
	};
}

function unsupportedRows(
	source: PaletteStudySource,
	study: PaletteStudyId,
	variants: readonly PaletteStudyVariant[],
	colorSpaces: readonly ColorSpaceId[]
): PaletteStudyRow[] {
	return colorSpaces.flatMap((colorSpace) =>
		variants.map((variant) => ({
			study,
			source: source.id,
			pixels: source.imageData.width * source.imageData.height,
			paletteColors: enabledBenchmarkPalette().filter(
				(color) => color.rgb && color.kind !== 'transparent'
			).length,
			dither: 'none' as const,
			colorSpace,
			variant,
			buildMs: 0,
			loopMs: 0,
			totalMs: 0,
			candidateEvaluations: 0,
			queries: 0,
			uniqueKeys: 0,
			cacheHits: 0,
			cacheMisses: 0,
			cacheCollisions: 0,
			cacheSets: 0,
			cacheBytes: 0,
			tableBytes: 0,
			workBytes: 0,
			checksum: '',
			matchesBaseline: false,
			notes: 'study not implemented yet'
		}))
	);
}

function enabledBenchmarkPalette(): EnabledPaletteColor[] {
	return WPLACE_PALETTE.map((color) => ({ ...color, enabled: true }));
}

function mean(values: readonly number[]) {
	return values.reduce((total, value) => total + value, 0) / values.length;
}

export function paletteStudyResultsToCsv(result: PaletteStudyResult): string {
	const headers: (keyof PaletteStudyRow)[] = [
		'study',
		'source',
		'pixels',
		'paletteColors',
		'dither',
		'colorSpace',
		'variant',
		'buildMs',
		'loopMs',
		'totalMs',
		'candidateEvaluations',
		'queries',
		'uniqueKeys',
		'cacheHits',
		'cacheMisses',
		'cacheCollisions',
		'cacheSets',
		'cacheBytes',
		'tableBytes',
		'workBytes',
		'checksum',
		'matchesBaseline',
		'notes'
	];
	return [
		headers.join(','),
		...result.rows.map((row) => headers.map((header) => csvCell(row[header])).join(','))
	].join('\n');
}

function csvCell(value: unknown) {
	const text = String(value);
	return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

export function formatPaletteStudyTable(result: PaletteStudyResult): string {
	const headers = [
		'study',
		'dither',
		'color',
		'variant',
		'build',
		'loop',
		'candidates',
		'hits',
		'misses',
		'unique',
		'cache',
		'ok'
	];
	const rows = result.rows.map((row) => [
		row.study,
		row.dither,
		row.colorSpace,
		row.variant,
		formatMs(row.buildMs),
		formatMs(row.loopMs),
		formatCount(row.candidateEvaluations),
		formatCount(row.cacheHits),
		formatCount(row.cacheMisses),
		formatCount(row.uniqueKeys),
		formatBytes(row.cacheBytes + row.tableBytes + row.workBytes),
		row.matchesBaseline ? 'yes' : 'no'
	]);
	return table([headers, ...rows]);
}

function table(rows: string[][]) {
	const widths = rows[0]!.map((_, column) => Math.max(...rows.map((row) => row[column]!.length)));
	return rows
		.map((row) => row.map((cell, column) => cell.padEnd(widths[column]!)).join('  '))
		.join('\n');
}

function formatMs(value: number) {
	return `${value.toFixed(value >= 100 ? 0 : 1)}ms`;
}

function formatCount(value: number) {
	if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
	if (value >= 1_000) return `${Math.round(value / 1_000)}K`;
	return String(Math.round(value));
}

function formatBytes(value: number) {
	if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)}MB`;
	if (value >= 1024) return `${(value / 1024).toFixed(1)}KB`;
	return `${value}B`;
}
