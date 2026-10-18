import { DitheretteError } from './errors.js';
import { createScalar, normalizeWasmInput } from './scalar.js';
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
import { kernels, spaces, tiles, validateDitherAndQuantize } from './validation-fields.js';
import { validateProcess } from './validation-process.js';
import {
	fallbackPolicy,
	gpuFallbackError,
	gpuEffectsDecoderBytes,
	type GpuEffectsOptions
} from './gpu-effects.js';
import type {
	DitherAndQuantizeRequest,
	Ditherette,
	DiffusionKernel,
	Effect,
	Field,
	IndexedImage,
	InitInput,
	Matching,
	PaletteEntry,
	Placement,
	ProcessRequest,
	QuantizeRequest,
	ResizeAnchor,
	ResizeRequest,
	Rgba8Image,
	TilingPolicy,
	WorkingSpace
} from './types.js';

/** Optional GPU artifact and ordinary scalar fallback, with one combined memory limit. */
export interface GpuDitherOptions extends GpuEffectsOptions {
	/** Image tiling for resize; workers applies to scalar fallback. Hardware workgroups stay internal. */
	readonly tiling?: TilingPolicy;
}

/** Hybrid means GPU Adaptive analysis with the unchanged CPU diffusion recurrence. */
export interface DitherExecution {
	readonly backend: 'gpu' | 'hybrid' | 'cpu';
	readonly adapter?: string;
	readonly fallback?: string;
	/** True when byte-verified source coverage bypassed the complete GPU selector. */
	readonly analysisReused?: boolean;
}

/** Serial asynchronous image/indexed processing. A GPU failure retires this facade's artifact. */
export interface GpuDither {
	readonly execution: DitherExecution | undefined;
	/** All resize filters, with global anchors, bounded halo tiles and independent output. */
	resize(request: ResizeRequest, signal?: AbortSignal): Promise<Rgba8Image>;
	/** Match directly on GPU without a dither field. */
	quantize(request: QuantizeRequest, signal?: AbortSignal): Promise<IndexedImage>;
	ditherAndQuantize(request: DitherAndQuantizeRequest, signal?: AbortSignal): Promise<IndexedImage>;
	/** Effects and resizing currently use scalar fallback; prepared 1:1 images use GPU dithering. */
	process(request: ProcessRequest, signal?: AbortSignal): Promise<IndexedImage>;
	dispose(): void;
}

/** Numeric-only bridge, shared with failure-path tests. */
export interface GpuDitherBindings {
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
		coverage: Uint8Array | undefined,
		retain: boolean,
		memoryLimit: number,
		sink: { value?: IndexedImage; coverage?: Uint8Array; check(completed?: number): boolean }
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

/** Load the opt-in artifact without affecting synchronous createDitherette or inert imports. */
export async function createGpuDither(options?: GpuDitherOptions): Promise<GpuDither> {
	const input = object(
		options === undefined ? {} : options,
		['memoryLimitBytes', 'wasm', 'gpuWasm', 'fallback', 'tiling'],
		'invalid-settings',
		''
	);
	const normalized = validateOptions({
		memoryLimitBytes: field(input, 'memoryLimitBytes'),
		wasm: field(input, 'wasm'),
		tiling: field(input, 'tiling')
	});
	const gpu = validateOptions({ wasm: field(input, 'gpuWasm') });
	const fallbackMode = fallbackPolicy(field(input, 'fallback'));
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
	return initializeGpuDither(bindings, { ...normalized, fallback: fallbackMode }, fallback);
}

/** Owns one GPU attempt and a persistent scalar fallback; source capture precedes every await. */
export function initializeGpuDither(
	bindings: GpuDitherBindings | undefined,
	options: {
		wasm?: InitInput;
		memoryLimitBytes: number;
		fallback?: 'cpu' | 'error';
		tiling?: TilingPolicy;
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
	const run = async (
		input: Normalized,
		process: ProcessInput | undefined,
		signal: AbortSignal | undefined
	) => {
		requireIdle();
		active = true;
		try {
			signal?.throwIfAborted();
			let reason = unavailable;
			const d = input.dither;
			const unsupported =
				typeof options.tiling === 'object' ||
				d.family === 3 ||
				(d.family === 2 && options.fallback === 'error') ||
				d.placement === 1 ||
				(d.placement === 2 && d.radius > 100) ||
				(d.family === 1 && d.strength > 4) ||
				(process &&
					(process.outputWidth !== input.sourceWidth ||
						process.outputHeight !== input.sourceHeight ||
						process.effects?.enabled));
			if (bindings) {
				reason = unsupported ? 'UnsupportedStage' : '';
				if (!reason) {
					try {
						if (
							bindings.privateGpuDitherPlan(input.sourceWidth, input.sourceHeight) >
							options.memoryLimitBytes
						)
							reason = 'MemoryLimit';
					} catch {
						reason = 'Failed';
					}
				}
				if (reason) retire(reason);
			}
			if (reason && options.fallback === 'error') throw gpuFallbackError(reason);
			const bytes = input.data.byteLength;
			// Scalar process fallback decodes the same effects representation.
			// Reserve its outer canonical copy/JS tree before either string ABI.
			const decoder = process?.effects?.enabled
				? gpuEffectsDecoderBytes(process.effects.json.length)
				: 0;
			if (bytes + decoder >= options.memoryLimitBytes)
				throw new DitheretteError(
					'memory-limit',
					'memoryLimitBytes',
					'Source snapshot exceeds the instance memory limit.'
				);
			const available = options.memoryLimitBytes - bytes - decoder;
			if (cpu && cpuAvailableBytes !== available) {
				cpu.dispose();
				cpu = undefined;
			}
			let snapshot: Uint8Array;
			try {
				if (
					analysis &&
					(analysis.width !== input.sourceWidth ||
						analysis.height !== input.sourceHeight ||
						!sameBytes(input.data, analysis.source))
				)
					analysis = undefined;
				if (analysis) snapshot = analysis.source;
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
											effects: JSON.parse(process.effects!.json) as Effect[]
										}
									: { ...recipe, version: 1 },
							onProgress: input.onProgress
						}
					: undefined;
			let adapter: string | undefined;
			let value: IndexedImage | undefined;
			let coverage: Uint8Array | undefined;
			let retained =
				d.placement === 2 &&
				analysis?.sampleSize === d.radius &&
				analysis.tolerance === Math.fround(d.threshold)
					? analysis.coverage
					: undefined;
			const retain =
				d.placement === 2 &&
				bytes + 4 * Math.ceil((input.sourceWidth * input.sourceHeight) / 32) + 1024 <=
					Math.min(256 * 1024 ** 2, Math.floor(options.memoryLimitBytes / 4));
			let callbackFailed = false;
			let reported = 0,
				lastReport = 0;
			const total = input.sourceWidth * input.sourceHeight;
			if (bindings) {
				try {
					if (!detected) {
						reason = await bindings.privateGpuDitherDetect(options.memoryLimitBytes);
						detected = !reason;
					}
					signal?.throwIfAborted();
					if (!reason) {
						adapter = bindings.privateGpuEffectsAdapter();
						try {
							input.onProgress?.({ stage: 'prepare', completed: 0, total: 1 });
						} catch {
							callbackFailed = true;
							throw new Error('Progress callback threw.');
						}
						const sink = {
							value: undefined as IndexedImage | undefined,
							coverage: undefined as Uint8Array | undefined,
							check: (completed = 0) => {
								if (signal?.aborted) return false;
								if (completed > 0 && input.onProgress) {
									const now = performance.now();
									if (reported === 0 || completed === total || now - lastReport >= 50) {
										try {
											input.onProgress({ stage: 'dither-and-quantize', completed, total });
										} catch {
											callbackFailed = true;
											throw new Error('Progress callback threw.');
										}
										reported = completed;
										lastReport = now;
									}
								}
								return true;
							}
						};
						reason = await bindings.privateGpuDitherAndQuantize(
							snapshot,
							input.sourceWidth,
							input.sourceHeight,
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
							retained,
							retain,
							options.memoryLimitBytes,
							sink
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
							stage: 'dither-and-quantize',
							completed: input.sourceWidth * input.sourceHeight,
							total: input.sourceWidth * input.sourceHeight
						});
					input.onProgress?.({ stage: 'complete', completed: 1, total: 1 });
				} catch {
					throw new DitheretteError('callback', 'onProgress', 'Progress callback threw.');
				}
				execution = {
					backend: d.family === 2 ? (d.placement === 2 ? 'hybrid' : 'cpu') : 'gpu',
					adapter,
					...(d.placement === 2 ? { analysisReused: Boolean(retained) } : {})
				};
				if (coverage && retain)
					analysis = {
						source: snapshot,
						width: input.sourceWidth,
						height: input.sourceHeight,
						sampleSize: d.radius,
						tolerance: Math.fround(d.threshold),
						coverage
					};
				return value;
			}
			retire(reason || 'Failed');
			if (options.fallback === 'error') throw gpuFallbackError(reason || 'Failed');
			retained = undefined;
			coverage = undefined;
			cpu ??= await createScalar({
				wasm: options.wasm,
				memoryLimitBytes: available,
				tiling: options.tiling
			});
			cpuAvailableBytes = available;
			signal?.throwIfAborted();
			const result = processRequest ? cpu.process(processRequest) : cpu.ditherAndQuantize(request);
			signal?.throwIfAborted();
			execution = { backend: 'cpu', fallback: reason || 'Failed', ...(adapter ? { adapter } : {}) };
			return result;
		} finally {
			active = false;
		}
	};
	return {
		get execution() {
			return execution;
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
				cpu ??= await createScalar({
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
			return run(validateDitherAndQuantize(request), undefined, signal);
		},
		async quantize(request, signal) {
			requireIdle();
			const input = validateQuantize(request);
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
			const input = validateProcess(request);
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
