import { DitheretteError } from './errors.js';
import { createScalar, normalizeWasmInput } from './scalar.js';
import { validateApplyEffects } from './validation-effects.js';
import { field, object, TRANSPARENT_CODE, validateOptions } from './validation.js';
import { spaces } from './validation-fields.js';
import type {
	ApplyEffectsRequest,
	Effect,
	InitInput,
	PaletteEntry,
	Progress,
	Rgba8Image,
	WorkingSpace
} from './types.js';

/** GPU assets are opt-in; `wasm` supplies the ordinary scalar fallback artifact. */
export interface GpuEffectsOptions {
	readonly memoryLimitBytes?: number;
	readonly wasm?: InitInput;
	readonly gpuWasm?: InitInput;
}

/** Execution of the last successful call. Adapter text includes the backend and device type. */
export interface EffectsExecution {
	readonly backend: 'gpu' | 'cpu';
	readonly adapter?: string;
	readonly fallback?: string;
}

/**
 * Serial async effects processing. CPU fallback retires GPU ownership for this facade;
 * create a fresh facade to retry GPU. Hosts guard their source/slider generation before publishing output.
 */
export interface GpuEffects {
	readonly execution: EffectsExecution | undefined;
	/**
	 * Applies real effects with exact channel tables, preserving alpha and independent output ownership.
	 * Uploads only the request's captured packed RGBA8 bytes. Image decoding belongs to the host.
	 */
	applyEffects(request: ApplyEffectsRequest): Promise<Rgba8Image>;
	/** Release the device once. An active call rejects disposal and overlapping calls. */
	dispose(): void;
}

/** Private ABI of the optional artifact. It is absent from ordinary scalar/threaded builds. */
export interface GpuEffectsBindings {
	privateGpuEffectsPlanningBytes(effectsBytes: number): number;
	privateGpuEffectsPlan(effects: string, width: number, height: number): number;
	privateGpuEffectsDetect(memoryLimit: number): Promise<string>;
	privateGpuEffectsAdapter(): string;
	privateGpuApplyEffects(
		input: Uint8Array,
		width: number,
		height: number,
		effects: string,
		memoryLimit: number,
		sink: { value?: Rgba8Image }
	): Promise<string>;
	privateGpuEffectsDispose(): void;
}

/**
 * Create an optional effects processor without changing synchronous createDitherette.
 * Missing WebGPU or optional assets uses the scalar artifact. No Wasm loads on package import.
 */
export async function createGpuEffects(options?: GpuEffectsOptions): Promise<GpuEffects> {
	const input = object(
		options === undefined ? {} : options,
		['memoryLimitBytes', 'wasm', 'gpuWasm'],
		'invalid-settings',
		''
	);
	const normalized = validateOptions({
		memoryLimitBytes: field(input, 'memoryLimitBytes'),
		wasm: field(input, 'wasm')
	});
	const gpuWasm = field(input, 'gpuWasm');
	const normalizedGpu = validateOptions({ wasm: gpuWasm });
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
	return initializeGpuEffects(bindings, normalized, fallback);
}

/** Share the public boundary with focused failure-path tests, without a second CPU implementation. */
export function initializeGpuEffects(
	bindings: GpuEffectsBindings | undefined,
	options: { wasm?: InitInput; memoryLimitBytes: number },
	initialFallback = 'NoAdapter'
): GpuEffects {
	let active = false;
	let disposed = false;
	let detected = false;
	let execution: EffectsExecution | undefined;
	let unavailable = initialFallback;
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
	return {
		get execution() {
			return execution;
		},
		async applyEffects(request) {
			requireIdle();
			active = true;
			try {
				const input = validateApplyEffects(request);
				const report = (progress: Progress) => {
					try {
						input.onProgress?.(progress);
					} catch {
						throw new DitheretteError('callback', 'onProgress', 'Progress callback threw.');
					}
				};
				if (!input.effects.enabled) {
					report({ stage: 'prepare', completed: 0, total: 1 });
					report({ stage: 'complete', completed: 1, total: 1 });
					execution = { backend: 'cpu', fallback: 'NoEnabledEffects' };
					return input.source;
				}
				const bytes = input.data.length;
				if (bytes >= options.memoryLimitBytes)
					throw new DitheretteError(
						'memory-limit',
						'memoryLimitBytes',
						'Source snapshot exceeds the instance memory limit.'
					);
				let reason = unavailable;
				let adapter: string | undefined;
				let output: Rgba8Image | undefined;
				if (bindings) {
					try {
						// The validator emits ASCII-only keys, enum values and numbers.
						// Ask the numeric ABI to admit its wire/decode/table capacity
						// before wasm-bindgen allocates a copy of that JSON string.
						const planning = bindings.privateGpuEffectsPlanningBytes(input.effects.json.length);
						reason = planning > options.memoryLimitBytes ? 'MemoryLimit' : '';
						if (!reason) {
							const required = bindings.privateGpuEffectsPlan(
								input.effects.json,
								input.sourceWidth,
								input.sourceHeight
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
				if (bindings) {
					try {
						if (!reason && !detected) {
							reason = await bindings.privateGpuEffectsDetect(options.memoryLimitBytes - bytes);
							detected = !reason;
						}
						if (!reason) {
							adapter = bindings.privateGpuEffectsAdapter();
							const sink: { value?: Rgba8Image } = {};
							reason = await bindings.privateGpuApplyEffects(
								source.data,
								source.width,
								source.height,
								input.effects.json,
								options.memoryLimitBytes,
								sink
							);
							output = sink.value;
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
					execution = { backend: 'gpu', adapter };
					return output;
				}
				// No library-owned GPU artifact may coexist with CPU fallback capacity.
				retire(reason || 'Failed');
				const cpu = await createScalar({
					wasm: options.wasm,
					memoryLimitBytes: options.memoryLimitBytes - bytes
				});
				try {
					const palette = input.palette?.map(
						(code): PaletteEntry =>
							code === TRANSPARENT_CODE
								? { kind: 'transparent' }
								: { kind: 'color', rgb: [(code >>> 16) & 255, (code >>> 8) & 255, code & 255] }
					);
					const result = cpu.applyEffects({
						version: 1,
						source,
						effects: JSON.parse(input.effects.json) as Effect[],
						context: {
							palette,
							// The existing validator supplies a checked working-space tag.
							space: input.space < 0 ? undefined : (spaces[input.space] as WorkingSpace)
						},
						onProgress: input.onProgress
					});
					execution = {
						backend: 'cpu',
						...(adapter ? { adapter } : {}),
						fallback: reason || 'Failed'
					};
					return result;
				} finally {
					cpu.dispose();
				}
			} finally {
				active = false;
			}
		},
		dispose() {
			if (disposed) return;
			requireIdle();
			retire('Disposed');
			disposed = true;
		}
	};
}
