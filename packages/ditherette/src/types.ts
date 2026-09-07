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
			| {
					readonly algorithm: 'bicubic' | 'lanczos2' | 'lanczos3';
					readonly anchor: ResizeAnchor;
					readonly support: 'fixed' | 'scale-aware';
			  }
			| { readonly algorithm: 'area' };
	};
	/** Currently rejected explicitly. S33 adds progress delivery without changing this request shape. */
	readonly onProgress?: (progress: Progress) => void;
}

/** Caller-selected entries retain their input positions, including duplicates. */
export type PaletteEntry =
	| { readonly kind: 'color'; readonly rgb: readonly [number, number, number] }
	| { readonly kind: 'transparent' };

export type AlphaPolicy =
	| { readonly mode: 'preserve'; readonly threshold: number }
	| { readonly mode: 'premultiplied' }
	| { readonly mode: 'matte'; readonly rgb: readonly [number, number, number] };

/** Valid tagged color/metric pairs. Cylindrical hue uses radians. */
export type Matching =
	| 'srgb-euclidean'
	| 'linear-rgb-euclidean'
	| 'oklab-euclidean'
	| 'cielab-euclidean'
	| 'ycbcr-euclidean'
	| 'srgb-compuphase'
	| 'srgb-rec601'
	| 'srgb-rec709'
	| 'oklch-euclidean'
	| 'oklch-circular-hue'
	| 'oklch-hue-arc'
	| 'cielab-ciede2000'
	| 'cielch-euclidean'
	| 'cielch-circular-hue'
	| 'cielch-hue-arc';

export interface QuantizeRequest {
	readonly version: 1;
	readonly source: Rgba8Image;
	readonly palette: readonly PaletteEntry[];
	readonly alpha: AlphaPolicy;
	readonly matching: Matching;
	/** S33 adds progress delivery; supplied callbacks are explicitly rejected for now. */
	readonly onProgress?: (progress: Progress) => void;
}

/** Durable index bytes and their exact ordered palette, independent of later calls/disposal. */
export interface IndexedImage {
	readonly width: number;
	readonly height: number;
	readonly indices: Uint8Array;
	readonly palette: { readonly rgba: Uint8Array; readonly transparentIndex: number | null };
	readonly warnings: readonly {
		readonly code: 'palette-truncated' | 'transparent-only' | 'transparent-fallback';
		readonly message: string;
	}[];
}

/** One isolated scalar processor. Calls are synchronous; hosts choose their execution context. */
export interface Ditherette {
	/** Return durable JS-owned RGBA8, independent of later calls and disposal. */
	resize(request: ResizeRequest): Rgba8Image;
	/** Match source pixels to the supplied palette without resizing or dithering. */
	quantize(request: QuantizeRequest): IndexedImage;
	/** Release instance ownership once. Wasm pages may retain their high-water mark until collection. */
	dispose(): void;
}
