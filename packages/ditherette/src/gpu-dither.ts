import { DitheretteError } from './errors.js';
import { executionTimings } from './timings.js';
import { normalizeWasmInput } from './scalar.js';
import { createCpu } from './cpu.js';
import {
	validateApplyEffects,
	validateAnalyzeRecolour,
	validateAnalyzePaletteFit
} from './validation-effects.js';
import {
	anchors,
	field,
	matchingModes,
	object,
	TRANSPARENT_CODE,
	validateOptions,
	validateQuantize,
	validateResize
} from './validation.js';
import {
	kernels,
	spaces,
	tiles,
	validateDitherAndQuantize,
	validatePerturb
} from './validation-fields.js';
import { validateProcess } from './validation-process.js';
import {
	fallbackPolicy,
	gpuFallbackError,
	gpuEffectsDecoderBytes,
	initializeGpuEffects,
	capturedEffects,
	type GpuEffectsBindings,
	type GpuEffectsOptions
} from './gpu-effects.js';
import type {
	DitherAndQuantizeRequest,
	ApplyEffectsRequest,
	EffectMaskRequest,
	CpuInitOptions,
	AnalyzeRecolourRequest,
	AnalyzePaletteFitRequest,
	FitLook,
	PaletteFitSpace,
	Ditherette,
	DiffusionKernel,
	Effect,
	Field,
	IndexedImage,
	InitInput,
	Matching,
	PaletteEntry,
	Placement,
	PerturbRequest,
	ProcessRequest,
	Progress,
	QuantizeRequest,
	ResizeAnchor,
	ResizeRequest,
	Rgba8Image,
	TilingPolicy,
	WorkingSpace
} from './types.js';

/** Optional GPU artifact and ordinary scalar fallback, with one combined memory limit. */
export interface GpuDitherOptions extends GpuEffectsOptions {
	/** Row bands for independent stages; workers applies to CPU fallback. Diffusion stays continuous. */
	readonly tiling?: TilingPolicy;
	/** Keep CPU recurrence (default), or execute continuous diffusion on the GPU. */
	readonly diffusionBackend?: 'cpu' | 'gpu';
}

/** Hybrid combines GPU preparation/Adaptive analysis with the CPU diffusion recurrence. */
export interface DitherExecution {
	readonly backend: 'gpu' | 'hybrid' | 'cpu';
	readonly adapter?: string;
	readonly fallback?: string;
	/** True when byte-verified source coverage bypassed the complete GPU selector. */
	readonly analysisReused?: boolean;
	/** Host wall times; nested GPU spans overlap and include synchronization. */
	readonly timings?: readonly import('./timings.js').ExecutionTiming[];
}

/** Keep the CPU operation surface available on the asynchronous engine. */
type AsyncProcessing = {
	[K in Exclude<keyof Ditherette, 'dispose'>]: (
		...args: Parameters<Ditherette[K]>
	) => Promise<ReturnType<Ditherette[K]>>;
};
/** Serial asynchronous processing. A GPU failure retires this facade's artifact. */
export interface GpuDither extends AsyncProcessing {
	readonly execution: DitherExecution | undefined;
	/** Effects share this processor's single device and retirement lifetime. */
	applyEffects(request: ApplyEffectsRequest): Promise<Rgba8Image>;
	/** Read a mask from the continuous effects carrier on the same device. */
	effectMask(request: EffectMaskRequest): Promise<Rgba8Image>;
	/** Recipe analysis uses CPU fallback; strict GPU policy rejects these calls. */
	analyzeRecolour(
		request: AnalyzeRecolourRequest
	): Promise<ReturnType<Ditherette['analyzeRecolour']>>;
	analyzePaletteFit(
		request: AnalyzePaletteFitRequest
	): Promise<ReturnType<Ditherette['analyzePaletteFit']>>;
	measurePaletteFit(
		request: AnalyzePaletteFitRequest
	): Promise<ReturnType<Ditherette['measurePaletteFit']>>;
	/** All resize filters, with global anchors, bounded halo tiles and independent output. */
	resize(request: ResizeRequest, signal?: AbortSignal): Promise<Rgba8Image>;
	/** Match directly on GPU without a dither field. */
	quantize(request: QuantizeRequest, signal?: AbortSignal): Promise<IndexedImage>;
	/** Apply fields without quantisation, preserving original alpha and hidden RGB. */
	perturb(request: PerturbRequest, signal?: AbortSignal): Promise<Rgba8Image>;
	ditherAndQuantize(request: DitherAndQuantizeRequest, signal?: AbortSignal): Promise<IndexedImage>;
	/** Apply resolved effects, coverage-aware resize, then indexed processing on the chosen backends. */
	process(request: ProcessRequest, signal?: AbortSignal): Promise<IndexedImage>;
	dispose(): void;
}

/** Numeric-only bridge, shared with failure-path tests. */
export interface GpuDitherBindings extends Omit<GpuEffectsBindings, 'privateGpuEffectsDetect'> {
	privateGpuResizePlan(
		width: number,
		height: number,
		outputWidth: number,
		outputHeight: number,
		algorithm: number,
		anchor: number,
		support: number,
		rows: number
	): number;
	privateGpuResize(
		input: Uint8Array,
		width: number,
		height: number,
		outputWidth: number,
		outputHeight: number,
		algorithm: number,
		anchor: number,
		support: number,
		rows: number,
		coverage: boolean,
		memoryLimit: number,
		sink: { value?: Rgba8Image; check(completed?: number): boolean }
	): Promise<string>;
	privateGpuDitherPlan(width: number, height: number): number;
	privateGpuPerturbPlan(width: number, height: number): number;
	privateGpuYliluomaPlan(width: number, height: number, entries: number, size: number): number;
	privateGpuDiffusionPlan(width: number, height: number): number;
	privateGpuDitherDetect(memoryLimit: number): Promise<string>;
	privateGpuEffectsAdapter(): string;
	privateGpuEffectsDispose(): void;
	privateGpuDitherAndQuantize(
		input: Uint8Array,
		width: number,
		height: number,
		palette: number[],
		matching: number,
		alpha: number,
		alphaThreshold: number,
		matte: number,
		family: number,
		field: number,
		parameter: number,
		space: number,
		strength: number,
		placement: number,
		radius: number,
		threshold: number,
		softness: number,
		rgbaOutput: boolean,
		gpuDiffusion: boolean,
		coverage: Uint8Array | undefined,
		retain: boolean,
		memoryLimit: number,
		sink: {
			value?: IndexedImage | Rgba8Image;
			coverage?: Uint8Array;
			check(completed?: number): boolean;
		},
		rows: number
	): Promise<string>;
}

type Normalized = ReturnType<typeof validateDitherAndQuantize>;
type ProcessInput = ReturnType<typeof validateProcess>;

/** Verify every byte before trusting source-only coverage, including in-place caller edits. */
function sameBytes(left: Uint8Array, right: Uint8Array): boolean {
	if (left.byteLength !== right.byteLength) return false;
	if (left.byteOffset % 4 === 0 && right.byteOffset % 4 === 0) {
		const a = new Uint32Array(left.buffer, left.byteOffset, left.byteLength / 4);
		const b = new Uint32Array(right.buffer, right.byteOffset, right.byteLength / 4);
		for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
		return true;
	}
	for (let i = 0; i < left.length; i++) if (left[i] !== right[i]) return false;
	return true;
}

function placement(input: Normalized['dither']): Placement {
	if (input.placement === 0) return { mode: 'everywhere' };
	if (input.placement === 2)
		return { mode: 'smooth-area-v1', sampleSize: input.radius, contrastThreshold: input.threshold };
	return {
		mode: 'adaptive',
		radius: input.radius,
		threshold: input.threshold,
		softness: input.softness
	};
}

function dither(input: Normalized['dither']): DitherAndQuantizeRequest['dither'] {
	if (input.family === 0) return { family: 'none' };
	if (input.family === 2)
		return {
			family: 'diffusion',
			kernel: kernels[input.field] as DiffusionKernel,
			feedback: input.parameter === 0 ? 'srgb-bytes' : 'matching',
			serpentine: Boolean(input.space),
			strength: input.strength,
			placement: placement(input)
		};
	if (input.family === 3)
		return {
			family: 'yliluoma',
			size: String(input.parameter) as '2' | '4' | '8' | '16',
			placement: placement(input)
		};
	let field: Field;
	switch (input.field) {
		case 0:
			field = { algorithm: 'bayer', size: String(input.parameter) as '2' | '4' | '8' | '16' };
			break;
		case 1:
			field = { algorithm: 'random', seed: input.parameter };
			break;
		case 2:
			field = { algorithm: 'blue-noise' };
			break;
		default:
			field = {
				algorithm: 'ordered',
				tile: tiles[input.parameter] as '3x1' | '4x1' | '4x2' | '5x3'
			};
	}
	return {
		family: 'separable',
		perturb: {
			field,
			space: spaces[input.space] as WorkingSpace,
			strength: input.strength,
			placement: placement(input)
		}
	};
}

function captured(input: Normalized, data: Uint8Array): DitherAndQuantizeRequest {
	const rgb = (code: number): readonly [number, number, number] => [
		(code >>> 16) & 255,
		(code >>> 8) & 255,
		code & 255
	];
	return {
		version: 1,
		source: { width: input.sourceWidth, height: input.sourceHeight, data },
		palette: input.palette.map(
			(code): PaletteEntry =>
				code === TRANSPARENT_CODE ? { kind: 'transparent' } : { kind: 'color', rgb: rgb(code) }
		),
		matching: matchingModes[input.matching] as Matching,
		alpha:
			input.alphaMode === 0
				? { mode: 'preserve', threshold: input.threshold }
				: input.alphaMode === 1
					? { mode: 'premultiplied' }
					: { mode: 'matte', rgb: rgb(input.matte) },
		dither: dither(input.dither),
		onProgress: input.onProgress
	};
}

function output(
	input: Pick<ProcessInput, 'anchor' | 'algorithm' | 'support' | 'outputWidth' | 'outputHeight'>
): ResizeRequest['output'] {
	const anchor = anchors[input.anchor] as ResizeAnchor;
	let resize: ResizeRequest['output']['resize'];
	switch (input.algorithm) {
		case 0:
			resize = { algorithm: 'nearest', anchor };
			break;
		case 1:
			resize = { algorithm: 'area' };
			break;
		case 2:
			resize = { algorithm: 'bilinear', anchor };
			break;
		case 3:
			resize = {
				algorithm: 'bicubic',
				anchor,
				support: input.support === 0 ? 'fixed' : 'scale-aware'
			};
			break;
		case 4:
			resize = {
				algorithm: 'lanczos2',
				anchor,
				support: input.support === 0 ? 'fixed' : 'scale-aware'
			};
			break;
		case 5:
			resize = {
				algorithm: 'lanczos3',
				anchor,
				support: input.support === 0 ? 'fixed' : 'scale-aware'
			};
			break;
		default:
			resize = { algorithm: 'trilinear', anchor };
	}
	return { width: input.outputWidth, height: input.outputHeight, resize };
}

/** Compatibility factory with CPU diffusion by default; prefer createDitherette({ engine: 'gpu' }). */
export async function createGpuDither(options?: GpuDitherOptions): Promise<GpuDither> {
	const input = object(
		options === undefined ? {} : options,
		['memoryLimitBytes', 'wasm', 'gpuWasm', 'fallback', 'tiling', 'diffusionBackend', 'threads'],
		'invalid-settings',
		''
	);
	const normalized = validateOptions({
		threads: field(input, 'threads'),
		memoryLimitBytes: field(input, 'memoryLimitBytes'),
		wasm: field(input, 'wasm'),
		tiling: field(input, 'tiling')
	});
	const gpu = validateOptions({ wasm: field(input, 'gpuWasm') });
	const fallbackMode = fallbackPolicy(field(input, 'fallback'));
	const diffusionBackend = field(input, 'diffusionBackend') ?? 'cpu';
	if (diffusionBackend !== 'cpu' && diffusionBackend !== 'gpu')
		throw new DitheretteError('invalid-settings', 'diffusionBackend', 'Expected cpu or gpu.');
	let bindings: GpuDitherBindings | undefined;
	let fallback = 'NoAdapter';
	if (typeof navigator !== 'undefined' && 'gpu' in navigator) {
		try {
			const url = new URL('./wasm/gpu/ditherette_wasm.factory.js', import.meta.url);
			const factory = (await import(/* @vite-ignore */ url.href)) as {
				createScalarBindings(): GpuDitherBindings & {
					default(input: { module_or_path?: InitInput }): Promise<unknown>;
				};
			};
			const artifact = factory.createScalarBindings();
			await artifact.default({ module_or_path: normalizeWasmInput(gpu.wasm) });
			bindings = artifact;
		} catch {
			fallback = 'Initialization';
		}
	}
	return initializeGpuDither(
		bindings,
		{ ...normalized, fallback: fallbackMode, diffusionBackend },
		fallback
	);
}

/** Owns one GPU attempt and a persistent scalar fallback; source capture precedes every await. */
export function initializeGpuDither(
	bindings: GpuDitherBindings | undefined,
	options: {
		threads?: CpuInitOptions['threads'];
		wasm?: InitInput;
		memoryLimitBytes: number;
		fallback?: 'cpu' | 'error';
		tiling?: TilingPolicy;
		diffusionBackend?: 'cpu' | 'gpu';
	},
	initialFallback = 'NoAdapter'
): GpuDither {
	let active = false;
	let disposed = false;
	let detected = false;
	let unavailable = initialFallback;
	let cpu: Ditherette | undefined;
	let cpuAvailableBytes = 0;
	// One source generation, within the existing 256 MiB / quarter-budget bound.
	// Publication follows durable output and successful completion callbacks.
	let analysis:
		| {
				source: Uint8Array;
				width: number;
				height: number;
				sampleSize: number;
				tolerance: number;
				coverage: Uint8Array;
		  }
		| undefined;
	let execution: DitherExecution | undefined;
	const requireIdle = () => {
		if (disposed) throw new DitheretteError('disposed', 'instance', 'Processor is disposed.');
		if (active)
			throw new DitheretteError(
				'reentrant-call',
				'instance',
				'Processing and disposal cannot reenter an active call.'
			);
	};
	const retire = (reason: string) => {
		try {
			bindings?.privateGpuEffectsDispose();
		} finally {
			bindings = undefined;
			analysis = undefined;
			detected = false;
			unavailable = reason;
		}
	};
	function run(
		input: Normalized,
		process: ProcessInput | undefined,
		signal: AbortSignal | undefined,
		rgba: true
	): Promise<Rgba8Image>;
	function run(
		input: Normalized,
		process: ProcessInput | undefined,
		signal: AbortSignal | undefined,
		rgba?: false
	): Promise<IndexedImage>;
	async function run(
		input: Normalized,
		process: ProcessInput | undefined,
		signal: AbortSignal | undefined,
		rgba = false
	): Promise<IndexedImage | Rgba8Image> {
		requireIdle();
		active = true;
		const measured = executionTimings();
		measured.mark('request preparation', true);
		try {
			signal?.throwIfAborted();
			let reason = unavailable;
			const d = input.dither;
			const effectSpace = spaces.findIndex((space) =>
				matchingModes[input.matching].startsWith(`${space}-`)
			);
			const bytes = input.data.byteLength;
			const decoder = process?.effects?.enabled
				? gpuEffectsDecoderBytes(process.effects.json.length)
				: 0;
			if (bytes + decoder >= options.memoryLimitBytes)
				throw new DitheretteError(
					'memory-limit',
					'memoryLimitBytes',
					'Source and decoder admission exceed the instance memory limit.'
				);
			const width = process?.outputWidth ?? input.sourceWidth;
			const height = process?.outputHeight ?? input.sourceHeight;
			const resizing = Boolean(
				process && (width !== input.sourceWidth || height !== input.sourceHeight)
			);
			const preparing = Boolean(process?.effects?.enabled || resizing);
			const rows =
				typeof options.tiling === 'object'
					? options.tiling.height
					: options.tiling === 'disabled'
						? 0xffffffff
						: 0;
			// Raw input, fresh intermediate results and the previous admitted coverage
			// generation may coexist during preparation. Charge them outside each stage's
			// own complete plan; no second facade/device owns a hidden copy.
			const preparationBytes = preparing
				? bytes +
					(process?.effects?.enabled ? bytes : 0) +
					4 * width * height +
					decoder +
					(analysis ? analysis.source.byteLength + analysis.coverage.byteLength : 0)
				: 0;
			const gpuLimit = options.memoryLimitBytes - preparationBytes;
			const unsupported =
				(d.family === 2 && options.diffusionBackend !== 'gpu' && options.fallback === 'error') ||
				d.placement === 1 ||
				(d.placement === 2 && d.radius > 100) ||
				(d.family === 1 && d.strength > 4);
			if (bindings) {
				reason = unsupported ? 'UnsupportedStage' : '';
				if (!reason) {
					try {
						if (
							gpuLimit <= 0 ||
							(rgba
								? bindings.privateGpuPerturbPlan(width, height)
								: d.family === 3
									? bindings.privateGpuYliluomaPlan(
											width,
											height,
											Math.min(256, input.palette.length),
											d.parameter
										)
									: d.family === 2 && options.diffusionBackend === 'gpu'
										? bindings.privateGpuDiffusionPlan(width, height)
										: bindings.privateGpuDitherPlan(width, height)) > gpuLimit
						)
							reason = 'MemoryLimit';
						if (!reason && process?.effects?.enabled) {
							const plan = bindings.privateGpuEffectsPlan(
								process.effects.json,
								input.sourceWidth,
								input.sourceHeight,
								effectSpace,
								rows
							);
							reason = plan === 0 ? 'UnsupportedChain' : plan > gpuLimit ? 'MemoryLimit' : '';
						}
						if (!reason && resizing && process) {
							if (
								bindings.privateGpuResizePlan(
									input.sourceWidth,
									input.sourceHeight,
									width,
									height,
									process.algorithm,
									process.anchor,
									process.support,
									rows
								) > gpuLimit
							)
								reason = 'MemoryLimit';
						}
					} catch {
						reason = 'Failed';
					}
				}
				if (reason) retire(reason);
			}
			if (reason && options.fallback === 'error') throw gpuFallbackError(reason);
			const available = options.memoryLimitBytes - bytes - decoder;
			if (cpu && cpuAvailableBytes !== available) {
				cpu.dispose();
				cpu = undefined;
			}
			let snapshot: Uint8Array;
			try {
				if (
					!preparing &&
					analysis &&
					(analysis.width !== input.sourceWidth ||
						analysis.height !== input.sourceHeight ||
						!sameBytes(input.data, analysis.source))
				)
					analysis = undefined;
				if (analysis && !preparing) snapshot = analysis.source;
				else {
					snapshot = new Uint8Array(bytes);
					Uint8Array.prototype.set.call(snapshot, input.data);
				}
			} catch {
				throw new DitheretteError(
					'wasm-memory-unavailable',
					'source.data',
					'Source snapshot could not be copied.'
				);
			}
			const request = captured(input, snapshot);
			const recipe = process
				? {
						output: output(process),
						alpha: request.alpha,
						match: request.matching,
						dither: request.dither
					}
				: undefined;
			const processRequest: ProcessRequest | undefined =
				process && recipe
					? {
							source: request.source,
							palette: request.palette,
							recipe:
								process.version === 2
									? {
											...recipe,
											version: 2,
											effects: process.effects!.enabled
												? (JSON.parse(process.effects!.json) as Effect[])
												: []
										}
									: { ...recipe, version: 1 },
							onProgress: input.onProgress
						}
					: undefined;
			let adapter: string | undefined;
			let prepared: Rgba8Image = request.source;
			let effected: Rgba8Image | undefined;
			let resized: Rgba8Image | undefined;
			let value: IndexedImage | Rgba8Image | undefined;
			let coverage: Uint8Array | undefined;
			let retained =
				!preparing &&
				d.placement === 2 &&
				analysis?.sampleSize === d.radius &&
				analysis.tolerance === Math.fround(d.threshold)
					? analysis.coverage
					: undefined;
			const retain =
				d.placement === 2 &&
				4 * width * height + 4 * Math.ceil((width * height) / 32) + 1024 <=
					Math.min(256 * 1024 ** 2, Math.floor(options.memoryLimitBytes / 4));
			let callbackFailed = false;
			const report = (progress: Progress) => {
				try {
					input.onProgress?.(progress);
				} catch {
					callbackFailed = true;
					throw new DitheretteError('callback', 'onProgress', 'Progress callback threw.');
				}
			};
			let reported = 0,
				lastReport = 0;
			const total = width * height;
			const stage = rgba ? 'perturb' : 'dither-and-quantize';
			if (bindings) {
				try {
					measured.mark('request preparation', false);
					if (!detected) {
						measured.mark('GPU device initialization', true);
						reason = await bindings.privateGpuDitherDetect(gpuLimit);
						measured.mark('GPU device initialization', false);
						detected = !reason;
					}
					signal?.throwIfAborted();
					if (!reason) {
						adapter = bindings.privateGpuEffectsAdapter();
						report({ stage: 'prepare', completed: 0, total: 1 });
						if (process?.effects?.enabled) {
							measured.mark('GPU effects', true);
							const sink: { value?: Rgba8Image } = {};
							reason = await bindings.privateGpuApplyEffects(
								snapshot,
								input.sourceWidth,
								input.sourceHeight,
								process.effects.json,
								gpuLimit,
								sink,
								effectSpace,
								rows
							);
							effected = sink.value;
							measured.mark('GPU effects', false);
							sink.value = undefined;
							if (!reason && !effected) reason = 'Failed';
							if (effected) prepared = effected;
							if (!reason)
								report({
									stage: 'effects',
									completed: input.sourceWidth * input.sourceHeight,
									total: input.sourceWidth * input.sourceHeight
								});
							signal?.throwIfAborted();
						}
						if (!reason && resizing && process) {
							measured.mark('GPU resize', true);
							let progress = 0,
								lastResize = 0;
							const sink = {
								value: undefined as Rgba8Image | undefined,
								check(completed = 0) {
									if (signal?.aborted) return false;
									const now = input.onProgress && completed > progress ? performance.now() : 0;
									if (
										completed > progress &&
										(!progress || completed === total || now - lastResize >= 50)
									) {
										report({ stage: 'resize', completed, total });
										progress = completed;
										lastResize = now;
									}
									return true;
								}
							};
							reason = await bindings.privateGpuResize(
								prepared.data,
								prepared.width,
								prepared.height,
								width,
								height,
								process.algorithm,
								process.anchor,
								process.support,
								rows,
								process.version === 2,
								gpuLimit,
								sink
							);
							resized = sink.value;
							measured.mark('GPU resize', false);
							sink.value = undefined;
							if (!reason && !resized) reason = 'Failed';
							if (resized) prepared = resized;
							signal?.throwIfAborted();
						}
						if (preparing) {
							if (
								analysis &&
								(analysis.width !== width ||
									analysis.height !== height ||
									!sameBytes(prepared.data, analysis.source))
							)
								analysis = undefined;
							retained =
								d.placement === 2 &&
								analysis?.sampleSize === d.radius &&
								analysis.tolerance === Math.fround(d.threshold)
									? analysis.coverage
									: undefined;
						}
						const sink = {
							timing: measured.mark,
							value: undefined as IndexedImage | Rgba8Image | undefined,
							coverage: undefined as Uint8Array | undefined,
							check: (completed = 0) => {
								if (signal?.aborted) return false;
								if (completed > 0 && input.onProgress) {
									const now = performance.now();
									if (reported === 0 || completed === total || now - lastReport >= 50) {
										report({ stage, completed, total });
										reported = completed;
										lastReport = now;
									}
								}
								return true;
							}
						};
						if (!reason)
							reason = await bindings.privateGpuDitherAndQuantize(
								prepared.data,
								prepared.width,
								prepared.height,
								input.palette,
								input.matching,
								input.alphaMode,
								input.threshold,
								input.matte,
								d.family,
								d.field,
								d.parameter,
								d.space,
								d.strength,
								d.placement,
								d.radius,
								d.threshold,
								d.softness,
								rgba,
								options.diffusionBackend === 'gpu',
								retained,
								retain,
								gpuLimit,
								sink,
								rows
							);
						value = sink.value;
						coverage = sink.coverage;
					}
				} catch {
					reason = 'Failed';
				}
			}
			signal?.throwIfAborted();
			if (callbackFailed || reason === 'Callback')
				throw new DitheretteError('callback', 'onProgress', 'Progress callback threw.');
			if (!reason && value) {
				try {
					if (reported !== total)
						input.onProgress?.({
							stage,
							completed: total,
							total
						});
					input.onProgress?.({ stage: 'complete', completed: 1, total: 1 });
				} catch {
					throw new DitheretteError('callback', 'onProgress', 'Progress callback threw.');
				}
				signal?.throwIfAborted();
				execution = {
					backend:
						d.family === 2 && options.diffusionBackend !== 'gpu'
							? d.placement === 2 || preparing
								? 'hybrid'
								: 'cpu'
							: 'gpu',
					adapter,
					timings: measured.finish(),
					...(d.placement === 2 ? { analysisReused: Boolean(retained) } : {})
				};
				if (coverage && retain)
					analysis = {
						source: prepared.data,
						width,
						height,
						sampleSize: d.radius,
						tolerance: Math.fround(d.threshold),
						coverage
					};
				return value;
			}
			retire(reason || 'Failed');
			if (options.fallback === 'error') throw gpuFallbackError(reason || 'Failed');
			measured.mark('request preparation', false);
			measured.finish();
			measured.mark('CPU fallback', true);
			retained = undefined;
			coverage = undefined;
			prepared = request.source;
			effected = undefined;
			resized = undefined;
			cpu ??= await createCpu({
				threads: options.threads,
				wasm: options.wasm,
				memoryLimitBytes: available,
				tiling: options.tiling
			});
			cpuAvailableBytes = available;
			signal?.throwIfAborted();
			const result = processRequest
				? cpu.process(processRequest)
				: rgba && request.dither.family === 'separable'
					? cpu.perturb({
							version: 1,
							source: request.source,
							perturb: request.dither.perturb,
							onProgress: input.onProgress
						})
					: cpu.ditherAndQuantize(request);
			signal?.throwIfAborted();
			measured.mark('CPU fallback', false);
			execution = {
				backend: 'cpu',
				fallback: reason || 'Failed',
				timings: measured.finish(),
				...(adapter ? { adapter } : {})
			};
			return result;
		} finally {
			active = false;
		}
	}
	// Caller getters execute during validation. Hold the same guard there; the
	// runner takes it over synchronously before the first asynchronous boundary.
	const normalize = <T>(read: () => T): T => {
		requireIdle();
		active = true;
		try {
			return read();
		} finally {
			active = false;
		}
	};
	// Borrow the existing artifact for effects. The forwarding closures never
	// retain a Wasm instance after retirement, and closing the borrowed facade
	// leaves a healthy owner/device available for the next indexed operation.
	const runEffects = async (request: ApplyEffectsRequest | EffectMaskRequest, mask = false) => {
		requireIdle();
		active = true;
		analysis = undefined;
		cpu?.dispose();
		cpu = undefined;
		let closing = false;
		const current = () => {
			if (!bindings) throw gpuFallbackError(unavailable);
			return bindings;
		};
		const bridge: GpuEffectsBindings | undefined = bindings
			? {
					privateGpuEffectsPlanningBytes: (...args) =>
						current().privateGpuEffectsPlanningBytes(...args),
					privateGpuEffectsPlan: (...args) => current().privateGpuEffectsPlan(...args),
					privateGpuApplyEffects: (...args) => current().privateGpuApplyEffects(...args),
					privateGpuEffectsAdapter: () => current().privateGpuEffectsAdapter(),
					privateGpuEffectsDetect: async (limit) => {
						if (detected) return '';
						const reason = await current().privateGpuDitherDetect(limit);
						detected = !reason;
						return reason;
					},
					privateGpuEffectsDispose: () => {
						if (!closing) retire('Failed');
					}
				}
			: undefined;
		const effects = initializeGpuEffects(bridge, options, unavailable);
		try {
			const result = mask ? await effects.effectMask(request) : await effects.applyEffects(request);
			execution = effects.execution;
			if (!bindings && execution?.fallback) unavailable = execution.fallback;
			return result;
		} finally {
			closing = true;
			effects.dispose();
			active = false;
		}
	};
	const cpuAnalysis = async <T>(
		prepare: () => {
			input: ReturnType<typeof validateApplyEffects>;
			invoke: (processor: Ditherette, request: ReturnType<typeof capturedEffects>) => T;
		}
	) => {
		requireIdle();
		active = true;
		try {
			const { input, invoke } = prepare();
			if (options.fallback === 'error') throw gpuFallbackError('UnsupportedStage');
			const bytes = input.data.byteLength;
			if (bytes + gpuEffectsDecoderBytes(input.effects.json.length) >= options.memoryLimitBytes)
				throw gpuFallbackError('MemoryLimit');
			retire('UnsupportedStage');
			let data: Uint8Array;
			try {
				data = new Uint8Array(bytes);
				Uint8Array.prototype.set.call(data, input.data);
			} catch {
				throw new DitheretteError(
					'wasm-memory-unavailable',
					'source.data',
					'Source snapshot could not be allocated or copied.'
				);
			}
			const captured = capturedEffects(input, {
				width: input.sourceWidth,
				height: input.sourceHeight,
				data
			});
			const available = options.memoryLimitBytes - bytes;
			if (cpu && cpuAvailableBytes !== available) {
				cpu.dispose();
				cpu = undefined;
			}
			cpu ??= await createCpu({
				wasm: options.wasm,
				memoryLimitBytes: available,
				threads: options.threads,
				tiling: options.tiling
			});
			cpuAvailableBytes = available;
			const result = invoke(cpu, captured);
			execution = { backend: 'cpu', fallback: 'UnsupportedStage' };
			return result;
		} finally {
			active = false;
		}
	};
	const paletteAnalysis = <T>(
		request: AnalyzePaletteFitRequest,
		invoke: (processor: Ditherette, request: AnalyzePaletteFitRequest) => T
	) =>
		cpuAnalysis(() => {
			const input = validateAnalyzePaletteFit(request);
			return {
				input: { ...input, space: input.contextSpace },
				invoke: (processor: Ditherette, captured: ReturnType<typeof capturedEffects>) => {
					const analysis = {
						...captured,
						look: ['fitted', 'natural', 'vivid'][input.look] as FitLook,
						space: spaces[input.space] as PaletteFitSpace
					};
					return invoke(processor, analysis);
				}
			};
		});
	return {
		get execution() {
			return execution;
		},
		applyEffects: (request) => runEffects(request),
		effectMask: (request) => runEffects(request, true),
		analyzeRecolour: (request) =>
			cpuAnalysis(() => {
				const input = validateAnalyzeRecolour(request);
				return {
					input,
					invoke: (processor, captured) =>
						processor.analyzeRecolour({
							...captured,
							context: { ...captured.context, space: spaces[input.space] as WorkingSpace }
						})
				};
			}),
		analyzePaletteFit: (request) =>
			paletteAnalysis(request, (processor, input) => processor.analyzePaletteFit(input)),
		measurePaletteFit: (request) =>
			paletteAnalysis(request, (processor, input) => processor.measurePaletteFit(input)),
		async perturb(request, signal) {
			requireIdle();
			const { data, onProgress, sourceWidth, sourceHeight, ...policy } = normalize(() =>
				validatePerturb(request)
			);
			return run(
				{
					data,
					onProgress,
					sourceWidth,
					sourceHeight,
					palette: [0],
					matching: 0,
					alphaMode: 0,
					threshold: 0,
					matte: 0,
					dither: { family: 1, ...policy }
				},
				undefined,
				signal,
				true
			);
		},
		async resize(request, signal) {
			requireIdle();
			active = true;
			try {
				const input = validateResize(request);
				signal?.throwIfAborted();
				let callbackFailed = false;
				let reported = 0,
					last = 0;
				const total = input.outputWidth * input.outputHeight;
				const report = (
					stage: 'prepare' | 'resize' | 'complete',
					completed: number,
					count: number
				) => {
					try {
						input.onProgress?.({ stage, completed, total: count });
					} catch {
						callbackFailed = true;
						throw new DitheretteError('callback', 'onProgress', 'Progress callback threw.');
					}
				};
				if (input.sourceWidth === input.outputWidth && input.sourceHeight === input.outputHeight) {
					report('prepare', 0, 1);
					report('complete', 1, 1);
					execution = { backend: 'cpu', fallback: 'Identity' };
					return input.source;
				}
				const rows =
					typeof options.tiling === 'object'
						? options.tiling.height
						: options.tiling === 'disabled'
							? 0xffffffff
							: 0;
				const args = [
					input.sourceWidth,
					input.sourceHeight,
					input.outputWidth,
					input.outputHeight,
					input.algorithm,
					input.anchor,
					input.support,
					rows
				] as const;
				let reason = unavailable;
				let value: Rgba8Image | undefined;
				let adapter: string | undefined;
				if (bindings) {
					try {
						reason =
							bindings.privateGpuResizePlan(...args) > options.memoryLimitBytes
								? 'MemoryLimit'
								: '';
					} catch {
						reason = 'Failed';
					}
					if (reason) retire(reason);
				}
				if (reason && options.fallback === 'error') throw gpuFallbackError(reason);
				analysis = undefined;
				const bytes = input.data.byteLength;
				if (bytes >= options.memoryLimitBytes) throw gpuFallbackError('MemoryLimit');
				const available = options.memoryLimitBytes - bytes;
				if (cpu && cpuAvailableBytes !== available) {
					cpu.dispose();
					cpu = undefined;
				}
				let snapshot: Uint8Array;
				try {
					snapshot = new Uint8Array(bytes);
					Uint8Array.prototype.set.call(snapshot, input.data);
				} catch {
					throw new DitheretteError(
						'wasm-memory-unavailable',
						'source.data',
						'Source snapshot could not be copied.'
					);
				}
				if (bindings) {
					try {
						if (!detected) {
							reason = await bindings.privateGpuDitherDetect(options.memoryLimitBytes);
							detected = !reason;
						}
						signal?.throwIfAborted();
						if (!reason) {
							adapter = bindings.privateGpuEffectsAdapter();
							report('prepare', 0, 1);
							const sink = {
								value: undefined as Rgba8Image | undefined,
								check(completed = 0) {
									if (signal?.aborted) return false;
									if (completed && input.onProgress) {
										const now = performance.now();
										if (!reported || completed === total || now - last >= 50) {
											report('resize', completed, total);
											reported = completed;
											last = now;
										}
									}
									return true;
								}
							};
							reason = await bindings.privateGpuResize(
								snapshot,
								...args,
								false,
								options.memoryLimitBytes,
								sink
							);
							value = sink.value;
						}
					} catch {
						reason = 'Failed';
					}
				}
				signal?.throwIfAborted();
				if (callbackFailed || reason === 'Callback')
					throw new DitheretteError('callback', 'onProgress', 'Progress callback threw.');
				if (!reason && value) {
					if (reported !== total) report('resize', total, total);
					report('complete', 1, 1);
					execution = { backend: 'gpu', adapter };
					return value;
				}
				retire(reason || 'Failed');
				if (options.fallback === 'error') throw gpuFallbackError(reason || 'Failed');
				cpu ??= await createCpu({
					threads: options.threads,
					wasm: options.wasm,
					memoryLimitBytes: available,
					tiling: options.tiling
				});
				cpuAvailableBytes = available;
				signal?.throwIfAborted();
				const result = cpu.resize({
					version: 1,
					source: { width: input.sourceWidth, height: input.sourceHeight, data: snapshot },
					output: output(input),
					onProgress: input.onProgress
				});
				signal?.throwIfAborted();
				execution = {
					backend: 'cpu',
					fallback: reason || 'Failed',
					...(adapter ? { adapter } : {})
				};
				return result;
			} finally {
				active = false;
			}
		},
		async ditherAndQuantize(request, signal) {
			requireIdle();
			return run(
				normalize(() => validateDitherAndQuantize(request)),
				undefined,
				signal
			);
		},
		async quantize(request, signal) {
			requireIdle();
			const input = normalize(() => validateQuantize(request));
			return run(
				{
					...input,
					dither: {
						family: 0,
						field: 0,
						parameter: 0,
						space: 0,
						strength: 0,
						placement: 0,
						radius: 0,
						threshold: 0,
						softness: 0
					}
				},
				undefined,
				signal
			);
		},
		async process(request, signal) {
			requireIdle();
			const input = normalize(() => validateProcess(request));
			return run(input, input, signal);
		},
		dispose() {
			if (disposed) return;
			requireIdle();
			retire('Disposed');
			cpu?.dispose();
			cpu = undefined;
			disposed = true;
		}
	};
}
