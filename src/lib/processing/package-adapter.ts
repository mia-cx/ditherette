import type {
	EffectContext,
	Field,
	IndexedImage,
	Matching,
	PaletteEntry,
	Placement,
	ProcessRequest,
	RecipeV1,
	RecipeV2,
	ResizeAnchor,
	Rgba8Image,
	WorkingSpace
} from 'ditherette';
import { bayerSizeForAlgorithm } from './bayer';
import type {
	ColorSpaceId,
	CropRect,
	DitherId,
	EnabledPaletteColor,
	ProcessingSettings,
	QuantizeResult,
	ResizeId,
	Rgb
} from './types';

const MATCHING = {
	srgb: 'srgb-euclidean',
	'linear-rgb': 'linear-rgb-euclidean',
	oklab: 'oklab-euclidean',
	cielab: 'cielab-euclidean',
	'cielab-ciede2000': 'cielab-ciede2000',
	oklch: 'oklch-hue-arc',
	'oklch-euclidean': 'oklch-euclidean',
	'oklch-circular-hue': 'oklch-circular-hue',
	cielch: 'cielch-hue-arc',
	'cielch-euclidean': 'cielch-euclidean',
	'cielch-circular-hue': 'cielch-circular-hue',
	ycbcr: 'ycbcr-euclidean',
	'weighted-rgb': 'srgb-compuphase',
	'weighted-rgb-601': 'srgb-rec601',
	'weighted-rgb-709': 'srgb-rec709'
} as const satisfies Record<ColorSpaceId, Matching>;

const WORKING_SPACE = {
	srgb: 'srgb',
	'linear-rgb': 'linear-rgb',
	oklab: 'oklab',
	cielab: 'cielab',
	'cielab-ciede2000': 'cielab',
	oklch: 'oklch',
	'oklch-euclidean': 'oklch',
	'oklch-circular-hue': 'oklch',
	cielch: 'cielch',
	'cielch-euclidean': 'cielch',
	'cielch-circular-hue': 'cielch',
	ycbcr: 'ycbcr',
	'weighted-rgb': 'srgb',
	'weighted-rgb-601': 'srgb',
	'weighted-rgb-709': 'srgb'
} as const satisfies Record<ColorSpaceId, WorkingSpace>;

type ResizePolicy = RecipeV1['output']['resize'];

/** The package policy for a website resize mode, pinned at `anchor`. Area has no anchor. */
function packageResize(resize: ResizeId, anchor: ResizeAnchor): ResizePolicy {
	switch (resize) {
		case 'nearest':
		case 'bilinear':
		case 'trilinear':
			return { algorithm: resize, anchor };
		case 'bicubic':
		case 'lanczos2':
		case 'lanczos3':
			return { algorithm: resize, anchor, support: 'fixed' };
		case 'bicubic-scale-aware':
			return { algorithm: 'bicubic', anchor, support: 'scale-aware' };
		case 'lanczos2-scale-aware':
			return { algorithm: 'lanczos2', anchor, support: 'scale-aware' };
		case 'lanczos3-scale-aware':
			return { algorithm: 'lanczos3', anchor, support: 'scale-aware' };
		case 'area':
			return { algorithm: 'area' };
	}
}

const YLILUOMA_SIZE = {
	'yliluoma-2': '2',
	'yliluoma-4': '4',
	'yliluoma-8': '8',
	'yliluoma-16': '16'
} as const;

/** Rectangular ordered tiles, named width by height. */
const ORDERED_TILE: Partial<Record<DitherId, Extract<Field, { algorithm: 'ordered' }>['tile']>> = {
	'ordered-3x1': '3x1',
	'ordered-4x1': '4x1',
	'ordered-4x2': '4x2',
	'ordered-5x3': '5x3'
};

// Match the current website byte scale to the public normalized field's 255 / 4.
const RGB_DITHER_NOISE_SCALE = 96;
const BYTE_FIELD_STRENGTH_RATIO = RGB_DITHER_NOISE_SCALE / 63.75;
const MAX_ADAPTIVE_RADIUS = 32768;

/** The part of `source` that processing reads: the whole image, or the crop, borrowing bytes when contiguous. */
export function croppedSource(
	source: { width: number; height: number; data: Uint8Array | Uint8ClampedArray },
	crop?: CropRect
): Rgba8Image {
	const rect = clampCrop(source.width, source.height, crop);
	if (!Object.values(rect).every(Number.isInteger)) {
		throw new Error('The package processing path does not support fractional crop rectangles.');
	}
	const { x, y, width, height } = rect;
	if (x === 0 && width === source.width) {
		return {
			width,
			height,
			data: new Uint8Array(
				source.data.buffer,
				source.data.byteOffset + y * width * 4,
				width * height * 4
			)
		};
	}
	const data = new Uint8Array(width * height * 4);
	for (let row = 0; row < height; row++) {
		const start = ((y + row) * source.width + x) * 4;
		data.set(source.data.subarray(start, start + width * 4), row * width * 4);
	}
	return { width, height, data };
}

function finiteRectValue(value: number, label: string) {
	if (!Number.isFinite(value)) throw new Error(`${label} must be finite.`);
	return value;
}

/** Clamp the requested crop to the source bounds without rounding fractional coordinates. */
function clampCrop(sourceWidth: number, sourceHeight: number, crop?: CropRect): CropRect {
	if (!crop) return { x: 0, y: 0, width: sourceWidth, height: sourceHeight };
	finiteRectValue(crop.x, 'Crop x');
	finiteRectValue(crop.y, 'Crop y');
	finiteRectValue(crop.width, 'Crop width');
	finiteRectValue(crop.height, 'Crop height');
	const x = Math.min(sourceWidth - 1, Math.max(0, crop.x));
	const y = Math.min(sourceHeight - 1, Math.max(0, crop.y));
	return {
		x,
		y,
		width: Math.max(1, Math.min(sourceWidth - x, crop.width)),
		height: Math.max(1, Math.min(sourceHeight - y, crop.height))
	};
}

function packageDither(settings: ProcessingSettings): RecipeV1['dither'] {
	const { dither } = settings;
	const placement: Placement =
		dither.placement === 'everywhere'
			? { mode: 'everywhere' }
			: {
					mode: 'adaptive',
					radius: Math.min(MAX_ADAPTIVE_RADIUS, Math.max(1, Math.round(dither.placementRadius))),
					threshold: dither.placementThreshold,
					softness: dither.placementSoftness
				};
	// Yliluoma places whole palette colors, so it has no strength to turn down.
	switch (dither.algorithm) {
		case 'yliluoma-2':
		case 'yliluoma-4':
		case 'yliluoma-8':
		case 'yliluoma-16':
			return { family: 'yliluoma', size: YLILUOMA_SIZE[dither.algorithm], placement };
	}
	const strength = dither.strength / 100;
	if (strength === 0) return { family: 'none' };
	const vector = dither.useColorSpace && settings.colorSpace !== 'weighted-rgb';
	switch (dither.algorithm) {
		case 'none':
			return { family: 'none' };
		case 'floyd-steinberg':
		case 'sierra':
		case 'sierra-lite':
		case 'atkinson':
		case 'jarvis-judice-ninke':
		case 'stucki':
		case 'burkes':
		case 'two-row-sierra':
		case 'fan':
		case 'shiau-fan':
		case 'shiau-fan-2':
		case 'simple-2d':
			return {
				family: 'diffusion',
				kernel: dither.algorithm,
				feedback: vector ? 'matching' : 'srgb-bytes',
				strength,
				serpentine: dither.serpentine,
				placement
			};
		case 'bayer-2':
		case 'bayer-4':
		case 'bayer-8':
		case 'bayer-16':
		case 'ordered-3x1':
		case 'ordered-4x1':
		case 'ordered-4x2':
		case 'ordered-5x3':
		case 'random':
		case 'blue-noise': {
			const size = bayerSizeForAlgorithm(dither.algorithm);
			const tile = ORDERED_TILE[dither.algorithm];
			return {
				family: 'separable',
				perturb: {
					field: size
						? { algorithm: 'bayer', size: `${size}` }
						: tile
							? { algorithm: 'ordered', tile }
							: dither.algorithm === 'blue-noise'
								? { algorithm: 'blue-noise' }
								: { algorithm: 'random', seed: dither.seed >>> 0 },
					space: vector ? WORKING_SPACE[settings.colorSpace] : 'srgb',
					strength: strength * (vector ? 1 : BYTE_FIELD_STRENGTH_RATIO),
					placement
				}
			};
		}
	}
}

function packagePalette(palette: EnabledPaletteColor[]): PaletteEntry[] {
	return palette.map((color) => {
		if (color.kind === 'transparent') return { kind: 'transparent' };
		if (!color.rgb) throw new Error(`Palette colour ${color.name} needs RGB.`);
		return { kind: 'color', rgb: [color.rgb.r, color.rgb.g, color.rgb.b] };
	});
}

/** The palette and working space a recolour step fits, matching what `process` gives it. */
export function packageEffectContext(
	palette: EnabledPaletteColor[],
	colorSpace: ColorSpaceId
): Required<EffectContext> {
	return { palette: packagePalette(palette), space: WORKING_SPACE[colorSpace] };
}

/** Always recipe v2: besides effects, it resizes colour weighted by coverage, so transparency never bleeds. */
function packageRecipe(settings: ProcessingSettings, stages: Omit<RecipeV1, 'version'>): RecipeV2 {
	return { version: 2, effects: settings.effects, ...stages };
}

/** Translate controls for a synchronous public call. Contiguous crops borrow bytes; Rust owns the snapshot. */
export function packageProcessRequest(
	source: { width: number; height: number; data: Uint8Array | Uint8ClampedArray },
	palette: EnabledPaletteColor[],
	settings: ProcessingSettings,
	size: { width: number; height: number }
): { request: ProcessRequest; warnings: string[] } {
	const normalized = palette.slice(0, 256);
	const visible = normalized.filter((color) => color.rgb && color.kind !== 'transparent');
	const matte = visible.length
		? resolveMatteRgb(normalized, visible, settings.output.matteKey)
		: undefined;
	const rgb = matte?.matte ?? { r: 0, g: 0, b: 0 };
	const alpha: RecipeV1['alpha'] =
		settings.output.alphaMode === 'preserve'
			? { mode: 'preserve', threshold: settings.output.alphaThreshold }
			: settings.output.alphaMode === 'matte'
				? { mode: 'matte', rgb: [rgb.r, rgb.g, rgb.b] }
				: { mode: 'premultiplied' };
	return {
		request: {
			source: croppedSource(source, settings.output.crop),
			palette: packagePalette(palette),
			recipe: packageRecipe(settings, {
				output: {
					width: size.width,
					height: size.height,
					resize: packageResize(settings.output.resize, settings.output.anchor ?? 'center')
				},
				alpha,
				match: MATCHING[settings.colorSpace],
				dither: packageDither(settings)
			})
		},
		warnings: matte?.warning ? [matte.warning] : []
	};
}

/** Restore website palette metadata by ordered index, including duplicate colors. */
export function packageQuantizeResult(
	result: IndexedImage,
	palette: EnabledPaletteColor[],
	warnings: string[]
): QuantizeResult {
	return {
		indices: result.indices,
		palette: palette.slice(0, result.palette.rgba.length / 4).map((color, index) => {
			const offset = index * 4;
			return {
				...color,
				tags: color.tags?.slice(),
				rgb:
					color.kind === 'transparent'
						? undefined
						: {
								r: result.palette.rgba[offset]!,
								g: result.palette.rgba[offset + 1]!,
								b: result.palette.rgba[offset + 2]!
							}
			};
		}),
		transparentIndex: result.palette.transparentIndex ?? -1,
		warnings: [...result.warnings.map((warning) => warning.message), ...warnings]
	};
}

/** Resolve the selected matte against a normalized palette containing visible colors. */
function resolveMatteRgb(
	palette: EnabledPaletteColor[],
	visible: EnabledPaletteColor[],
	matteKey: string
): { matte: Rgb; warning?: string } {
	const selected = palette.find((color) => color.key === matteKey)?.rgb;
	if (selected) return { matte: selected };
	const matteRgb = rgbFromHexKey(matteKey);
	if (matteRgb) {
		return {
			matte: nearestVisibleColor(matteRgb, visible).rgb!,
			warning: 'Matte colour is disabled; using the nearest enabled visible colour for alpha matte.'
		};
	}
	return {
		matte: visible[0]!.rgb!,
		warning: 'Matte colour is unavailable; using the first enabled visible colour for alpha matte.'
	};
}

function rgbFromHexKey(key: string): Rgb | undefined {
	if (!/^#[0-9a-fA-F]{6}$/.test(key)) return undefined;
	return {
		r: Number.parseInt(key.slice(1, 3), 16),
		g: Number.parseInt(key.slice(3, 5), 16),
		b: Number.parseInt(key.slice(5, 7), 16)
	};
}

function nearestVisibleColor(rgb: Rgb, visible: EnabledPaletteColor[]) {
	let best = visible[0]!;
	let bestDistance = Number.POSITIVE_INFINITY;
	for (const color of visible) {
		const candidate = color.rgb!;
		const distance =
			(candidate.r - rgb.r) ** 2 + (candidate.g - rgb.g) ** 2 + (candidate.b - rgb.b) ** 2;
		if (distance < bestDistance) {
			best = color;
			bestDistance = distance;
		}
	}
	return best;
}
