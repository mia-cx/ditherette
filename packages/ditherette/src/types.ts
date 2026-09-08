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

/** Measured work. Stage changes report immediately; same-stage events occur at most once per 50 ms. */
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
			| { readonly algorithm: 'nearest' | 'bilinear' | 'trilinear'; readonly anchor: ResizeAnchor }
			| {
					readonly algorithm: 'bicubic' | 'lanczos2' | 'lanczos3';
					readonly anchor: ResizeAnchor;
					readonly support: 'fixed' | 'scale-aware';
			  }
			| { readonly algorithm: 'area' };
	};
	/** Synchronous progress. Throwing fails the call; processing and disposal cannot reenter this instance. */
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
	/** Completion follows durable output construction and precedes successful cache publication. */
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
	/** Completion follows durable output construction and precedes successful cache publication. */
	readonly onProgress?: (progress: Progress) => void;
}

/** Separable fields materialize the exact clipped, rounded RGBA8 boundary before matching. */
export interface DitherAndQuantizeRequest extends QuantizeRequest {
	readonly dither:
		| { readonly family: 'none' }
		| { readonly family: 'separable'; readonly perturb: PerturbPolicy }
		| {
				readonly family: 'diffusion';
				readonly kernel: 'floyd-steinberg' | 'sierra' | 'sierra-lite' | 'atkinson';
				readonly feedback: 'srgb-bytes' | 'matching';
				readonly strength: number;
				readonly serpentine: boolean;
				readonly placement: Placement;
		  }
		| {
				readonly family: 'yliluoma';
				readonly size: '2' | '4' | '8' | '16';
				readonly placement: Placement;
		  };
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

/** Complete version-one composition. The serialized matching key is match, not matching. */
export interface RecipeV1 {
	readonly version: 1;
	readonly output: ResizeRequest['output'];
	readonly alpha: AlphaPolicy;
	readonly match: Matching;
	readonly dither: DitherAndQuantizeRequest['dither'];
}

/** Resize and dither in Wasm, copying only the final durable indexed result back. */
export interface ProcessRequest {
	readonly source: Rgba8Image;
	readonly palette: readonly PaletteEntry[];
	readonly recipe: RecipeV1;
	/** S33 adds delivery; supplied callbacks remain explicitly unsupported. */
	readonly onProgress?: (progress: Progress) => void;
}

/** One isolated scalar processor. Calls are synchronous; hosts choose their execution context. */
export interface Ditherette {
	/** Apply the full recipe, preserving the same RGBA8 boundaries as staged calls. */
	process(request: ProcessRequest): IndexedImage;
	/** Return durable JS-owned RGBA8, independent of later calls and disposal. */
	resize(request: ResizeRequest): Rgba8Image;
	/** Match source pixels to the supplied palette without resizing or dithering. */
	quantize(request: QuantizeRequest): IndexedImage;
	/** Perturb RGB without palette influence, preserving every source alpha byte. */
	perturb(request: PerturbRequest): Rgba8Image;
	/** Quantize directly or apply separable fields, scalar diffusion, or literal Yliluoma mixtures. */
	ditherAndQuantize(request: DitherAndQuantizeRequest): IndexedImage;
	/** Release instance ownership once. Wasm pages may retain their high-water mark until collection. */
	dispose(): void;
}
