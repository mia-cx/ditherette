import {
	applyCompiledEffects,
	compileEffectMask,
	createGpuEffects,
	createGpuDither,
	DitheretteError,
	indexColours,
	type Curve,
	type Ditherette,
	type GpuEffects,
	type GpuDither,
	type IndexedColours,
	type Progress,
	type RecolourRecipe,
	type Rgba8Image
} from 'ditherette';
import type { LiveEffectsRequest, LiveEffectsResponse } from './live-effects';
import type { ProcessingStageTiming } from './metrics';
import { packageProcessRequest, packageQuantizeResult } from './package-adapter';
import { measureFits, resolveRecipes, type AnalysedFit } from './recipes';
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
/** Pixels as either byte array: an `ImageData` or the package's `Rgba8Image`. */
type Pixels = { width: number; height: number; data: Uint8Array | Uint8ClampedArray };

/** A loaded source, its colour index once needed, and its latest effects result. */
type SourceCache = {
	sourceId: string;
	source: ImageData;
	colours?: IndexedColours;
	effects?: { key: string; mapped: Pixels };
};

const EFFECTS_MEMORY_BYTES = 512 * 1024 ** 2;
// RGB indexing has at most 2^24 entries. Reserve host staging and packed results
// outside the facade's own snapshot, decoder, upload and readback admission.
const COLOUR_HOST_BYTES = 2 * (1 << 24) * Uint32Array.BYTES_PER_ELEMENT;
const COLOUR_ROW = 4096;

/**
 * One serial effects session for Source and processing. The package selects whole-chain
 * lookup eligibility and permanently retires its GPU bindings after CPU fallback.
 * Source changes and cancellation invalidate publication without overlapping facade calls.
 */
export class EffectsWorkerPipeline {
	#loaded: { sourceId: number; image: Rgba8Image; colours: Uint32Array } | undefined;
	#analyses = new Map<string, RecolourRecipe | readonly Curve[]>();
	#gpu: Promise<GpuEffects> | undefined;
	#queue = Promise.resolve();
	#cancelledThrough = 0;
	#disposed = false;

	constructor(private post: (response: LiveEffectsResponse, transfer?: Transferable[]) => void) {}

	/** Handle source/cancel messages immediately; serialize all package calls behind active work. */
	handle(request: LiveEffectsRequest): Promise<void> {
		if (request.type === 'dispose')
			return this.dispose().then(() => this.post({ type: 'disposed' }));
		if (this.#disposed) return this.#queue;
		if (request.type === 'cancel') {
			this.#cancelledThrough = Math.max(this.#cancelledThrough, request.id);
			return Promise.resolve();
		}
		if (request.type === 'source') {
			const started = performance.now();
			const image = bytes(request.source);
			const { colours, indices } = indexColours(image);
			this.#loaded = { sourceId: request.sourceId, image, colours };
			this.#analyses = new Map();
			for (let pixel = 0; pixel < indices.length; pixel++)
				indices[pixel] = (indices[pixel]! | (image.data[pixel * 4 + 3]! << 24)) >>> 0;
			this.post(
				{
					type: 'indexed',
					sourceId: request.sourceId,
					pixels: indices,
					count: colours.length,
					timings: [{ name: 'source colour index', ms: performance.now() - started }]
				},
				[indices.buffer]
			);
			return Promise.resolve();
		}
		const loaded = this.#loaded;
		const analyses = this.#analyses;
		const current = () =>
			!this.#disposed &&
			loaded === this.#loaded &&
			loaded?.sourceId === request.sourceId &&
			(request.type !== 'compile' || request.id > this.#cancelledThrough);
		const { id, sourceId, key } = request;
		const queuedAt = performance.now();
		this.#queue = this.#queue.then(async () => {
			const timings: ProcessingStageTiming[] = [];
			const mark = (name: string, start: number) =>
				timings.push({ name, ms: performance.now() - start });
			mark('effects queue wait', queuedAt);
			let cpu: Ditherette | undefined;
			try {
				if (!current() || !loaded) return;
				const { effects, context, crop } = request;
				// Ordinary lookup compiles need no second, resident scalar processor.
				const needsCpu =
					request.type !== 'compile' ||
					effects.some(
						(step) =>
							step.enabled &&
							((step.effect === 'recolour' && step.recipe === null) ||
								(step.effect === 'palette-fit' && step.curves === null))
					);
				let started = performance.now();
				cpu = needsCpu ? await initializePackageProcessor() : undefined;
				if (needsCpu) mark('effects CPU initialization', started);
				if (!current()) return;
				const fits: AnalysedFit[] = [];
				started = performance.now();
				const resolved = cpu
					? resolveRecipes(cpu, loaded.image, effects, context, crop, analyses, fits)
					: effects;
				if (needsCpu) mark('effects recipe analysis', started);
				if (request.type === 'measure') {
					this.post({
						type: 'measured',
						id,
						sourceId,
						key,
						fits: measureFits(cpu!, loaded.image, effects, context, crop, analyses)
					});
					return;
				}
				if (request.type === 'mask') {
					const results = compileEffectMask(cpu!, {
						version: 1,
						colours: loaded.colours,
						effects: resolved,
						mask: request.mask,
						context
					});
					this.post({ type: 'masked', id, sourceId, key, results }, [results.buffer]);
					return;
				}
				// Resolved recipes own their curves; release analysis Wasm before GPU or CPU fallback.
				cpu?.dispose();
				cpu = undefined;
				const results = new Uint32Array(loaded.colours.length);
				let execution: GpuEffects['execution'];
				if (results.length) {
					started = performance.now();
					const width = Math.min(results.length, COLOUR_ROW);
					const height = Math.ceil(results.length / width);
					const data = new Uint8Array(width * height * 4);
					for (let pixel = 0; pixel < width * height; pixel++) {
						const colour = loaded.colours[Math.min(pixel, results.length - 1)]!;
						data[pixel * 4] = colour & 255;
						data[pixel * 4 + 1] = (colour >>> 8) & 255;
						data[pixel * 4 + 2] = (colour >>> 16) & 255;
						data[pixel * 4 + 3] = 255;
					}
					mark('effects colour packing', started);
					started = performance.now();
					this.#gpu ??= createGpuEffects({
						memoryLimitBytes: EFFECTS_MEMORY_BYTES - COLOUR_HOST_BYTES
					}).catch((error: unknown) => {
						this.#gpu = undefined;
						throw error;
					});
					const gpu = await this.#gpu;
					mark('effects package initialization wait', started);
					if (!current()) return;
					started = performance.now();
					const output = await gpu.applyEffects({
						version: 1,
						source: { width, height, data },
						effects: resolved,
						context
					});
					mark('effects package total', started);
					if (!current()) return;
					execution = gpu.execution;
					timings.push(...(execution?.timings ?? []));
					started = performance.now();
					for (let pixel = 0; pixel < results.length; pixel++)
						results[pixel] =
							output.data[pixel * 4]! |
							(output.data[pixel * 4 + 1]! << 8) |
							(output.data[pixel * 4 + 2]! << 16);
					mark('effects result packing', started);
				}
				this.post({ type: 'compiled', id, sourceId, key, results, fits, execution, timings }, [
					results.buffer
				]);
			} catch (error) {
				if (current())
					this.post({
						type:
							request.type === 'mask'
								? 'mask-failed'
								: request.type === 'measure'
									? 'measure-failed'
									: 'failed',
						id,
						sourceId,
						key,
						message: error instanceof Error ? error.message : 'Effects failed.'
					});
			} finally {
				cpu?.dispose();
				if (!current() && !this.#disposed)
					this.post({ type: 'cancelled', id, sourceId, key, operation: request.type });
			}
		});
		return this.#queue;
	}

	/** Stop publication now, then release package resources after any active readback finishes. */
	async dispose() {
		this.#disposed = true;
		await this.#queue;
		(await this.#gpu)?.dispose();
		this.#loaded = undefined;
		this.#analyses.clear();
		this.#gpu = undefined;
	}
}

export class ProcessorWorkerPipeline {
	#sourceCache: SourceCache | undefined;
	#canceledIds = new Set<number>();
	#package: Promise<Ditherette | GpuDither> | undefined;
	/** The latest initialization's stages, reported once by the next request that completes. */
	#startup: ProcessingStageTiming[] = [];
	#active: { id: number; controller: AbortController } | undefined;

	handle(request: Exclude<WorkerRequest, { type: 'process' }>): WorkerResponse | undefined {
		if (request.type === 'cancel') {
			this.#canceledIds.add(request.id);
			if (this.#active?.id === request.id) this.#active.controller.abort();
			return undefined;
		}
		if (this.#canceledIds.has(request.id)) return undefined;
		this.#sourceCache = { sourceId: request.sourceId, source: request.source };
		return { id: request.id, type: 'source-loaded', sourceId: request.sourceId };
	}

	/**
	 * Process one request. Effects arrive compiled from the effects worker; the source maps through
	 * them once and the mapped source is reused until they change.
	 */
	async handleAsync(
		request: WorkerRequest,
		progress: ProgressSink
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
		this.#package ??= initializePackageProcessor(true).then(
			(processor) => {
				this.#startup = startupTimings(initializeStart, performance.now());
				return processor;
			},
			(error: unknown) => {
				this.#package = undefined;
				throw error;
			}
		);
		const processor = await this.#package;
		if (this.#canceledIds.has(id)) return undefined;
		mark('package initialisation wait', initializeStart);
		timings.push(...this.#startup.splice(0));
		const source = applyEffects(cache, request, mark);
		const requestStart = performance.now();
		// The mapped source already carries the effects, so the package only resizes and dithers.
		const mapped = packageProcessRequest(source, palette, { ...settings, effects: [] }, size);
		mark('package request adapter', requestStart);
		const processStart = performance.now();
		const stages: ProcessingStageTiming[] = [];
		let stageName = 'prepare';
		let stageStart = processStart;
		const controller = new AbortController();
		this.#active = { id, controller };
		let output;
		try {
			const processRequest = {
				...mapped.request,
				onProgress({ stage, completed, total }: Progress) {
					if (stage !== stageName) {
						const now = performance.now();
						if (stageName !== 'complete')
							stages.push({ name: `package ${stageName}`, ms: now - stageStart });
						stageName = stage;
						stageStart = now;
					}
					progress(stage, stage === 'complete' ? 1 : total ? (completed ?? 0) / total : 0, {
						completed,
						total
					});
				}
			};
			output =
				'execution' in processor
					? await processor.process(processRequest, controller.signal)
					: processor.process(processRequest);
		} catch (error) {
			if (this.#canceledIds.has(id)) return undefined;
			if (
				'execution' in processor &&
				error instanceof DitheretteError &&
				(error.code === 'initialization' || error.code === 'capability')
			) {
				processor.dispose();
				this.#package = undefined;
				throw new PackageInitializationError('Wasm could not initialise. Try processing again.', {
					cause: error
				});
			}
			throw error;
		} finally {
			if (this.#active?.id === id) this.#active = undefined;
		}
		if (this.#canceledIds.has(id)) return undefined;
		mark('package process', processStart);
		const adapterStart = performance.now();
		const execution = 'execution' in processor ? processor.execution : { backend: 'cpu' as const };
		if (execution && 'timings' in execution) timings.push(...(execution.timings ?? []));
		if (execution?.backend === 'cpu') timings.push(...stages);
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
				execution,
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
 * The source with the request's compiled effects applied, byte-exact: each pixel takes its
 * colour's result. Cached by the effects key until the next edit.
 */
function applyEffects(
	cache: SourceCache,
	{ settings, compiledEffects }: WorkerProcessRequest,
	mark: (name: string, start: number) => void
): Pixels {
	if (!settings.effects.length) return cache.source;
	if (!compiledEffects) throw new Error('Effects arrived without their compiled results.');
	const { key, results } = compiledEffects;
	if (cache.effects?.key === key) return cache.effects.mapped;
	let start = performance.now();
	cache.colours ??= indexColours(bytes(cache.source));
	mark('effects colours', start);
	if (results.length !== cache.colours.colours.length)
		throw new Error('The compiled effects are for another image.');
	start = performance.now();
	// The previous result is stale now, so its buffer takes the new one.
	const mapped = applyCompiledEffects(
		bytes(cache.source),
		cache.colours.indices,
		results,
		cache.effects && bytes(cache.effects.mapped).data
	);
	mark('effects map', start);
	cache.effects = { key, mapped };
	return mapped;
}

/**
 * Package initialization split by the browser's resource timings for what it fetched: its
 * module scripts, its Wasm, then compiling and instantiating once the Wasm arrived. Streaming
 * compilation overlaps the Wasm download, so the last stage is only what remained after it.
 */
function startupTimings(startedAt: number, readyAt: number): ProcessingStageTiming[] {
	const fetched = (performance.getEntriesByType('resource') as PerformanceResourceTiming[]).filter(
		(entry) => entry.startTime >= startedAt && entry.responseEnd <= readyAt
	);
	const wasm = fetched.filter((entry) => new URL(entry.name).pathname.endsWith('.wasm'));
	const scripts = fetched.filter((entry) => !wasm.includes(entry));
	const span = (name: string, entries: PerformanceResourceTiming[]) =>
		entries.length
			? [
					{
						name,
						ms:
							Math.max(...entries.map((entry) => entry.responseEnd)) -
							Math.min(...entries.map((entry) => entry.startTime))
					}
				]
			: [];
	const wasmArrivedAt = Math.max(...wasm.map((entry) => entry.responseEnd));
	return [
		...span('module download', scripts),
		...span('Wasm download', wasm),
		...(wasm.length ? [{ name: 'Wasm compile + instantiate', ms: readyAt - wasmArrivedAt }] : [])
	];
}

/** An image's pixels as the package's byte view, sharing memory. */
const bytes = ({ width, height, data }: Pixels) => ({
	width,
	height,
	data: new Uint8Array(data.buffer, data.byteOffset, data.length)
});

/** Signals that a fresh worker must clear cached module or Wasm initialization failures. */
export class PackageInitializationError extends Error {}

/** Load the requested processor, marking initialization failures a fresh worker may clear. */
export function initializePackageProcessor(): Promise<Ditherette>;
export function initializePackageProcessor(gpu: true): Promise<Ditherette | GpuDither>;
export async function initializePackageProcessor(gpu = false) {
	const message = 'Wasm could not initialise. Try processing again.';
	let module;
	try {
		module = await import('ditherette');
	} catch (error) {
		throw new PackageInitializationError(message, { cause: error });
	}
	try {
		// The editor accepts visually similar diffusion for parallel GPU speed.
		if (gpu && typeof navigator !== 'undefined' && 'gpu' in navigator)
			return await createGpuDither({ diffusionBackend: 'gpu-approximate' });
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
 * Transfer completed indexed output with its preview bitmaps; control responses keep their
 * owned data.
 */
export function transferablesForWorkerResponse(response: WorkerResponse): Transferable[] {
	const owned = (buffer: ArrayBufferLike) => (buffer instanceof ArrayBuffer ? [buffer] : []);
	if (response.type !== 'complete') return [];
	return [...owned(response.image.indices.buffer), ...(response.preview ?? [])];
}
