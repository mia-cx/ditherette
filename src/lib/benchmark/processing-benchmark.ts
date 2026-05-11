import { WPLACE_PALETTE } from '$lib/palette/wplace';
import { encodeIndexedPng } from '$lib/processing/png';
import {
	prepareQuantizeColorSpace,
	quantizeImage,
	type ColorVectorImage,
	type PaletteVectorSpace,
	type QuantizeCaches
} from '$lib/processing/quantize';
import { processedToImageData } from '$lib/processing/render';
import { resizeImageData } from '$lib/processing/resize';
import type {
	ColorSpaceId,
	DitherId,
	EnabledPaletteColor,
	OutputSettings,
	ProcessedImage,
	ProcessingSettings,
	ResizeId
} from '$lib/processing/types';

export type BenchmarkProfile = 'smoke' | 'baseline' | 'large' | 'exhaustive';
export type BenchmarkStage = 'resize' | 'quantize' | 'previewRender' | 'pngEncode' | 'total';
export type BenchmarkMatrixDimension = 'scale' | 'resize' | 'dither' | 'colorSpace';
export type BenchmarkStopStage = Exclude<BenchmarkStage, 'total'> | 'colorSpaceConvert';

export type BenchmarkCase = {
	id: string;
	label: string;
	sourceWidth?: number;
	sourceHeight?: number;
	outputWidth?: number;
	outputHeight?: number;
	outputScale?: number;
	noResize?: boolean;
	resize: ResizeId;
	dither: DitherId;
	colorSpace: ColorSpaceId;
	useColorSpace: boolean;
	includePng: boolean;
};

export type BenchmarkSource = {
	id: string;
	label: string;
	kind: 'synthetic' | 'image';
	imageData: ImageData;
	path?: string;
	decodeMs?: number;
};

export type BenchmarkProgressEvent =
	| {
			type: 'case-start';
			caseIndex: number;
			totalCases: number;
			sourceId: string;
			caseId: string;
			case: BenchmarkCase;
			label: string;
			outputWidth: number;
			outputHeight: number;
			outputPixels: number;
	  }
	| {
			type: 'case-end';
			caseIndex: number;
			totalCases: number;
			sourceId: string;
			caseId: string;
			durationMs: number;
			result: BenchmarkCaseResult;
	  }
	| {
			type: 'iteration-start';
			sourceId: string;
			caseId: string;
			iteration: number;
			warmup: boolean;
	  }
	| {
			type: 'iteration-end';
			sourceId: string;
			caseId: string;
			iteration: number;
			warmup: boolean;
			durationMs: number;
			resizeCacheHit: boolean;
			stages: Record<BenchmarkStage, number>;
			quantizeTimings: Record<string, number>;
			quantizeCounts: Record<string, number>;
	  }
	| {
			type: 'stage-start';
			sourceId: string;
			caseId: string;
			iteration: number;
			warmup: boolean;
			stage: BenchmarkStage;
	  }
	| {
			type: 'stage-end';
			sourceId: string;
			caseId: string;
			iteration: number;
			warmup: boolean;
			stage: BenchmarkStage;
			durationMs: number;
	  };

export type BenchmarkOptions = {
	profile?: BenchmarkProfile;
	iterations?: number;
	warmups?: number;
	caseIds?: readonly string[];
	includePng?: boolean;
	sources?: readonly BenchmarkSource[];
	matrixDimensions?: readonly BenchmarkMatrixDimension[];
	stopAfterStage?: BenchmarkStopStage;
	noResize?: boolean;
	onProgress?: (event: BenchmarkProgressEvent) => void;
};

export type StageStats = {
	minMs: number;
	meanMs: number;
	medianMs: number;
	p95Ms: number;
	maxMs: number;
};

export type BenchmarkMemoryShape = {
	sourcePixels: number;
	outputPixels: number;
	sourceRgbaBytes: number;
	resizedRgbaBytes: number;
	indexBytes: number;
	errorWorkBufferBytes: number;
	paletteColors: number;
	pngBytes?: number;
};

export type BenchmarkSourceSummary = Omit<BenchmarkSource, 'imageData'> & {
	width: number;
	height: number;
	pixels: number;
};

export type BenchmarkRunRecord = Record<BenchmarkStage, number> & {
	resizeCacheHit: boolean;
	quantizeTimings: Record<string, number>;
	quantizeCounts: Record<string, number>;
};

export type BenchmarkCaseResult = {
	case: BenchmarkCase;
	source: BenchmarkSourceSummary;
	iterations: number;
	stages: Record<BenchmarkStage, StageStats>;
	quantizeSubstages: Record<string, StageStats>;
	quantizeCounters: Record<string, StageStats>;
	runs: BenchmarkRunRecord[];
	memory: BenchmarkMemoryShape;
	hotspot: BenchmarkStage;
	quantizeHotspot?: string;
};

export type BenchmarkResult = {
	version: 1;
	profile: BenchmarkProfile;
	runAt: string;
	results: BenchmarkCaseResult[];
};

type BenchmarkResizeCache = Map<string, ImageData>;

const DEFAULT_ITERATIONS = 3;
const DEFAULT_WARMUPS = 1;
const ERROR_DIFFUSION_ALGORITHMS = new Set<DitherId>(['floyd-steinberg', 'sierra', 'sierra-lite']);
const EXHAUSTIVE_SCALES = [0.125, 0.25, 0.5, 0.75] as const;
const EXHAUSTIVE_RESIZE_MODES: readonly ResizeId[] = [
	'nearest',
	'bilinear',
	'lanczos2',
	'lanczos2-scale-aware',
	'lanczos3',
	'lanczos3-scale-aware',
	'area'
];
const DEFAULT_MATRIX_SCALE = 0.5;
const DEFAULT_MATRIX_RESIZE: ResizeId = 'lanczos3';
const DEFAULT_MATRIX_DITHER: DitherId = 'none';
const DEFAULT_MATRIX_COLOR_SPACE: ColorSpaceId = 'oklab';
const EXHAUSTIVE_COLOR_SPACES: readonly ColorSpaceId[] = [
	'oklab',
	'srgb',
	'linear-rgb',
	'weighted-rgb',
	'weighted-rgb-601',
	'weighted-rgb-709',
	'cielab',
	'oklch'
];
const EXHAUSTIVE_DITHER_MODES: readonly DitherId[] = [
	'none',
	'bayer-2',
	'bayer-4',
	'bayer-8',
	'bayer-16',
	'floyd-steinberg',
	'sierra',
	'sierra-lite',
	'random'
];

const BASE_OUTPUT: OutputSettings = {
	width: 1,
	height: 1,
	lockAspect: true,
	resize: 'lanczos3',
	alphaMode: 'preserve',
	alphaThreshold: 0,
	matteKey: '#FFFFFF',
	autoSizeOnUpload: false,
	scaleFactor: 1
};

const CASES: BenchmarkCase[] = [
	{
		id: 'smoke-direct',
		label: 'Smoke preview · direct · sRGB',
		sourceWidth: 256,
		sourceHeight: 192,
		outputWidth: 160,
		outputHeight: 120,
		resize: 'bilinear',
		dither: 'none',
		colorSpace: 'srgb',
		useColorSpace: false,
		includePng: false
	},
	{
		id: 'smoke-bayer-oklab',
		label: 'Smoke preview · Bayer 8 · OKLab',
		sourceWidth: 256,
		sourceHeight: 192,
		outputWidth: 160,
		outputHeight: 120,
		resize: 'bilinear',
		dither: 'bayer-8',
		colorSpace: 'oklab',
		useColorSpace: true,
		includePng: false
	},
	{
		id: 'preview-direct',
		label: 'Small live preview · direct · sRGB',
		sourceWidth: 960,
		sourceHeight: 540,
		outputWidth: 512,
		outputHeight: 288,
		resize: 'bilinear',
		dither: 'none',
		colorSpace: 'srgb',
		useColorSpace: false,
		includePng: true
	},
	{
		id: 'common-bayer-oklab',
		label: '2MP common image · Bayer 8 · OKLab',
		sourceWidth: 1920,
		sourceHeight: 1080,
		outputWidth: 1024,
		outputHeight: 576,
		resize: 'lanczos3',
		dither: 'bayer-8',
		colorSpace: 'oklab',
		useColorSpace: true,
		includePng: true
	},
	{
		id: 'common-floyd-srgb',
		label: '2MP common image · Floyd–Steinberg · sRGB',
		sourceWidth: 1920,
		sourceHeight: 1080,
		outputWidth: 1024,
		outputHeight: 576,
		resize: 'lanczos3',
		dither: 'floyd-steinberg',
		colorSpace: 'srgb',
		useColorSpace: false,
		includePng: true
	},
	{
		id: 'large-sierra-oklab',
		label: 'Large image · Sierra · OKLab',
		sourceWidth: 3840,
		sourceHeight: 2160,
		outputWidth: 2048,
		outputHeight: 1152,
		resize: 'lanczos3',
		dither: 'sierra',
		colorSpace: 'oklab',
		useColorSpace: true,
		includePng: true
	},
	{
		id: 'large-area-direct',
		label: 'Large image · area resize · direct',
		sourceWidth: 3840,
		sourceHeight: 2160,
		outputWidth: 2048,
		outputHeight: 1152,
		resize: 'area',
		dither: 'none',
		colorSpace: 'srgb',
		useColorSpace: false,
		includePng: true
	}
];

const CASES_BY_PROFILE: Record<Exclude<BenchmarkProfile, 'exhaustive'>, readonly string[]> = {
	smoke: ['smoke-direct', 'smoke-bayer-oklab'],
	baseline: ['preview-direct', 'common-bayer-oklab', 'common-floyd-srgb'],
	large: ['large-sierra-oklab', 'large-area-direct']
};

export function benchmarkCases(
	profile: BenchmarkProfile,
	matrixDimensions: readonly BenchmarkMatrixDimension[] = [],
	options: { noResize?: boolean } = {}
): BenchmarkCase[] {
	if (matrixDimensions.length || options.noResize)
		return matrixCases(matrixDimensions, false, options.noResize);
	if (profile === 'exhaustive') return exhaustiveCases(options.noResize);
	const ids = new Set(CASES_BY_PROFILE[profile]);
	return CASES.filter((testCase) => ids.has(testCase.id));
}

function defaultCaseIds(
	profile: BenchmarkProfile,
	matrixDimensions: readonly BenchmarkMatrixDimension[],
	noResize = false
): readonly string[] {
	return benchmarkCases(profile, matrixDimensions, { noResize }).map((testCase) => testCase.id);
}

function exhaustiveCases(noResize = false): BenchmarkCase[] {
	return matrixCases(['scale', 'resize', 'dither', 'colorSpace'], true, noResize);
}

function matrixCases(
	dimensions: readonly BenchmarkMatrixDimension[],
	includePng = false,
	noResize = false
): BenchmarkCase[] {
	const selected = new Set(dimensions);
	const scales: readonly (number | undefined)[] = noResize
		? [undefined]
		: selected.has('scale')
			? EXHAUSTIVE_SCALES
			: [DEFAULT_MATRIX_SCALE];
	const resizeModes = noResize
		? [DEFAULT_MATRIX_RESIZE]
		: selected.has('resize')
			? EXHAUSTIVE_RESIZE_MODES
			: [DEFAULT_MATRIX_RESIZE];
	const ditherModes = selected.has('dither') ? EXHAUSTIVE_DITHER_MODES : [DEFAULT_MATRIX_DITHER];
	const colorSpaces = selected.has('colorSpace')
		? EXHAUSTIVE_COLOR_SPACES
		: [DEFAULT_MATRIX_COLOR_SPACE];

	return scales.flatMap((scale) =>
		resizeModes.flatMap((resize) =>
			ditherModes.flatMap((dither) =>
				colorSpaces.map((colorSpace) => ({
					id: noResize
						? `native-${dither}-${colorSpace}`
						: `scale-${scaleLabel(scale!)}-${resize}-${dither}-${colorSpace}`,
					label: noResize
						? `Native size · no resize · ${dither} · ${colorSpace}`
						: `${formatScale(scale!)} scale · ${resize} · ${dither} · ${colorSpace}`,
					outputScale: scale,
					noResize,
					resize,
					dither,
					colorSpace,
					useColorSpace: true,
					includePng
				}))
			)
		)
	);
}

export function runProcessingBenchmarks(options: BenchmarkOptions = {}): BenchmarkResult {
	const profile = options.profile ?? 'smoke';
	const iterations = Math.max(1, Math.floor(options.iterations ?? DEFAULT_ITERATIONS));
	const warmups = Math.max(0, Math.floor(options.warmups ?? DEFAULT_WARMUPS));
	const matrixDimensions = options.matrixDimensions ?? [];
	const availableCases = benchmarkCases(profile, matrixDimensions, { noResize: options.noResize });
	const selectedIds = new Set(
		options.caseIds ?? defaultCaseIds(profile, matrixDimensions, options.noResize)
	);
	const cases = availableCases.filter((testCase) => selectedIds.has(testCase.id));
	const resizeCache =
		options.stopAfterStage === 'resize' ? undefined : new Map<string, ImageData>();
	const results: BenchmarkCaseResult[] = [];
	let caseIndex = 0;
	if (options.sources?.length) {
		const totalCases = options.sources.length * cases.length;
		for (const source of options.sources) {
			for (const testCase of cases) {
				caseIndex++;
				results.push(
					runBenchmarkCase(
						testCase,
						source,
						iterations,
						warmups,
						options,
						caseIndex,
						totalCases,
						resizeCache
					)
				);
			}
		}
	} else {
		const totalCases = cases.length;
		for (const testCase of cases) {
			caseIndex++;
			results.push(
				runBenchmarkCase(
					testCase,
					syntheticSourceForCase(testCase),
					iterations,
					warmups,
					options,
					caseIndex,
					totalCases,
					resizeCache
				)
			);
		}
	}

	return {
		version: 1,
		profile,
		runAt: new Date().toISOString(),
		results
	};
}

function syntheticSourceForCase(testCase: BenchmarkCase): BenchmarkSource {
	const width = sourceWidthForCase(testCase);
	const height = sourceHeightForCase(testCase);
	const id = `synthetic-${width}x${height}`;
	return {
		id,
		label: `Synthetic ${width}×${height}`,
		kind: 'synthetic',
		imageData: createFixtureImage(width, height, id)
	};
}

function runBenchmarkCase(
	testCase: BenchmarkCase,
	source: BenchmarkSource,
	iterations: number,
	warmups: number,
	options: BenchmarkOptions,
	caseIndex: number,
	totalCases: number,
	resizeCache: BenchmarkResizeCache | undefined
): BenchmarkCaseResult {
	const includePng = options.includePng ?? testCase.includePng;
	const dimensions = outputDimensionsForCase(testCase, source.imageData);
	const caseStart = performance.now();
	options.onProgress?.({
		type: 'case-start',
		caseIndex,
		totalCases,
		sourceId: source.id,
		caseId: testCase.id,
		case: { ...testCase, includePng },
		label: testCase.label,
		outputWidth: dimensions.width,
		outputHeight: dimensions.height,
		outputPixels: dimensions.width * dimensions.height
	});
	for (let index = 0; index < warmups; index++)
		runOnce(
			testCase,
			source.imageData,
			includePng,
			options,
			source.id,
			index + 1,
			true,
			resizeCache
		);
	const runs = Array.from({ length: iterations }, (_, index) =>
		runOnce(
			testCase,
			source.imageData,
			includePng,
			options,
			source.id,
			index + 1,
			false,
			resizeCache
		)
	);
	const stages = summarizeRuns(runs);
	const quantizeSubstages = summarizeQuantizeSubstages(runs);
	const quantizeCounters = summarizeQuantizeCounters(runs);
	const memory = memoryShape(
		source.imageData,
		testCase,
		runs.find((run) => run.pngBytes !== undefined)?.pngBytes
	);
	const result: BenchmarkCaseResult = {
		case: { ...testCase, includePng },
		source: summarizeSource(source),
		iterations,
		stages,
		quantizeSubstages,
		quantizeCounters,
		runs: runs.map(recordedStages),
		memory,
		hotspot: hottestStage(stages),
		quantizeHotspot: hottestQuantizeSubstage(quantizeSubstages)
	};
	options.onProgress?.({
		type: 'case-end',
		caseIndex,
		totalCases,
		sourceId: source.id,
		caseId: testCase.id,
		durationMs: performance.now() - caseStart,
		result
	});
	return result;
}

function runOnce(
	testCase: BenchmarkCase,
	source: ImageData,
	includePng: boolean,
	options: BenchmarkOptions,
	sourceId: string,
	iteration: number,
	warmup: boolean,
	resizeCache: BenchmarkResizeCache | undefined
) {
	const dimensions = outputDimensionsForCase(testCase, source);
	const settings = processingSettingsForCase(testCase, dimensions);
	const palette = enabledBenchmarkPalette();
	const skipResize = options.noResize || testCase.noResize;
	const resizeKey = skipResize ? '' : resizeCacheKey(sourceId, source, dimensions, testCase.resize);
	let resizeCacheHit = false;
	const totalStart = performance.now();
	options.onProgress?.({
		type: 'iteration-start',
		sourceId,
		caseId: testCase.id,
		iteration,
		warmup
	});

	const [resizeMs, resized] = skipResize
		? ([0, source] as const)
		: measureStage('resize', options, sourceId, testCase.id, iteration, warmup, () => {
				const cached = resizeCache?.get(resizeKey);
				if (cached) {
					resizeCacheHit = true;
					return cached;
				}
				const resized = resizeImageData(
					source,
					dimensions.width,
					dimensions.height,
					testCase.resize
				);
				resizeCache?.set(resizeKey, resized);
				return resized;
			});
	if (shouldStopAfter('resize', options.stopAfterStage)) {
		return finishRun({
			resize: resizeMs,
			quantize: 0,
			previewRender: 0,
			pngEncode: 0,
			pngBytes: undefined,
			resizeCacheHit,
			quantizeTimings: {},
			quantizeCounts: {},
			totalStart,
			options,
			sourceId,
			caseId: testCase.id,
			iteration,
			warmup
		});
	}

	const quantizeTimings: Record<string, number> = {};
	const quantizeCounts: Record<string, number> = {};
	const quantizeCaches = benchmarkQuantizeCaches(
		`${sourceId}|${testCase.id}|${warmup ? 'warmup' : 'run'}-${iteration}`,
		quantizeTimings,
		quantizeCounts
	);
	if (shouldStopAfter('colorSpaceConvert', options.stopAfterStage)) {
		const [quantizeMs] = measureStage(
			'quantize',
			options,
			sourceId,
			testCase.id,
			iteration,
			warmup,
			() => prepareQuantizeColorSpace(resized, palette, settings, quantizeCaches)
		);
		return finishRun({
			resize: resizeMs,
			quantize: quantizeMs,
			previewRender: 0,
			pngEncode: 0,
			pngBytes: undefined,
			resizeCacheHit,
			quantizeTimings,
			quantizeCounts,
			totalStart,
			options,
			sourceId,
			caseId: testCase.id,
			iteration,
			warmup
		});
	}

	const [quantizeMs, quantized] = measureStage(
		'quantize',
		options,
		sourceId,
		testCase.id,
		iteration,
		warmup,
		() => quantizeImage(resized, palette, settings, quantizeCaches)
	);
	if (shouldStopAfter('quantize', options.stopAfterStage)) {
		return finishRun({
			resize: resizeMs,
			quantize: quantizeMs,
			previewRender: 0,
			pngEncode: 0,
			pngBytes: undefined,
			resizeCacheHit,
			quantizeTimings,
			quantizeCounts,
			totalStart,
			options,
			sourceId,
			caseId: testCase.id,
			iteration,
			warmup
		});
	}

	const processed: ProcessedImage = {
		...quantized,
		width: resized.width,
		height: resized.height,
		settingsHash: testCase.id,
		updatedAt: 0
	};
	const [previewMs] = measureStage(
		'previewRender',
		options,
		sourceId,
		testCase.id,
		iteration,
		warmup,
		() => processedToImageData(processed)
	);
	if (shouldStopAfter('previewRender', options.stopAfterStage)) {
		return finishRun({
			resize: resizeMs,
			quantize: quantizeMs,
			previewRender: previewMs,
			pngEncode: 0,
			pngBytes: undefined,
			resizeCacheHit,
			quantizeTimings,
			quantizeCounts,
			totalStart,
			options,
			sourceId,
			caseId: testCase.id,
			iteration,
			warmup
		});
	}

	let pngEncodeMs = 0;
	let pngBytes: number | undefined;
	if (includePng || shouldStopAfter('pngEncode', options.stopAfterStage)) {
		const [encodeMs, blob] = measureStage(
			'pngEncode',
			options,
			sourceId,
			testCase.id,
			iteration,
			warmup,
			() => encodeIndexedPng(processed)
		);
		pngEncodeMs = encodeMs;
		pngBytes = blob.size;
	}

	return finishRun({
		resize: resizeMs,
		quantize: quantizeMs,
		previewRender: previewMs,
		pngEncode: pngEncodeMs,
		pngBytes,
		resizeCacheHit,
		quantizeTimings,
		quantizeCounts,
		totalStart,
		options,
		sourceId,
		caseId: testCase.id,
		iteration,
		warmup
	});
}

type FinishRunInput = Record<Exclude<BenchmarkStage, 'total'>, number> & {
	pngBytes: number | undefined;
	resizeCacheHit: boolean;
	quantizeTimings: Record<string, number>;
	quantizeCounts: Record<string, number>;
	totalStart: number;
	options: BenchmarkOptions;
	sourceId: string;
	caseId: string;
	iteration: number;
	warmup: boolean;
};

function finishRun(input: FinishRunInput) {
	const totalMs = performance.now() - input.totalStart;
	input.options.onProgress?.({
		type: 'iteration-end',
		sourceId: input.sourceId,
		caseId: input.caseId,
		iteration: input.iteration,
		warmup: input.warmup,
		durationMs: totalMs,
		resizeCacheHit: input.resizeCacheHit,
		stages: {
			resize: input.resize,
			quantize: input.quantize,
			previewRender: input.previewRender,
			pngEncode: input.pngEncode,
			total: totalMs
		},
		quantizeTimings: input.quantizeTimings,
		quantizeCounts: input.quantizeCounts
	});
	return {
		resize: input.resize,
		quantize: input.quantize,
		previewRender: input.previewRender,
		pngEncode: input.pngEncode,
		total: totalMs,
		pngBytes: input.pngBytes,
		resizeCacheHit: input.resizeCacheHit,
		quantizeTimings: input.quantizeTimings,
		quantizeCounts: input.quantizeCounts
	};
}

function shouldStopAfter(
	stage: BenchmarkStopStage,
	stopAfterStage: BenchmarkStopStage | undefined
) {
	return stopAfterStage === stage;
}

function recordedStages(run: ReturnType<typeof runOnce>): BenchmarkRunRecord {
	return {
		resize: run.resize,
		quantize: run.quantize,
		previewRender: run.previewRender,
		pngEncode: run.pngEncode,
		total: run.total,
		resizeCacheHit: run.resizeCacheHit,
		quantizeTimings: run.quantizeTimings,
		quantizeCounts: run.quantizeCounts
	};
}

function benchmarkQuantizeCaches(
	colorVectorImageScope: string,
	quantizeTimings: Record<string, number>,
	quantizeCounts: Record<string, number>
): QuantizeCaches {
	const paletteVectors = new Map<string, PaletteVectorSpace>();
	const images = new Map<string, ColorVectorImage>();
	return {
		colorVectorImageScope,
		getPaletteVectorSpace: (key) => paletteVectors.get(key),
		setPaletteVectorSpace: (key, value) => paletteVectors.set(key, value),
		getColorVectorImage: (key) => images.get(key),
		canStoreColorVectorImage: () => true,
		setColorVectorImage: (key, value) => images.set(key, value),
		recordTiming(name, ms) {
			quantizeTimings[name] = (quantizeTimings[name] ?? 0) + ms;
		},
		recordCount(name, amount = 1) {
			quantizeCounts[name] = (quantizeCounts[name] ?? 0) + amount;
		}
	};
}

function resizeCacheKey(
	sourceId: string,
	source: ImageData,
	dimensions: { width: number; height: number },
	resize: ResizeId
) {
	return [
		sourceId,
		`${source.width}x${source.height}`,
		`${dimensions.width}x${dimensions.height}`,
		resize
	].join('|');
}

function processingSettingsForCase(
	testCase: BenchmarkCase,
	dimensions: { width: number; height: number }
): ProcessingSettings {
	return {
		output: {
			...BASE_OUTPUT,
			width: dimensions.width,
			height: dimensions.height,
			resize: testCase.resize
		},
		dither: {
			algorithm: testCase.dither,
			strength: 100,
			placement: 'everywhere',
			placementRadius: 3,
			placementThreshold: 12,
			placementSoftness: 8,
			serpentine: true,
			seed: 0xc0ffee42,
			useColorSpace: testCase.useColorSpace
		},
		colorSpace: testCase.colorSpace
	};
}

function enabledBenchmarkPalette(): EnabledPaletteColor[] {
	return WPLACE_PALETTE.map((color) => ({ ...color, enabled: true }));
}

function outputDimensionsForCase(testCase: BenchmarkCase, source: ImageData) {
	if (testCase.noResize) return { width: source.width, height: source.height };
	if (testCase.outputScale) {
		return {
			width: Math.max(1, Math.round(source.width * testCase.outputScale)),
			height: Math.max(1, Math.round(source.height * testCase.outputScale))
		};
	}
	return {
		width: testCase.outputWidth ?? source.width,
		height: testCase.outputHeight ?? source.height
	};
}

function sourceWidthForCase(testCase: BenchmarkCase) {
	return testCase.sourceWidth ?? 1920;
}

function sourceHeightForCase(testCase: BenchmarkCase) {
	return testCase.sourceHeight ?? 1080;
}

function scaleLabel(scale: number) {
	return String(scale).replace('0.', '').replace('.', '_');
}

function formatScale(scale: number) {
	return `${scale}×`;
}

function createFixtureImage(width: number, height: number, seedKey: string): ImageData {
	const seed = hashString(seedKey);
	const data = new Uint8ClampedArray(width * height * 4);
	for (let y = 0; y < height; y++) {
		for (let x = 0; x < width; x++) {
			const offset = (y * width + x) * 4;
			const wave = Math.sin((x + seed) * 0.017) + Math.cos((y - seed) * 0.021);
			data[offset] = (x * 13 + y * 3 + seed + wave * 32) & 255;
			data[offset + 1] = (x * 5 + y * 17 + seed * 3 - wave * 24) & 255;
			data[offset + 2] = (x * y + seed * 7 + wave * 18) & 255;
			data[offset + 3] = (x + y + seed) % 31 === 0 ? 96 : 255;
		}
	}
	return new ImageData(data, width, height);
}

function hashString(value: string) {
	let hash = 2166136261;
	for (let index = 0; index < value.length; index++) {
		hash ^= value.charCodeAt(index);
		hash = Math.imul(hash, 16777619);
	}
	return hash >>> 0;
}

function measureStage<T>(
	stage: BenchmarkStage,
	options: BenchmarkOptions,
	sourceId: string,
	caseId: string,
	iteration: number,
	warmup: boolean,
	callback: () => T
): [number, T] {
	options.onProgress?.({ type: 'stage-start', sourceId, caseId, iteration, warmup, stage });
	const [durationMs, value] = measure(callback);
	options.onProgress?.({
		type: 'stage-end',
		sourceId,
		caseId,
		iteration,
		warmup,
		stage,
		durationMs
	});
	return [durationMs, value];
}

function measure<T>(callback: () => T): [number, T] {
	const start = performance.now();
	const value = callback();
	return [performance.now() - start, value];
}

function summarizeRuns(
	runs: Array<Record<BenchmarkStage, number>>
): Record<BenchmarkStage, StageStats> {
	return {
		resize: summarize(runs.map((run) => run.resize)),
		quantize: summarize(runs.map((run) => run.quantize)),
		previewRender: summarize(runs.map((run) => run.previewRender)),
		pngEncode: summarize(runs.map((run) => run.pngEncode)),
		total: summarize(runs.map((run) => run.total))
	};
}

function summarizeQuantizeSubstages(runs: Array<ReturnType<typeof runOnce>>) {
	return summarizeNamedRecords(runs.map((run) => run.quantizeTimings));
}

function summarizeQuantizeCounters(runs: Array<ReturnType<typeof runOnce>>) {
	return summarizeNamedRecords(runs.map((run) => run.quantizeCounts));
}

function summarizeNamedRecords(records: Array<Record<string, number>>) {
	const valuesByName = new Map<string, number[]>();
	for (const record of records) {
		for (const [name, value] of Object.entries(record)) {
			const values = valuesByName.get(name) ?? [];
			values.push(value);
			valuesByName.set(name, values);
		}
	}
	return Object.fromEntries(
		[...valuesByName.entries()].map(([name, values]) => [name, summarize(values)])
	) as Record<string, StageStats>;
}

function summarize(values: number[]): StageStats {
	const sorted = [...values].sort((left, right) => left - right);
	const sum = sorted.reduce((total, value) => total + value, 0);
	return {
		minMs: sorted[0] ?? 0,
		meanMs: sum / Math.max(1, sorted.length),
		medianMs: percentile(sorted, 0.5),
		p95Ms: percentile(sorted, 0.95),
		maxMs: sorted.at(-1) ?? 0
	};
}

function percentile(sortedValues: readonly number[], percentileValue: number) {
	if (sortedValues.length === 0) return 0;
	const index = Math.min(
		sortedValues.length - 1,
		Math.ceil(sortedValues.length * percentileValue) - 1
	);
	return sortedValues[index] ?? 0;
}

function summarizeSource(source: BenchmarkSource): BenchmarkSourceSummary {
	return {
		id: source.id,
		label: source.label,
		kind: source.kind,
		path: source.path,
		decodeMs: source.decodeMs,
		width: source.imageData.width,
		height: source.imageData.height,
		pixels: source.imageData.width * source.imageData.height
	};
}

function memoryShape(
	source: ImageData,
	testCase: BenchmarkCase,
	pngBytes?: number
): BenchmarkMemoryShape {
	const sourcePixels = source.width * source.height;
	const dimensions = outputDimensionsForCase(testCase, source);
	const outputPixels = dimensions.width * dimensions.height;
	return {
		sourcePixels,
		outputPixels,
		sourceRgbaBytes: sourcePixels * 4,
		resizedRgbaBytes: testCase.noResize ? 0 : outputPixels * 4,
		indexBytes: outputPixels,
		errorWorkBufferBytes: ERROR_DIFFUSION_ALGORITHMS.has(testCase.dither)
			? outputPixels * 3 * 4
			: 0,
		paletteColors: WPLACE_PALETTE.length,
		pngBytes
	};
}

function hottestStage(stages: Record<BenchmarkStage, StageStats>): BenchmarkStage {
	const candidates: BenchmarkStage[] = ['resize', 'quantize', 'previewRender', 'pngEncode'];
	return candidates.reduce((hottest, stage) =>
		stages[stage].meanMs > stages[hottest].meanMs ? stage : hottest
	);
}

function hottestQuantizeSubstage(substages: Record<string, StageStats>) {
	return Object.entries(substages).reduce<string | undefined>((hottest, [name, stats]) => {
		if (!hottest) return name;
		return stats.meanMs > substages[hottest]!.meanMs ? name : hottest;
	}, undefined);
}

function quantizeMean(entry: BenchmarkCaseResult, name: string) {
	return entry.quantizeSubstages[name]?.meanMs ?? 0;
}

function quantizeLoopMean(entry: BenchmarkCaseResult) {
	return (
		quantizeMean(entry, 'quantize direct dither+match loop') +
		quantizeMean(entry, 'quantize vector diffusion dither+match loop') +
		quantizeMean(entry, 'quantize rgb diffusion dither+match loop')
	);
}

function quantizeCountMean(entry: BenchmarkCaseResult, name: string) {
	return entry.quantizeCounters[name]?.meanMs ?? 0;
}

export function benchmarkResultsToCsv(result: BenchmarkResult): string {
	const quantizeSubstageNames = [
		...new Set(result.results.flatMap((entry) => Object.keys(entry.quantizeSubstages)))
	].sort();
	const quantizeCounterNames = [
		...new Set(result.results.flatMap((entry) => Object.keys(entry.quantizeCounters)))
	].sort();
	const rows = [
		[
			'profile',
			'sourceId',
			'sourceLabel',
			'sourceKind',
			'sourceWidth',
			'sourceHeight',
			'sourceDecodeMs',
			'caseId',
			'label',
			'sourcePixels',
			'outputPixels',
			'resizeMeanMs',
			'quantizeMeanMs',
			'quantizeStageMeanMs',
			...quantizeSubstageNames.map((name) => `quantize:${name}:meanMs`),
			...quantizeCounterNames.map((name) => `quantize-count:${name}:mean`),
			'previewRenderMeanMs',
			'pngEncodeMeanMs',
			'totalMeanMs',
			'hotspot',
			'quantizeHotspot',
			'resizedRgbaBytes',
			'indexBytes',
			'errorWorkBufferBytes',
			'pngBytes'
		],
		...result.results.map((entry) => [
			result.profile,
			entry.source.id,
			entry.source.label,
			entry.source.kind,
			entry.source.width,
			entry.source.height,
			entry.source.decodeMs ?? '',
			entry.case.id,
			entry.case.label,
			entry.memory.sourcePixels,
			entry.memory.outputPixels,
			entry.stages.resize.meanMs,
			quantizeLoopMean(entry),
			entry.stages.quantize.meanMs,
			...quantizeSubstageNames.map((name) => entry.quantizeSubstages[name]?.meanMs ?? ''),
			...quantizeCounterNames.map((name) => entry.quantizeCounters[name]?.meanMs ?? ''),
			entry.stages.previewRender.meanMs,
			entry.stages.pngEncode.meanMs,
			entry.stages.total.meanMs,
			entry.hotspot,
			entry.quantizeHotspot ?? '',
			entry.memory.resizedRgbaBytes,
			entry.memory.indexBytes,
			entry.memory.errorWorkBufferBytes,
			entry.memory.pngBytes ?? ''
		])
	];
	return `${rows.map((row) => row.map(csvCell).join(',')).join('\n')}\n`;
}

export function formatBenchmarkTable(result: BenchmarkResult): string {
	const rows = result.results.map((entry) => [
		entry.source.id,
		entry.case.id,
		formatPixels(entry.memory.sourcePixels),
		formatPixels(entry.memory.outputPixels),
		formatMs(entry.stages.resize.meanMs),
		formatMs(quantizeLoopMean(entry)),
		formatMs(
			quantizeMean(entry, 'color space convert palette') +
				quantizeMean(entry, 'color space convert composited image') +
				quantizeMean(entry, 'color space convert source image')
		),
		formatMs(
			quantizeMean(entry, 'color space convert composited image') +
				quantizeMean(entry, 'color space convert source image')
		),
		formatMs(quantizeMean(entry, 'color space convert palette')),
		formatMs(quantizeLoopMean(entry)),
		formatCount(
			quantizeCountMean(entry, 'rgb memo hit') + quantizeCountMean(entry, 'vector memo hit')
		),
		formatMs(entry.stages.previewRender.meanMs),
		formatMs(entry.stages.pngEncode.meanMs),
		formatMs(entry.stages.total.meanMs),
		entry.hotspot,
		entry.quantizeHotspot ?? '—'
	]);
	return formatTable([
		[
			'source',
			'case',
			'src px',
			'out px',
			'resize',
			'quantize',
			'q convert',
			'q image',
			'q palette',
			'q loop',
			'q memo hits',
			'preview',
			'png',
			'total',
			'hotspot',
			'q hotspot'
		],
		...rows
	]);
}

function csvCell(value: unknown) {
	const text = String(value);
	return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

function formatMs(value: number) {
	return `${value.toFixed(value >= 100 ? 0 : 1)}ms`;
}

function formatCount(value: number) {
	if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
	if (value >= 1_000) return `${Math.round(value / 1_000)}K`;
	return String(Math.round(value));
}

function formatPixels(value: number) {
	if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}MP`;
	if (value >= 1_000) return `${Math.round(value / 1_000)}K`;
	return String(value);
}

function formatTable(rows: string[][]) {
	const widths = rows[0]!.map((_, column) =>
		Math.max(...rows.map((row) => row[column]?.length ?? 0))
	);
	return rows
		.map((row) => row.map((cell, column) => cell.padEnd(widths[column]!)).join('  '))
		.join('\n');
}
