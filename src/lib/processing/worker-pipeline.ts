import type { Ditherette, Progress } from 'ditherette';
import {
	applyEffectsTable,
	compileEffectsTable,
	distinctColours,
	resolveRecolour
} from './effects-table';
import type { ProcessingStageTiming } from './metrics';
import {
	packageEffectContext,
	packageProcessRequest,
	packageQuantizeResult
} from './package-adapter';
import {
	clampOutputSize,
	type WorkerProcessRequest,
	type WorkerRequest,
	type WorkerResponse
} from './types';

type ProgressSink = (
	stage: string,
	progress: number,
	counts?: Pick<Progress, 'completed' | 'total'>
) => void;
type TableSink = (table: Uint32Array) => void;
type Pixels = Pick<ImageData, 'width' | 'height' | 'data'>;

/** A loaded source, its distinct colours once needed, and its latest effects result. */
type SourceCache = {
	sourceId: string;
	source: ImageData;
	colours?: Uint32Array;
	effects?: { key: string; mapped: Pixels };
};

export class ProcessorWorkerPipeline {
	#sourceCache: SourceCache | undefined;
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

	/**
	 * Process one request. Effects run once, through an exact table of the source's colours, and
	 * the mapped source is reused until they change; each new table also goes to `publishTable`.
	 */
	async handleAsync(
		request: WorkerRequest,
		progress: ProgressSink,
		publishTable: TableSink = () => undefined
	): Promise<WorkerResponse | undefined> {
		if (request.type !== 'process') return this.handle(request);
		if (this.#canceledIds.has(request.id)) return undefined;
		const startedAt = performance.now();
		const timings: ProcessingStageTiming[] = [];
		const mark = (name: string, start: number) => {
			timings.push({ name, ms: Math.max(0, performance.now() - start) });
		};
		const { id, sourceId, settings, palette, settingsHash } = request;
		const cache = this.#sourceCache;
		if (!cache || cache.sourceId !== sourceId) throw new Error('Worker source is not loaded.');
		progress('Sizing output', 0.05);
		const size = clampOutputSize(settings.output.width, settings.output.height);
		const initializeStart = performance.now();
		this.#package ??= initializePackageProcessor().catch((error: unknown) => {
			this.#package = undefined;
			throw error;
		});
		const processor = await this.#package;
		if (this.#canceledIds.has(id)) return undefined;
		mark('package initialisation wait', initializeStart);
		const source = applyEffects(processor, cache, request, size, publishTable, mark);
		const requestStart = performance.now();
		// The mapped source already carries the effects, so the package only resizes and dithers.
		const mapped = packageProcessRequest(source, palette, { ...settings, effects: [] }, size);
		mark('package request adapter', requestStart);
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
		mark('package process', processStart);
		const adapterStart = performance.now();
		const result = packageQuantizeResult(output, palette, mapped.warnings);
		const warnings = size.warning ? [size.warning, ...result.warnings] : result.warnings;
		mark('package output adapter', adapterStart);
		const completedAt = performance.now();
		const crop = settings.output.crop;
		const cropKey = crop ? `${crop.x},${crop.y},${crop.width},${crop.height}` : '0,0,full,full';
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
				scopeKey: `package|${sourceId}|${size.width}x${size.height}|${settings.output.resize}|${cropKey}|effects:${settings.effects.map((step) => step.effect).join('+') || 'none'}`,
				startedAt,
				completedAt,
				totalMs: completedAt - startedAt,
				timings,
				outputPixels: size.width * size.height,
				colorSpace: settings.colorSpace,
				dither: settings.dither.algorithm,
				resize: settings.output.resize,
				warnings
			}
		};
	}
}

/**
 * The source with the request's effects applied through an exact table of its colours. The result
 * stays cached until the chain changes, or, with a palette fit, the palette or colour space, or,
 * with a palette fit that analyses the image, the crop. Each new table goes to `publishTable`.
 */
function applyEffects(
	processor: Ditherette,
	cache: SourceCache,
	{ settings, palette }: WorkerProcessRequest,
	size: { width: number; height: number },
	publishTable: TableSink,
	mark: (name: string, start: number) => void
): Pixels {
	if (!settings.effects.length) return cache.source;
	const context = packageEffectContext(palette, settings.colorSpace);
	// Only palette fit reads the palette and working space, and only one without a recipe
	// analyses the (cropped) image, so other edits keep the table.
	const fits = settings.effects.filter((step) => step.effect === 'recolour');
	const analysed = fits.some((step) => step.recipe === null);
	const key = JSON.stringify([
		settings.effects,
		fits.length > 0 && context,
		analysed && settings.output.crop
	]);
	if (cache.effects?.key === key) return cache.effects.mapped;
	let start = performance.now();
	// Palette fit analyses the cropped source `process` would receive.
	const cropped = packageProcessRequest(cache.source, palette, settings, size).request.source;
	const effects = resolveRecolour(processor, cropped, settings.effects, context);
	mark('palette fit analysis', start);
	start = performance.now();
	cache.colours ??= distinctColours(cache.source);
	mark('effects colours', start);
	start = performance.now();
	const table = compileEffectsTable(processor, cache.colours, effects, context);
	mark('effects compile', start);
	start = performance.now();
	// The previous result is stale now, so its buffer takes the new one.
	const mapped = applyEffectsTable(cache.source, table, cache.effects?.mapped.data);
	mark('effects map', start);
	cache.effects = { key, mapped };
	publishTable(table);
	return mapped;
}

/** Signals that a fresh worker must clear cached module or Wasm initialization failures. */
export class PackageInitializationError extends Error {}

/** Load the package and create a scalar processor, marking failures a fresh worker may clear. */
export async function initializePackageProcessor() {
	const message = 'Wasm could not initialise. Try processing again.';
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

/**
 * Transfer completed indexed output with its preview bitmaps, and effects tables; control
 * responses keep their owned data.
 */
export function transferablesForWorkerResponse(response: WorkerResponse): Transferable[] {
	const owned = (buffer: ArrayBufferLike) => (buffer instanceof ArrayBuffer ? [buffer] : []);
	if (response.type === 'effects-table') return owned(response.table.buffer);
	if (response.type !== 'complete') return [];
	return [...owned(response.image.indices.buffer), ...(response.preview ?? [])];
}
