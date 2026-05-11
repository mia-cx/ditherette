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

export type PaletteStudyVariant = 'scan' | 'distance-tables' | 'dense-rgb-distance-tables';

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

function matcherOptionsForVariant(variant: PaletteStudyVariant): PaletteMatcherOptions {
	switch (variant) {
		case 'scan':
			return { denseRgbMemo: false, distanceTables: false };
		case 'distance-tables':
			return { denseRgbMemo: false, distanceTables: true };
		case 'dense-rgb-distance-tables':
			return { denseRgbMemo: true, distanceTables: true };
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
