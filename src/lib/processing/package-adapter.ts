import type {
	IndexedImage,
	Matching,
	Placement,
	ProcessRequest,
	RecipeV1,
	Rgba8Image,
	WorkingSpace
} from 'ditherette';
import { bayerSizeForAlgorithm } from './bayer';
import type {
	ColorSpaceId,
	CropRect,
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
	oklch: 'oklch-hue-arc',
	'weighted-rgb': 'srgb-compuphase',
	'weighted-rgb-601': 'srgb-rec601',
	'weighted-rgb-709': 'srgb-rec709'
} as const satisfies Record<ColorSpaceId, Matching>;

const WORKING_SPACE = {
	srgb: 'srgb',
	'linear-rgb': 'linear-rgb',
	oklab: 'oklab',
	cielab: 'cielab',
	oklch: 'oklch',
	'weighted-rgb': 'srgb',
	'weighted-rgb-601': 'srgb',
	'weighted-rgb-709': 'srgb'
} as const satisfies Record<ColorSpaceId, WorkingSpace>;

const RESIZE = {
	nearest: { algorithm: 'nearest', anchor: 'center' },
	bilinear: { algorithm: 'bilinear', anchor: 'center' },
	lanczos2: { algorithm: 'lanczos2', anchor: 'center', support: 'fixed' },
	'lanczos2-scale-aware': { algorithm: 'lanczos2', anchor: 'center', support: 'scale-aware' },
	lanczos3: { algorithm: 'lanczos3', anchor: 'center', support: 'fixed' },
	'lanczos3-scale-aware': { algorithm: 'lanczos3', anchor: 'center', support: 'scale-aware' },
	area: { algorithm: 'area' }
} as const satisfies Record<ResizeId, RecipeV1['output']['resize']>;

// Match the current website byte scale to the public normalized field's 255 / 4.
const RGB_DITHER_NOISE_SCALE = 96;
const BYTE_FIELD_STRENGTH_RATIO = RGB_DITHER_NOISE_SCALE / 63.75;
const MAX_ADAPTIVE_RADIUS = 32768;

function croppedSource(
	source: Pick<ImageData, 'width' | 'height' | 'data'>,
	crop?: CropRect
): Rgba8Image {
	const rect = clampCrop(source.width, source.height, crop);
	if (!Object.values(rect).every(Number.isInteger)) {
		throw new Error('The package processing path does not support fractional crop rectangles.');
	}
	const { x, y, width, height } = rect;
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
	const strength = dither.strength / 100;
	if (strength === 0) return { family: 'none' };
	const placement: Placement =
		dither.placement === 'everywhere'
			? { mode: 'everywhere' }
			: {
					mode: 'adaptive',
					radius: Math.min(MAX_ADAPTIVE_RADIUS, Math.max(1, Math.round(dither.placementRadius))),
					threshold: dither.placementThreshold,
					softness: dither.placementSoftness
				};
	const vector = dither.useColorSpace && settings.colorSpace !== 'weighted-rgb';
	switch (dither.algorithm) {
		case 'none':
			return { family: 'none' };
		case 'floyd-steinberg':
		case 'sierra':
		case 'sierra-lite':
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
		case 'random': {
			const size = bayerSizeForAlgorithm(dither.algorithm);
			return {
				family: 'separable',
				perturb: {
					field: size
						? { algorithm: 'bayer', size: `${size}` }
						: { algorithm: 'random', seed: dither.seed >>> 0 },
					space: vector ? WORKING_SPACE[settings.colorSpace] : 'srgb',
					strength: strength * (vector ? 1 : BYTE_FIELD_STRENGTH_RATIO),
					placement
				}
			};
		}
	}
}

/** Translate website controls and pack its crop for one complete public process call. */
export function packageProcessRequest(
	source: Pick<ImageData, 'width' | 'height' | 'data'>,
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
			palette: palette.map((color) => {
				if (color.kind === 'transparent') return { kind: 'transparent' };
				if (!color.rgb) throw new Error(`Palette color ${color.name} needs RGB.`);
				return { kind: 'color', rgb: [color.rgb.r, color.rgb.g, color.rgb.b] };
			}),
			recipe: {
				version: 1,
				output: { width: size.width, height: size.height, resize: RESIZE[settings.output.resize] },
				alpha,
				match: MATCHING[settings.colorSpace],
				dither: packageDither(settings)
			}
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
			warning: 'Matte color is disabled; using the nearest enabled visible color for alpha matte.'
		};
	}
	return {
		matte: visible[0]!.rgb!,
		warning: 'Matte color is unavailable; using the first enabled visible color for alpha matte.'
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
