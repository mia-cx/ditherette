import { DitheretteError } from './errors.js';
import { executionTimings } from './timings.js';
import { normalizeWasmInput } from './scalar.js';
import { createCpu } from './cpu.js';
import { validateApplyEffects } from './validation-effects.js';
import { field, object, TRANSPARENT_CODE, validateOptions } from './validation.js';
import { spaces } from './validation-fields.js';
import type {
	ApplyEffectsRequest,
	CpuInitOptions,
	Effect,
	EffectMaskRequest,
	MaskCurve,
	InitInput,
	PaletteEntry,
	Progress,
	Rgba8Image,
	TilingPolicy,
	WorkingSpace
} from './types.js';

/** GPU assets are opt-in; `wasm` supplies the ordinary scalar fallback artifact. */
export interface GpuEffectsOptions {
	readonly threads?: CpuInitOptions['threads'];
	readonly memoryLimitBytes?: number;
	readonly wasm?: InitInput;
	readonly gpuWasm?: InitInput;
	/** Reject unavailable GPU work instead of instantiating the CPU fallback. */
	readonly fallback?: 'cpu' | 'error';
	/** Image tiles or whole-image execution; workers applies to CPU fallback. */
	readonly tiling?: TilingPolicy;
}

/** One fallback contract for the effects and indexed GPU facades. */
export function fallbackPolicy(value: unknown): 'cpu' | 'error' {
	if (value === undefined || value === 'cpu') return 'cpu';
	if (value === 'error') return value;
	throw new DitheretteError('invalid-settings', 'fallback', 'Unknown GPU fallback policy.');
}

export function gpuFallbackError(reason: string): DitheretteError {
	const code =
		reason === 'MemoryLimit'
			? 'memory-limit'
			: reason === 'UnsupportedStage' || reason === 'UnsupportedChain'
				? 'unsupported-operation'
				: reason === 'NoAdapter'
					? 'capability'
					: reason === 'Initialization'
						? 'initialization'
						: 'runtime';
	return new DitheretteError(code, 'gpu', `Required GPU processing is unavailable: ${reason}.`);
}

/** Execution of the last successful call. Adapter text includes the backend and device type. */
export interface EffectsExecution {
	readonly backend: 'gpu' | 'cpu';
	readonly adapter?: string;
	readonly fallback?: string;
	/** Host wall times, including GPU completion and result transfer. */
	readonly timings?: readonly import('./timings.js').ExecutionTiming[];
}

/**
 * Serial async effects processing. CPU fallback retires GPU ownership for this facade;
 * create a fresh facade to retry GPU. Hosts guard their source/slider generation before publishing output.
 */
export interface GpuEffects {
	readonly execution: EffectsExecution | undefined;
	/**
	 * Applies channel tables or continuous pointwise effects on GPU, preserving alpha and output ownership.
	 * Uploads only the request's captured packed RGBA8 bytes. Image decoding belongs to the host.
	 */
	applyEffects(request: ApplyEffectsRequest): Promise<Rgba8Image>;
	/** Evaluate a following step's mask from the continuous effects carrier. */
	effectMask(request: EffectMaskRequest): Promise<Rgba8Image>;
	/** Release the device once. An active call rejects disposal and overlapping calls. */
	dispose(): void;
}

/** Private ABI of the optional artifact. It is absent from ordinary scalar/threaded builds. */
export interface GpuEffectsBindings {
	privateGpuEffectsPlanningBytes(effectsBytes: number): number;
	privateGpuEffectsPlan(
		effects: string,
		width: number,
		height: number,
		space: number,
		rows: number,
		mask?: string
	): number;
	privateGpuEffectsDetect(memoryLimit: number): Promise<string>;
	privateGpuEffectsAdapter(): string;
	privateGpuApplyEffects(
		input: Uint8Array,
		width: number,
		height: number,
		effects: string,
		memoryLimit: number,
		sink: { value?: Rgba8Image },
		space: number,
		rows: number,
		mask?: string
	): Promise<string>;
	privateGpuEffectsDispose(): void;
}

/**
 * Decoder admission for validated canonical ASCII JSON, available without a GPU artifact.
 * Locked Wasm Value/Content occupy 24/16 bytes. Reserve per input byte: 96 for the
 * Value tree, 3 * 64 for flatten/tagged/untagged Content buffers, 32 for decoded
 * curve/grid vectors, and 4 for wire/string scratch. Vec reserves include minimum
 * capacity four and old+new storage during growth; object nodes fit 512 bytes.
 * The fixed reserve covers bounded step/planning records. The Wasm ABI tests pin
 * this numeric metadata; no module, factory, or instance survives CPU retirement.
 */
export function gpuEffectsDecoderBytes(effectsBytes: number): number {
	return 512 * 1024 + (96 + 3 * 64 + 32 + 4) * effectsBytes;
}

/** Rebuild a validated recipe around owned bytes before asynchronous CPU fallback. */
export function capturedEffects(
	input: ReturnType<typeof validateApplyEffects>,
	source: Rgba8Image
) {
	return {
		version: 1 as const,
		source,
		effects: JSON.parse(input.effects.json) as Effect[],
		context: {
			palette: (input.palette ?? []).map(
				(code): PaletteEntry =>
					code === TRANSPARENT_CODE
						? { kind: 'transparent' }
						: { kind: 'color', rgb: [(code >>> 16) & 255, (code >>> 8) & 255, code & 255] }
			),
			space: input.space < 0 ? undefined : (spaces[input.space] as WorkingSpace)
		},
		onProgress: input.onProgress
	};
}

/**
 * Compatibility effects-only factory. Prefer createDitherette({ engine: 'gpu' })
 * to share one device with resizing and dithering. Missing GPU assets use CPU fallback.
 */
export async function createGpuEffects(options?: GpuEffectsOptions): Promise<GpuEffects> {
	const input = object(
		options === undefined ? {} : options,
		['memoryLimitBytes', 'wasm', 'gpuWasm', 'fallback', 'tiling', 'threads'],
		'invalid-settings',
		''
	);
	const normalized = validateOptions({
		threads: field(input, 'threads'),
		memoryLimitBytes: field(input, 'memoryLimitBytes'),
		wasm: field(input, 'wasm'),
		tiling: field(input, 'tiling')
	});
	const gpuWasm = field(input, 'gpuWasm');
	const normalizedGpu = validateOptions({ wasm: gpuWasm });
	const fallbackMode = fallbackPolicy(field(input, 'fallback'));
	let bindings: GpuEffectsBindings | undefined;
	let fallback = 'NoAdapter';
	if (typeof navigator !== 'undefined' && 'gpu' in navigator) {
		try {
			// A variable URL keeps the optional artifact out of ordinary bundles. Hosts
			// serve dist/wasm/gpu alongside this module when they opt into the backend.
			const url = new URL('./wasm/gpu/ditherette_wasm.factory.js', import.meta.url);
			const factory = (await import(/* @vite-ignore */ url.href)) as {
				createScalarBindings(): GpuEffectsBindings & {
					default(input: { module_or_path?: InitInput }): Promise<unknown>;
				};
			};
			const artifact = factory.createScalarBindings();
			await artifact.default({ module_or_path: normalizeWasmInput(normalizedGpu.wasm) });
			bindings = artifact;
		} catch {
			bindings = undefined;
			fallback = 'Initialization';
		}
	}
	return initializeGpuEffects(bindings, { ...normalized, fallback: fallbackMode }, fallback);
}

/** Share the public boundary with focused failure-path tests, without a second CPU implementation. */
export function initializeGpuEffects(
	bindings: GpuEffectsBindings | undefined,
	options: {
		threads?: CpuInitOptions['threads'];
		wasm?: InitInput;
		memoryLimitBytes: number;
		fallback?: 'cpu' | 'error';
		tiling?: TilingPolicy;
	},
	initialFallback = 'NoAdapter'
): GpuEffects {
	let active = false;
	let disposed = false;
	let detected = false;
	let execution: EffectsExecution | undefined;
	let unavailable = initialFallback;
	const rows =
		typeof options.tiling === 'object'
			? options.tiling.height
			: options.tiling === 'disabled'
				? 0xffffffff
				: 0;
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
			// Drop the artifact itself before a fresh scalar module is allocated.
			// Device destruction cannot reclaim wgpu's forgotten callbacks or the
			// retained Wasm heap while this factory remains reachable.
			bindings = undefined;
			detected = false;
			unavailable = reason;
		}
	};
	const run = async (request: ApplyEffectsRequest | EffectMaskRequest, masked = false) => {
		requireIdle();
		active = true;
		const measured = executionTimings();
		measured.mark('effects preparation', true);
		try {
			const input = validateApplyEffects(request, masked);
			const jsonBytes = input.effects.json.length + (masked ? input.mask.length : 0);
			const report = (progress: Progress) => {
				try {
					input.onProgress?.(progress);
				} catch {
					throw new DitheretteError('callback', 'onProgress', 'Progress callback threw.');
				}
			};
			if (!masked && !input.effects.enabled) {
				report({ stage: 'prepare', completed: 0, total: 1 });
				report({ stage: 'complete', completed: 1, total: 1 });
				execution = { backend: 'cpu', fallback: 'NoEnabledEffects', timings: measured.finish() };
				return input.source;
			}
			const bytes = input.data.length;
			if (bytes >= options.memoryLimitBytes)
				throw new DitheretteError(
					'memory-limit',
					'memoryLimitBytes',
					'Source snapshot exceeds the instance memory limit.'
				);
			// Both backends use the same decoder. Rejection must precede either
			// string ABI, even without GPU assets or after artifact retirement.
			if (gpuEffectsDecoderBytes(jsonBytes) + bytes > options.memoryLimitBytes) {
				if (bindings) retire('MemoryLimit');
				throw new DitheretteError(
					'memory-limit',
					'memoryLimitBytes',
					'Effects decoder admission exceeds the instance memory limit.'
				);
			}
			let reason = unavailable;
			let adapter: string | undefined;
			let output: Rgba8Image | undefined;
			if (bindings) {
				try {
					// The ABI includes backend records alongside decoder capacity.
					// Its inputs remain numeric until all planning capacity fits.
					const planning = bindings.privateGpuEffectsPlanningBytes(jsonBytes);
					reason = planning > options.memoryLimitBytes ? 'MemoryLimit' : '';
					if (!reason) {
						const required = bindings.privateGpuEffectsPlan(
							input.effects.json,
							input.sourceWidth,
							input.sourceHeight,
							input.space,
							rows,
							masked ? input.mask : undefined
						);
						reason =
							required === 0
								? 'UnsupportedChain'
								: required > options.memoryLimitBytes
									? 'MemoryLimit'
									: '';
					}
				} catch {
					reason = 'Failed';
				}
			}
			// An unsupported or over-budget request must release the previous
			// device before allocating even its CPU-bound source snapshot.
			if (reason) retire(reason);
			if (reason && options.fallback === 'error') throw gpuFallbackError(reason);
			// Capture before the first await. Later source edits cannot change this call
			// or its CPU fallback. Charge this package-owned buffer to either backend.
			let snapshot: Uint8Array;
			try {
				snapshot = new Uint8Array(bytes);
				Uint8Array.prototype.set.call(snapshot, input.data);
			} catch {
				throw new DitheretteError(
					'wasm-memory-unavailable',
					'source.data',
					'Source snapshot could not be allocated or copied.'
				);
			}
			const source = { width: input.sourceWidth, height: input.sourceHeight, data: snapshot };
			measured.mark('effects preparation', false);
			if (bindings) {
				try {
					if (!reason && !detected) {
						measured.mark('GPU device initialization', true);
						reason = await bindings.privateGpuEffectsDetect(options.memoryLimitBytes - bytes);
						measured.mark('GPU device initialization', false);
						detected = !reason;
					}
					if (!reason) {
						adapter = bindings.privateGpuEffectsAdapter();
						const sink: { value?: Rgba8Image } = {};
						measured.mark('GPU effects + readback', true);
						reason = await bindings.privateGpuApplyEffects(
							source.data,
							source.width,
							source.height,
							input.effects.json,
							options.memoryLimitBytes,
							sink,
							input.space,
							rows,
							masked ? input.mask : undefined
						);
						output = sink.value;
						measured.mark('GPU effects + readback', false);
					}
				} catch {
					reason = 'Failed';
					// A Wasm trap may strand backend state. Retire this factory permanently.
					try {
						retire(reason);
					} catch {
						/* A trapped artifact is discarded below. */
					}
				}
			}
			if (!reason && output) {
				const total = source.width * source.height;
				report({ stage: 'prepare', completed: 0, total: 1 });
				report({ stage: 'effects', completed: total, total });
				report({ stage: 'complete', completed: 1, total: 1 });
				execution = { backend: 'gpu', adapter, timings: measured.finish() };
				return output;
			}
			// No library-owned GPU artifact may coexist with CPU fallback capacity.
			retire(reason || 'Failed');
			if (options.fallback === 'error') throw gpuFallbackError(reason || 'Failed');
			measured.finish();
			measured.mark('CPU effects fallback', true);
			const cpu = await createCpu({
				threads: options.threads,
				wasm: options.wasm,
				tiling: options.tiling,
				memoryLimitBytes: options.memoryLimitBytes - bytes
			});
			try {
				const fallbackRequest = capturedEffects(input, source);
				const result = masked
					? cpu.effectMask({ ...fallbackRequest, mask: JSON.parse(input.mask) as MaskCurve[] })
					: cpu.applyEffects(fallbackRequest);
				measured.mark('CPU effects fallback', false);
				execution = {
					backend: 'cpu',
					...(adapter ? { adapter } : {}),
					fallback: reason || 'Failed',
					timings: measured.finish()
				};
				return result;
			} finally {
				cpu.dispose();
			}
		} finally {
			active = false;
		}
	};
	return {
		get execution() {
			return execution;
		},
		applyEffects: (request) => run(request),
		effectMask: (request) => run(request, true),
		dispose() {
			if (disposed) return;
			requireIdle();
			retire('Disposed');
			disposed = true;
		}
	};
}
