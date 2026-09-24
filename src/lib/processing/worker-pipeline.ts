import type { Ditherette, Progress } from 'ditherette';
import { timingSink } from './metrics';
import { packageProcessRequest, packageQuantizeResult } from './package-adapter';
import { clampOutputSize, type WorkerRequest, type WorkerResponse } from './types';

type ProgressSink = (
	stage: string,
	progress: number,
	counts?: Pick<Progress, 'completed' | 'total'>
) => void;

export class ProcessorWorkerPipeline {
	#sourceCache: { sourceId: string; source: ImageData } | undefined;
	#canceledIds = new Set<number>();
	#package: Promise<Ditherette> | undefined;

	handle(request: Exclude<WorkerRequest, { type: 'process' }>): WorkerResponse | undefined {
		if (request.type === 'cancel') {
			this.#canceledIds.add(request.id);
			return undefined;
		}
		if (this.#canceledIds.has(request.id)) return undefined;
		this.#sourceCache = { sourceId: request.sourceId, source: request.source };
		return { id: request.id, type: 'source-loaded', sourceId: request.sourceId };
	}

	async handleAsync(
		request: WorkerRequest,
		progress: ProgressSink
	): Promise<WorkerResponse | undefined> {
		if (request.type !== 'process') return this.handle(request);
		if (this.#canceledIds.has(request.id)) return undefined;
		const startedAt = performance.now();
		const timings = timingSink();
		const { id, sourceId, settings, palette, settingsHash } = request;
		if (!this.#sourceCache || this.#sourceCache.sourceId !== sourceId)
			throw new Error('Worker source is not loaded.');
		progress('Sizing output', 0.05);
		const size = clampOutputSize(settings.output.width, settings.output.height);
		const mapped = packageProcessRequest(this.#sourceCache.source, palette, settings, size);
		timings.mark('package request adapter', startedAt);
		const initializeStart = performance.now();
		this.#package ??= initializePackageProcessor().catch((error: unknown) => {
			this.#package = undefined;
			throw error;
		});
		const processor = await this.#package;
		if (this.#canceledIds.has(id)) return undefined;
		timings.mark('package initialization wait', initializeStart);
		const processStart = performance.now();
		const output = processor.process({
			...mapped.request,
			onProgress({ stage, completed, total }) {
				progress(stage, stage === 'complete' ? 1 : total ? (completed ?? 0) / total : 0, {
					completed,
					total
				});
			}
		});
		timings.mark('package process', processStart);
		const adapterStart = performance.now();
		const result = packageQuantizeResult(output, palette, mapped.warnings);
		const warnings = size.warning ? [size.warning, ...result.warnings] : result.warnings;
		timings.mark('package output adapter', adapterStart);
		const completedAt = performance.now();
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
				scopeKey: `package|${sourceId}|${size.width}x${size.height}|${settings.output.resize}|${JSON.stringify(settings.output.crop)}`,
				startedAt,
				completedAt,
				totalMs: completedAt - startedAt,
				timings: timings.values,
				outputPixels: size.width * size.height,
				colorSpace: settings.colorSpace,
				dither: settings.dither.algorithm,
				resize: settings.output.resize,
				warnings
			}
		};
	}
}

/** Signals that a fresh worker must clear cached module or Wasm initialization failures. */
export class PackageInitializationError extends Error {}

async function initializePackageProcessor() {
	const message = 'Wasm could not initialize. Try processing again.';
	let module;
	try {
		module = await import('ditherette');
	} catch (error) {
		throw new PackageInitializationError(message, { cause: error });
	}
	try {
		return await module.createDitherette();
	} catch (error) {
		if (
			error instanceof module.DitheretteError &&
			(error.code === 'initialization' || error.code === 'capability')
		)
			throw new PackageInitializationError(message, { cause: error });
		throw error;
	}
}

/** Transfer only completed indexed output; control responses keep their owned data. */
export function transferablesForWorkerResponse(response: WorkerResponse): Transferable[] {
	if (response.type !== 'complete') return [];
	const buffer = response.image.indices.buffer;
	return buffer instanceof ArrayBuffer ? [buffer] : [];
}
