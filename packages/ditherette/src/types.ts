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

/** Reversible working coordinates are independent of the palette-matching metric. */
export type WorkingSpace =
	| 'srgb'
	| 'linear-rgb'
	| 'oklab'
	| 'oklch'
	| 'cielab'
	| 'cielch'
	| 'ycbcr';

/** Palette-free fields supported by this checkpoint. Seeds are unsigned 32-bit integers. */
export type Field =
	| { readonly algorithm: 'bayer'; readonly size: '2' | '4' | '8' | '16' }
	| { readonly algorithm: 'random'; readonly seed: number }
	| { readonly algorithm: 'blue-noise' };

export type Placement =
	| { readonly mode: 'everywhere' }
	| {
			readonly mode: 'adaptive';
			readonly radius: number;
			readonly threshold: number;
			readonly softness: number;
	  };

/** Finite nonnegative f32 controls. Adaptive radius is an integer from 1 through 32768. */
export interface PerturbPolicy {
	readonly field: Field;
	readonly space: WorkingSpace;
	readonly strength: number;
	readonly placement: Placement;
}

export interface PerturbRequest {
	readonly version: 1;
	readonly source: Rgba8Image;
	readonly perturb: PerturbPolicy;
	/** S33 adds progress delivery; supplied callbacks are explicitly rejected for now. */
	readonly onProgress?: (progress: Progress) => void;
}

/** Separable fields materialize the exact clipped, rounded RGBA8 boundary before matching. */
export interface DitherAndQuantizeRequest extends QuantizeRequest {
	readonly dither:
		| { readonly family: 'none' }
		| { readonly family: 'separable'; readonly perturb: PerturbPolicy };
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
	/** Perturb RGB without palette influence, preserving every source alpha byte. */
	perturb(request: PerturbRequest): Rgba8Image;
	/** Match reconstructed RGBA8 for separable fields, or quantize directly with family none. */
	ditherAndQuantize(request: DitherAndQuantizeRequest): IndexedImage;
	/** Release instance ownership once. Wasm pages may retain their high-water mark until collection. */
	dispose(): void;
}
