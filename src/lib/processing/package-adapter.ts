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
import {
	RGB_DITHER_NOISE_SCALE,
	resolveMatteRgb,
	supportsVectorDither,
	type QuantizeResult
} from './quantize-shared';
import { clampCrop } from './resize';
import type {
	ColorSpaceId,
	CropRect,
	EnabledPaletteColor,
	ProcessingSettings,
	ResizeId
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
const BYTE_FIELD_STRENGTH_RATIO = RGB_DITHER_NOISE_SCALE / 63.75;

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

function packageDither(settings: ProcessingSettings): RecipeV1['dither'] {
	const { dither } = settings;
	const strength = dither.strength / 100;
	if (strength === 0) return { family: 'none' };
	const placement: Placement =
		dither.placement === 'everywhere'
			? { mode: 'everywhere' }
			: {
					mode: 'adaptive',
					radius: Math.max(1, Math.round(dither.placementRadius)),
					threshold: dither.placementThreshold,
					softness: dither.placementSoftness
				};
	const vector = supportsVectorDither(settings);
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
