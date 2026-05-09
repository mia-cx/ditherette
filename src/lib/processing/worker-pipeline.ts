import { resizeImageData } from '$lib/processing/resize';
import {
	quantizeImage,
	type PaletteVectorSpace,
	type QuantizeCaches,
	type QuantizeResult
} from '$lib/processing/quantize';
import {
	deltaCacheSnapshot,
	estimateProcessingMemory,
	mergeCacheSnapshots,
	type ProcessingCacheSnapshot,
	type ProcessingStageTiming
} from './metrics';
import { clampOutputSize, type WorkerRequest, type WorkerResponse } from './types';
import {
	IDENTITY_GRADE_KEY,
	PaletteVectorCache,
	pipelineBranchKey,
	PipelineBranchCache
} from './worker-cache';

const PROGRESS = {
	queued: 0.05,
	resizing: 0.18,
	cacheHit: 0.22,
	quantizing: 0.62,
	quantizeCacheHit: 0.82,
	finalizing: 0.95
} as const;

type SourceCache = {
	sourceId: string;
	source: ImageData;
};

type ProgressSink = (stage: string, progress: number) => void;

type TimingSink = {
	values: ProcessingStageTiming[];
	add(name: string, ms: number, replayed?: boolean): void;
	mark(name: string, start: number): number;
	replay(name: string, ms: number | undefined): void;
};

export class ProcessorWorkerPipeline {
	#sourceCache: SourceCache | undefined;
	#branchCache = new PipelineBranchCache();
	#paletteVectorCache = new PaletteVectorCache<PaletteVectorSpace>();
	#canceledIds = new Set<number>();

	get branchCacheSize() {
		return this.#branchCache.size;
	}

	get resizeCacheSize() {
		return this.branchCacheSize;
	}

	handle(request: WorkerRequest, progress: ProgressSink): WorkerResponse | undefined {
		if (request.type === 'cancel') {
			this.#canceledIds.add(request.id);
			return undefined;
		}

		if (this.#canceledIds.has(request.id)) return undefined;

		if (request.type === 'load-source') {
			this.#sourceCache = { sourceId: request.sourceId, source: request.source };
			this.#branchCache.clear();
			return { id: request.id, type: 'source-loaded', sourceId: request.sourceId };
		}

		const startedAt = performance.now();
		const timings = timingSink();
		const cacheBefore = this.cacheSnapshot();
		const { id, sourceId, settings, palette, settingsHash } = request;
		const source = this.sourceFor(sourceId);
		progress('Sizing output', PROGRESS.queued);
		const sizingStart = performance.now();
		const size = clampOutputSize(settings.output.width, settings.output.height);
		const branchKey = pipelineBranchKey({
			sourceId,
			width: size.width,
			height: size.height,
			resize: settings.output.resize,
			crop: settings.output.crop,
			gradeKey: IDENTITY_GRADE_KEY
		});
		timings.mark('worker sizing', sizingStart);
		const resizeLookupStart = performance.now();
		let resized = this.#branchCache.getResized(branchKey);
		timings.mark('resize cache lookup', resizeLookupStart);
		if (resized) {
			progress('Using cached resize', PROGRESS.cacheHit);
			timings.replay('resize compute', this.#branchCache.getResizedTiming(branchKey));
		} else {
			progress('Resizing', PROGRESS.resizing);
			const resizeStart = performance.now();
			resized = resizeImageData(
				source,
				size.width,
				size.height,
				settings.output.resize,
				settings.output.crop
			);
			const resizeMs = timings.mark('resize compute', resizeStart);
			const resizeCacheWriteStart = performance.now();
			this.#branchCache.setResized(branchKey, resized, resizeMs);
			timings.mark('resize cache write', resizeCacheWriteStart);
		}
		const quantizeCacheLookupStart = performance.now();
		const cachedResult = this.cachedQuantizeResult(branchKey, settingsHash);
		timings.mark('quantize cache lookup', quantizeCacheLookupStart);
		let result = cachedResult?.result;
		if (result) {
			progress('Using cached quantization', PROGRESS.quantizeCacheHit);
			timings.replay('quantize compute', cachedResult?.timingMs);
		} else {
			progress('Quantizing palette', PROGRESS.quantizing);
			const quantizeStart = performance.now();
			result = quantizeImage(resized, palette, settings, this.quantizeCaches(branchKey));
			const quantizeMs = timings.mark('quantize compute', quantizeStart);
			const quantizeCacheWriteStart = performance.now();
			this.cacheQuantizeResult(branchKey, settingsHash, result, quantizeMs);
			timings.mark('quantize cache write', quantizeCacheWriteStart);
		}
		const finalizingStart = performance.now();
		const warnings = size.warning ? [size.warning, ...result.warnings] : result.warnings;
		progress('Finalizing indexed output', PROGRESS.finalizing);
		const completedAt = performance.now();
		const cacheAfter = this.cacheSnapshot();
		const memory = estimateProcessingMemory({
			sourceWidth: source.width,
			sourceHeight: source.height,
			outputWidth: size.width,
			outputHeight: size.height,
			settings,
			branchCacheBytes: cacheAfter.branchBytes,
			branchCacheMaxBytes: cacheAfter.branchMaxBytes
		});
		timings.mark('worker finalizing', finalizingStart);
		return {
			id,
			type: 'complete',
			image: {
				...result,
				width: size.width,
				height: size.height,
				warnings,
				settingsHash,
				updatedAt: Date.now()
			},
			metrics: {
				id,
				settingsHash,
				sourceId,
				scopeKey: branchKey,
				startedAt,
				completedAt,
				totalMs: completedAt - startedAt,
				timings: timings.values,
				cache: {
					delta: deltaCacheSnapshot(cacheBefore, cacheAfter),
					lifetime: cacheAfter
				},
				memory,
				outputPixels: size.width * size.height,
				colorSpace: settings.colorSpace,
				dither: settings.dither.algorithm,
				resize: settings.output.resize,
				warnings
			}
		};
	}

	private cachedQuantizeResult(
		branchKey: string,
		settingsHash: string
	): { result: QuantizeResult; timingMs?: number } | undefined {
		const key = quantizeResultCacheKey(settingsHash);
		const cached = this.#branchCache.getColorMapping<QuantizeResult>(branchKey, key);
		if (!cached) return undefined;
		return {
			result: cloneQuantizeResult(cached),
			timingMs: this.#branchCache.getColorMappingTiming(branchKey, key)
		};
	}

	private cacheQuantizeResult(
		branchKey: string,
		settingsHash: string,
		result: QuantizeResult,
		timingMs?: number
	) {
		this.#branchCache.setColorMapping(
			branchKey,
			quantizeResultCacheKey(settingsHash),
			cloneQuantizeResult(result),
			result.indices.byteLength,
			timingMs
		);
	}

	private quantizeCaches(branchKey: string): QuantizeCaches {
		return {
			getPaletteVectorSpace: (key) => this.#paletteVectorCache.get(key),
			setPaletteVectorSpace: (key, value) => this.#paletteVectorCache.set(key, value),
			colorVectorImageScope: branchKey,
			getColorVectorImage: (key) => this.#branchCache.getColorMapping(branchKey, key),
			canStoreColorVectorImage: (_key, bytes) =>
				this.#branchCache.canStoreColorMapping(branchKey, bytes),
			setColorVectorImage: (key, value, bytes) =>
				this.#branchCache.setColorMapping(branchKey, key, value, bytes)
		};
	}

	sourceFor(sourceId: string) {
		if (!this.#sourceCache || this.#sourceCache.sourceId !== sourceId) {
			throw new Error('Worker source is not loaded.');
		}
		return this.#sourceCache.source;
	}

	private cacheSnapshot(): ProcessingCacheSnapshot {
		const source = this.#sourceCache?.source;
		return mergeCacheSnapshots(
			Boolean(source),
			source ? source.width * source.height * 4 : 0,
			this.#branchCache.snapshotMetrics(),
			this.#paletteVectorCache.snapshotMetrics()
		);
	}
}

function timingSink(): TimingSink {
	return {
		values: [],
		add(name, ms, replayed = false) {
			if (!Number.isFinite(ms)) return;
			this.values.push({ name, ms: Math.max(0, ms), replayed: replayed || undefined });
		},
		mark(name, start) {
			const ms = performance.now() - start;
			this.add(name, ms);
			return ms;
		},
		replay(name, ms) {
			if (ms === undefined) return;
			this.add(name, ms, true);
		}
	};
}

function quantizeResultCacheKey(settingsHash: string) {
	return `quantized|${settingsHash}`;
}

function cloneQuantizeResult(result: QuantizeResult): QuantizeResult {
	return {
		indices: new Uint8Array(result.indices),
		palette: result.palette.slice(),
		transparentIndex: result.transparentIndex,
		warnings: result.warnings.slice()
	};
}

export function transferablesForWorkerResponse(response: WorkerResponse): Transferable[] {
	if (response.type !== 'complete') return [];
	const buffer = response.image.indices.buffer;
	return buffer instanceof ArrayBuffer ? [buffer] : [];
}
