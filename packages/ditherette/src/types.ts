/** Browser wasm-bindgen inputs. Views retain their byte offset and length. */
export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

/** Scalar is the default. Preferred threads fall back after failed capability checks or pool initialization. */
export interface InitOptions {
	/** Threaded synchronous calls require a worker context permitting blocking waits. */
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
		| 'effects'
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

/** Encoded sRGB channels a per-channel effect changes. */
export type EffectChannel = 'rgb' | 'red' | 'green' | 'blue';

/** A black/white pair in encoded sRGB units, each from 0 through 1. */
export interface LevelsPoints {
	readonly black: number;
	readonly white: number;
}

/**
 * Levels: clip to the input range, bend midtones by `gamma` (0.1 through 10, above 1 brightens),
 * then scale to the output range. Output black above output white inverts the channel.
 * Neutral is input 0..1, gamma 1, output 0..1.
 */
export interface LevelsEffect {
	readonly effect: 'levels';
	readonly enabled: boolean;
	readonly channel: EffectChannel;
	readonly input: LevelsPoints;
	readonly gamma: number;
	readonly output: LevelsPoints;
}

/**
 * Curves: a smooth, overshoot-free tone curve through 2 to 16 `[x, y]` points from 0 through 1,
 * each x at least 0.001 above the previous one. Values outside the first and last x take the end y values.
 * Neutral is `[[0, 0], [1, 1]]`.
 */
export interface CurvesEffect {
	readonly effect: 'curves';
	readonly enabled: boolean;
	readonly channel: EffectChannel;
	readonly points: readonly (readonly [number, number])[];
}

/** Contrast from -1 through 1 scales around mid-grey by `4^contrast`; brightness from -1 through 1 adds. */
export interface BrightnessContrastEffect {
	readonly effect: 'brightness-contrast';
	readonly enabled: boolean;
	readonly brightness: number;
	readonly contrast: number;
}

/** Exposure in stops from -4 through 4, applied in linear light. */
export interface ExposureEffect {
	readonly effect: 'exposure';
	readonly enabled: boolean;
	readonly stops: number;
}

/** Temperature (warm is positive) and tint (magenta is positive), each from -1 through 1. */
export interface WhiteBalanceEffect {
	readonly effect: 'white-balance';
	readonly enabled: boolean;
	readonly temperature: number;
	readonly tint: number;
}

/**
 * Hue turn in degrees from -180 through 180, with saturation and lightness from -1 through 1,
 * computed in Oklab. Saturation -1 is greyscale; lightness 1 is white and -1 is black.
 */
export interface HueSaturationEffect {
	readonly effect: 'hue-saturation';
	readonly enabled: boolean;
	readonly hue: number;
	readonly saturation: number;
	readonly lightness: number;
}

/** Adjusts colours near one hue: weight 1 at `hue`, 0 at `width` degrees away. */
export interface RecolourGroup {
	/** Degrees, 0 through 360, in the working space's opponent plane. */
	readonly hue: number;
	/** Half-width in degrees, 1 through 180. */
	readonly width: number;
	/** Hue turn in degrees at full weight, -180 through 180. */
	readonly turn: number;
	/** Chroma scale at full weight, 0 through 2. */
	readonly chroma: number;
}

/**
 * An inspectable, editable recolouring treatment. `analyzeRecolour` returns one; edit any field
 * and pass it back as a `recolour` step's `recipe`. It applies only in the space it names.
 */
export interface RecolourRecipe {
	readonly space: WorkingSpace;
	/** Lightness curve, with the same rules as `curves` points. */
	readonly tone: readonly (readonly [number, number])[];
	/** Scale for both opponent axes after `shift`, 0 through 2. */
	readonly chroma: number;
	/** Offset added to the opponent axes first, each -0.5 through 0.5. */
	readonly shift: readonly [number, number];
	/** At most 12 hue-targeted adjustments. */
	readonly groups: readonly RecolourGroup[];
}

/**
 * Fits colour to what the palette can show, directly or through dithered mixtures, before
 * quantization. With `recipe: null` it analyses the image reaching it against the context
 * palette and space. `strength` blends from the input (0) to the full treatment (1).
 */
export interface RecolourEffect {
	readonly effect: 'recolour';
	readonly enabled: boolean;
	readonly strength: number;
	readonly recipe: RecolourRecipe | null;
}

/** Analyse the image a recolour step would receive: `source` after `effects`. */
export interface AnalyzeRecolourRequest {
	readonly version: 1;
	readonly source: Rgba8Image;
	/** Steps before the recolour step. Defaults to none. */
	readonly effects: readonly Effect[];
	/** The palette and working space the recipe should fit. Both are required. */
	readonly context: { readonly palette: readonly PaletteEntry[]; readonly space: WorkingSpace };
	readonly onProgress?: (progress: Progress) => void;
}

/**
 * One step of an ordered effect chain. Steps run in array order on unrounded colour;
 * repeated effects keep their own arguments. A disabled step is validated but skipped.
 */
export type Effect =
	| LevelsEffect
	| CurvesEffect
	| BrightnessContrastEffect
	| ExposureEffect
	| WhiteBalanceEffect
	| HueSaturationEffect
	| RecolourEffect;

/** Shared inputs some effects read. Ordinary effects need neither. */
export interface EffectContext {
	readonly palette?: readonly PaletteEntry[];
	/** The working space final quantization will match in. */
	readonly space?: WorkingSpace;
}

/** Apply an ordered effect chain and return full-colour RGBA8 at the source size. */
export interface ApplyEffectsRequest {
	readonly version: 1;
	readonly source: Rgba8Image;
	readonly effects: readonly Effect[];
	readonly context?: EffectContext;
	/** Completion follows durable output construction and precedes successful cache publication. */
	readonly onProgress?: (progress: Progress) => void;
}

/**
 * Recipe v1 plus the effects that run first, on the source, before resize.
 * Effects read the request palette and the working space of `match` as their context.
 */
export interface RecipeV2 {
	readonly version: 2;
	readonly effects: readonly Effect[];
	readonly output: ResizeRequest['output'];
	readonly alpha: AlphaPolicy;
	readonly match: Matching;
	readonly dither: DitherAndQuantizeRequest['dither'];
}

/** Resize and dither in Wasm, copying only the final durable indexed result back. */
export interface ProcessRequest {
	readonly source: Rgba8Image;
	readonly palette: readonly PaletteEntry[];
	readonly recipe: RecipeV1 | RecipeV2;
	/** Completion follows durable output construction and precedes successful cache publication. */
	readonly onProgress?: (progress: Progress) => void;
}

/** One isolated processor. Calls are synchronous; hosts choose their execution context. */
export interface Ditherette {
	/** Apply the full recipe, preserving the same RGBA8 boundaries as staged calls. */
	process(request: ProcessRequest): IndexedImage;
	/** Return the source itself when no step is enabled; otherwise return independent JS-owned RGBA8. */
	applyEffects(request: ApplyEffectsRequest): Rgba8Image;
	/** Derive a fresh, editable recolouring recipe. Repeated inputs reuse the processor's cached analysis. */
	analyzeRecolour(request: AnalyzeRecolourRequest): RecolourRecipe;
	/** Return the source itself at unchanged dimensions; otherwise return independent JS-owned RGBA8. */
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
