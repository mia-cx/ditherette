import { WPLACE_PALETTE } from '$lib/palette/wplace';
import { createPaletteMatcher, type PaletteMatcherOptions } from '$lib/processing/color';
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
	| 'unrolled-kernel';

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
	'unrolled-kernel'
];
const RGB24_SIZE = 256 * 256 * 256;

export function runPaletteStudy(options: PaletteStudyOptions): PaletteStudyResult {
	const iterations = Math.max(1, Math.floor(options.iterations ?? 3));
	const warmups = Math.max(0, Math.floor(options.warmups ?? 1));
	const studies = options.studies?.length ? options.studies : DEFAULT_STUDIES;
	const variants = options.variants?.length ? options.variants : DEFAULT_VARIANTS;
	const colorSpaces = options.colorSpaces?.length ? options.colorSpaces : COLOR_SPACES;
	const rows: PaletteStudyRow[] = [];

	for (const source of options.sources) {
		for (const study of studies) {
			if (study === 'diffusion-kernel-only') {
				const dithers = options.dithers?.length ? options.dithers : DEFAULT_DIFFUSION_DITHERS;
				const kernelVariants = options.variants?.length
					? options.variants
					: DIFFUSION_KERNEL_VARIANTS;
				for (const dither of dithers) {
					if (!DEFAULT_DIFFUSION_DITHERS.includes(dither)) continue;
					for (const variant of kernelVariants) {
						if (variant !== 'generic-kernel' && variant !== 'unrolled-kernel') continue;
						for (let warmup = 0; warmup < warmups; warmup++) {
							runDiffusionKernelStudy(source, dither, variant);
						}
						const runs = Array.from({ length: iterations }, () =>
							runDiffusionKernelStudy(source, dither, variant)
						);
						rows.push(meanDirectRows(runs));
					}
				}
				continue;
			}
			if (study !== 'direct-byte-rgb' && study !== 'cache-study' && study !== 'matcher') {
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
