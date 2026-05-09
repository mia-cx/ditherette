import { resizeImageData } from '$lib/processing/resize';
import {
	quantizeImage,
	type PaletteVectorSpace,
	type QuantizeCaches,
	type QuantizeResult
} from '$lib/processing/quantize';
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

		const { id, sourceId, settings, palette, settingsHash } = request;
		const source = this.sourceFor(sourceId);
		progress('Sizing output', PROGRESS.queued);
		const size = clampOutputSize(settings.output.width, settings.output.height);
		const branchKey = pipelineBranchKey({
			sourceId,
			width: size.width,
			height: size.height,
			resize: settings.output.resize,
			crop: settings.output.crop,
			gradeKey: IDENTITY_GRADE_KEY
		});
		let resized = this.#branchCache.getResized(branchKey);
		if (resized) {
			progress('Using cached resize', PROGRESS.cacheHit);
		} else {
			progress('Resizing', PROGRESS.resizing);
			resized = resizeImageData(
				source,
				size.width,
				size.height,
				settings.output.resize,
				settings.output.crop
			);
			this.#branchCache.setResized(branchKey, resized);
		}
		let result = this.cachedQuantizeResult(branchKey, settingsHash);
		if (result) {
			progress('Using cached quantization', PROGRESS.quantizeCacheHit);
		} else {
			progress('Quantizing palette', PROGRESS.quantizing);
			result = quantizeImage(resized, palette, settings, this.quantizeCaches(branchKey));
			this.cacheQuantizeResult(branchKey, settingsHash, result);
		}
		const warnings = size.warning ? [size.warning, ...result.warnings] : result.warnings;
		progress('Finalizing indexed output', PROGRESS.finalizing);
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
			}
		};
	}

	private cachedQuantizeResult(
		branchKey: string,
		settingsHash: string
	): QuantizeResult | undefined {
		const cached = this.#branchCache.getColorMapping<QuantizeResult>(
			branchKey,
			quantizeResultCacheKey(settingsHash)
		);
		if (!cached) return undefined;
		return cloneQuantizeResult(cached);
	}

	private cacheQuantizeResult(branchKey: string, settingsHash: string, result: QuantizeResult) {
		this.#branchCache.setColorMapping(
			branchKey,
			quantizeResultCacheKey(settingsHash),
			cloneQuantizeResult(result),
			result.indices.byteLength
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
