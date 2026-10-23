import type { ColorSpaceId, DitherId, ResizeId } from './types';

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
	/**
	 * Recorded samples run from the user's action (an upload or restore starting, or the latest
	 * settings change scheduling) until the output painted. A worker response covers only its own
	 * work until the page replaces both.
	 */
	startedAt: number;
	completedAt: number;
	totalMs: number;
	timings: ProcessingStageTiming[];
	execution?: {
		backend: 'gpu' | 'hybrid' | 'cpu';
		adapter?: string;
		fallback?: string;
		analysisReused?: boolean;
	};
	cache?: ProcessingCacheMetrics;
	memory?: ProcessingMemoryShape;
	outputPixels: number;
	colorSpace: ColorSpaceId;
	dither: DitherId;
	resize: ResizeId;
	warnings: string[];
};

export type TimingSummary = {
	name: string;
	count: number;
	latest: number | undefined;
	average: number;
	p95: number;
	p98: number;
	p99: number;
};

export function appendMetricsSample(
	history: ProcessingMetricsSample[],
	sample: ProcessingMetricsSample,
	limit = METRICS_HISTORY_LIMIT
) {
	const scopedHistory = history.at(-1)?.scopeKey === sample.scopeKey ? history : [];
	return [...scopedHistory, sample].slice(-limit);
}

/** Summaries start with `total`, then follow first-emitted order so nested spans stay together. */
export function summarizeTimingHistory(
	samples: readonly ProcessingMetricsSample[]
): TimingSummary[] {
	const valuesByName = new Map<string, number[]>();
	for (const sample of samples) {
		pushTiming(valuesByName, 'total', sample.totalMs);
		for (const timing of sample.timings) pushTiming(valuesByName, timing.name, timing.ms);
	}
	return [...valuesByName.entries()].map(([name, values]) => ({
		...summarizeValues(name, values),
		latest:
			name === 'total'
				? samples.at(-1)?.totalMs
				: samples.at(-1)?.timings.find((timing) => timing.name === name)?.ms
	}));
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
