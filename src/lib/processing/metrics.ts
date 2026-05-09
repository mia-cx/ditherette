import type { ColorSpaceId, DitherId, ProcessingSettings, ResizeId } from './types';

export const METRICS_HISTORY_LIMIT = 100;

export type ProcessingStageTiming = {
	name: string;
	ms: number;
	replayed?: boolean;
};

export type PipelineCacheSnapshot = {
	branchCount: number;
	branchBytes: number;
	branchMaxBytes: number;
	resizedHits: number;
	resizedMisses: number;
	resizedSets: number;
	resizedSkips: number;
	resizedEvictions: number;
	derivedHits: number;
	derivedMisses: number;
	derivedSets: number;
	derivedSkips: number;
	derivedEvictions: number;
};

export type PaletteVectorCacheSnapshot = {
	entries: number;
	maxEntries: number;
	hits: number;
	misses: number;
	sets: number;
	evictions: number;
};

export type ProcessingCacheSnapshot = PipelineCacheSnapshot & {
	sourceLoaded: boolean;
	sourceBytes: number;
	paletteVectorEntries: number;
	paletteVectorMaxEntries: number;
	paletteVectorHits: number;
	paletteVectorMisses: number;
	paletteVectorSets: number;
	paletteVectorEvictions: number;
};

export type ProcessingCacheMetrics = {
	delta: ProcessingCacheSnapshot;
	lifetime: ProcessingCacheSnapshot;
};

export type ProcessingMemoryShape = {
	sourceBytes: number;
	resizedBytes: number;
	indexBytes: number;
	vectorBytes: number;
	ditherWorkBytes: number;
	branchCacheBytes: number;
	branchCacheMaxBytes: number;
};

export type ProcessingMetricsSample = {
	id: number;
	settingsHash: string;
	sourceId: string;
	scopeKey: string;
	startedAt: number;
	completedAt: number;
	totalMs: number;
	timings: ProcessingStageTiming[];
	cache: ProcessingCacheMetrics;
	memory: ProcessingMemoryShape;
	outputPixels: number;
	colorSpace: ColorSpaceId;
	dither: DitherId;
	resize: ResizeId;
	warnings: string[];
};

export type TimingSummary = {
	name: string;
	count: number;
	latest: number;
	average: number;
	p95: number;
	p98: number;
	p99: number;
};

type MemoryInput = {
	sourceWidth: number;
	sourceHeight: number;
	outputWidth: number;
	outputHeight: number;
	settings: ProcessingSettings;
	branchCacheBytes: number;
	branchCacheMaxBytes: number;
};

const ERROR_DIFFUSION_DITHERS = new Set<DitherId>(['floyd-steinberg', 'sierra', 'sierra-lite']);

export function imageBytes(width: number, height: number, bytesPerPixel: number) {
	return Math.max(0, width) * Math.max(0, height) * bytesPerPixel;
}

export function vectorImageBytes(width: number, height: number) {
	return imageBytes(width, height, 3 * Float32Array.BYTES_PER_ELEMENT);
}

export function estimateProcessingMemory({
	sourceWidth,
	sourceHeight,
	outputWidth,
	outputHeight,
	settings,
	branchCacheBytes,
	branchCacheMaxBytes
}: MemoryInput): ProcessingMemoryShape {
	const outputPixels = outputWidth * outputHeight;
	const ditherWorkBytes = ERROR_DIFFUSION_DITHERS.has(settings.dither.algorithm)
		? outputPixels * 3 * Float32Array.BYTES_PER_ELEMENT
		: 0;
	return {
		sourceBytes: imageBytes(sourceWidth, sourceHeight, 4),
		resizedBytes: imageBytes(outputWidth, outputHeight, 4),
		indexBytes: outputPixels,
		vectorBytes: vectorImageBytes(outputWidth, outputHeight),
		ditherWorkBytes,
		branchCacheBytes,
		branchCacheMaxBytes
	};
}

export function mergeCacheSnapshots(
	sourceLoaded: boolean,
	sourceBytes: number,
	pipeline: PipelineCacheSnapshot,
	palette: PaletteVectorCacheSnapshot
): ProcessingCacheSnapshot {
	return {
		sourceLoaded,
		sourceBytes,
		...pipeline,
		paletteVectorEntries: palette.entries,
		paletteVectorMaxEntries: palette.maxEntries,
		paletteVectorHits: palette.hits,
		paletteVectorMisses: palette.misses,
		paletteVectorSets: palette.sets,
		paletteVectorEvictions: palette.evictions
	};
}

export function deltaCacheSnapshot(
	before: ProcessingCacheSnapshot,
	after: ProcessingCacheSnapshot
): ProcessingCacheSnapshot {
	return {
		sourceLoaded: after.sourceLoaded,
		sourceBytes: after.sourceBytes,
		branchCount: after.branchCount,
		branchBytes: after.branchBytes,
		branchMaxBytes: after.branchMaxBytes,
		resizedHits: after.resizedHits - before.resizedHits,
		resizedMisses: after.resizedMisses - before.resizedMisses,
		resizedSets: after.resizedSets - before.resizedSets,
		resizedSkips: after.resizedSkips - before.resizedSkips,
		resizedEvictions: after.resizedEvictions - before.resizedEvictions,
		derivedHits: after.derivedHits - before.derivedHits,
		derivedMisses: after.derivedMisses - before.derivedMisses,
		derivedSets: after.derivedSets - before.derivedSets,
		derivedSkips: after.derivedSkips - before.derivedSkips,
		derivedEvictions: after.derivedEvictions - before.derivedEvictions,
		paletteVectorEntries: after.paletteVectorEntries,
		paletteVectorMaxEntries: after.paletteVectorMaxEntries,
		paletteVectorHits: after.paletteVectorHits - before.paletteVectorHits,
		paletteVectorMisses: after.paletteVectorMisses - before.paletteVectorMisses,
		paletteVectorSets: after.paletteVectorSets - before.paletteVectorSets,
		paletteVectorEvictions: after.paletteVectorEvictions - before.paletteVectorEvictions
	};
}

export function appendMetricsSample(
	history: ProcessingMetricsSample[],
	sample: ProcessingMetricsSample,
	limit = METRICS_HISTORY_LIMIT
) {
	const scopedHistory = history.at(-1)?.scopeKey === sample.scopeKey ? history : [];
	return [...scopedHistory, sample].slice(-limit);
}

export function summarizeTimingHistory(
	samples: readonly ProcessingMetricsSample[]
): TimingSummary[] {
	const valuesByName = new Map<string, number[]>();
	for (const sample of samples) {
		pushTiming(valuesByName, 'total', sample.totalMs);
		for (const timing of sample.timings) pushTiming(valuesByName, timing.name, timing.ms);
	}
	return [...valuesByName.entries()]
		.map(([name, values]) => summarizeValues(name, values))
		.sort((left, right) =>
			left.name === 'total' ? -1 : right.name === 'total' ? 1 : left.name.localeCompare(right.name)
		);
}

function pushTiming(valuesByName: Map<string, number[]>, name: string, value: number) {
	if (!Number.isFinite(value)) return;
	const values = valuesByName.get(name) ?? [];
	values.push(Math.max(0, value));
	valuesByName.set(name, values);
}

function summarizeValues(name: string, values: number[]): TimingSummary {
	const sorted = [...values].sort((left, right) => left - right);
	return {
		name,
		count: values.length,
		latest: values.at(-1) ?? 0,
		average: values.reduce((sum, value) => sum + value, 0) / Math.max(1, values.length),
		p95: percentile(sorted, 0.95),
		p98: percentile(sorted, 0.98),
		p99: percentile(sorted, 0.99)
	};
}

export function percentile(sortedValues: readonly number[], percentileValue: number) {
	if (sortedValues.length === 0) return 0;
	const index = Math.min(
		sortedValues.length - 1,
		Math.max(0, Math.ceil(sortedValues.length * percentileValue) - 1)
	);
	return sortedValues[index] ?? 0;
}
