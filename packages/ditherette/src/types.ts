/** Browser wasm-bindgen inputs. Views retain their byte offset and length. */
export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

/** Scalar is the default. Required threads are unavailable until the threaded runtime lands. */
export interface InitOptions {
	readonly threads?: 'disabled' | 'preferred' | 'required';
	readonly memoryLimitBytes?: number;
	readonly wasm?: InitInput;
}

/** Already-cropped, packed RGBA8. Processing never changes or retains the caller's bytes. */
export interface Rgba8Image {
	readonly width: number;
	readonly height: number;
	readonly data: Uint8Array;
}

/** Sampling alignment on each axis, in the frozen version-one 3x3 order. */
export type ResizeAnchor =
	| 'top-left'
	| 'top'
	| 'top-right'
	| 'left'
	| 'center'
	| 'right'
	| 'bottom-left'
	| 'bottom'
	| 'bottom-right';

/** Measured processing work. Progress delivery is introduced in S33. */
export interface Progress {
	readonly stage:
		| 'prepare'
		| 'resize'
		| 'alpha'
		| 'color'
		| 'perturb'
		| 'quantize'
		| 'dither-and-quantize'
		| 'complete';
	readonly completed?: number;
	readonly total?: number;
}

/** Version-one resize using the landed scalar kernels. */
export interface ResizeRequest {
	readonly version: 1;
	readonly source: Rgba8Image;
	readonly output: {
		readonly width: number;
		readonly height: number;
		readonly resize:
			| { readonly algorithm: 'nearest' | 'bilinear'; readonly anchor: ResizeAnchor }
			| { readonly algorithm: 'area' };
	};
	/** Currently rejected explicitly. S33 adds progress delivery without changing this request shape. */
	readonly onProgress?: (progress: Progress) => void;
}

/** One isolated scalar processor. Calls are synchronous; hosts choose their execution context. */
export interface Ditherette {
	/** Return durable JS-owned RGBA8, independent of later calls and disposal. */
	resize(request: ResizeRequest): Rgba8Image;
	/** Release instance ownership once. Wasm pages may retain their high-water mark until collection. */
	dispose(): void;
}
