import { TRANSPARENT_KEY, darkestVisible } from '$lib/palette/wplace';
import { bayerSizeForAlgorithm, normalizedBayerMatrix } from './bayer';
import { clampByte, createPaletteMatcher, vectorForRgb } from './color';
import type { EnabledPaletteColor, ProcessingSettings, Rgb } from './types';

const COLOR_SPACE_THRESHOLD_SCALE = 0.25;

type ColorVector = ReturnType<typeof vectorForRgb>;

type PaletteVector = { index: number; vector: ColorVector };

type PaletteVectorSpace = {
	colors: PaletteVector[];
	ranges: ColorVector;
};

const ERROR_KERNELS = {
	'floyd-steinberg': [
		[1, 0, 7 / 16],
		[-1, 1, 3 / 16],
		[0, 1, 5 / 16],
		[1, 1, 1 / 16]
	],
	sierra: [
		[1, 0, 5 / 32],
		[2, 0, 3 / 32],
		[-2, 1, 2 / 32],
		[-1, 1, 4 / 32],
		[0, 1, 5 / 32],
		[1, 1, 4 / 32],
		[2, 1, 2 / 32],
		[-1, 2, 2 / 32],
		[0, 2, 3 / 32],
		[1, 2, 2 / 32]
	],
	'sierra-lite': [
		[1, 0, 2 / 4],
		[-1, 1, 1 / 4],
		[0, 1, 1 / 4]
	]
} satisfies Record<string, number[][]>;

type QuantizeResult = {
	indices: Uint8Array;
	palette: EnabledPaletteColor[];
	transparentIndex: number;
	warnings: string[];
};

function mulberry32(seed: number) {
	let state = seed >>> 0;
	return () => {
		state += 0x6d2b79f5;
		let value = state;
		value = Math.imul(value ^ (value >>> 15), value | 1);
		value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
		return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
	};
}

function transparentIndex(palette: EnabledPaletteColor[]) {
	return palette.findIndex((color) => color.key === TRANSPARENT_KEY);
}

function paletteRgb(color: EnabledPaletteColor): Rgb {
	if (!color.rgb) return { r: 0, g: 0, b: 0 };
	return color.rgb;
}

function paletteVectorSpace(
	matcher: ReturnType<typeof createPaletteMatcher>,
	settings: ProcessingSettings
): PaletteVectorSpace {
	const colors: PaletteVector[] = [];
	let min: ColorVector = [Infinity, Infinity, Infinity];
	let max: ColorVector = [-Infinity, -Infinity, -Infinity];
	for (let index = 0; index < matcher.colors.length; index++) {
		const color = matcher.colors[index]!;
		if (!color.rgb || color.kind === 'transparent') continue;
		const vector = vectorForRgb(color.rgb.r, color.rgb.g, color.rgb.b, settings.colorSpace);
		colors.push({ index, vector });
		min = [Math.min(min[0], vector[0]), Math.min(min[1], vector[1]), Math.min(min[2], vector[2])];
		max = [Math.max(max[0], vector[0]), Math.max(max[1], vector[1]), Math.max(max[2], vector[2])];
	}
	return {
		colors,
		ranges: [
			Math.max(max[0] - min[0], Number.EPSILON),
			Math.max(max[1] - min[1], Number.EPSILON),
			Math.max(max[2] - min[2], Number.EPSILON)
		]
	};
}

function vectorDistance(left: ColorVector, right: ColorVector, settings: ProcessingSettings) {
	const dx = left[0] - right[0];
	const dy = left[1] - right[1];
	if (settings.colorSpace === 'oklch') {
		const hue = Math.atan2(Math.sin(left[2] - right[2]), Math.cos(left[2] - right[2]));
		const dh = Math.min(left[1], right[1]) * hue;
		return dx * dx + dy * dy + dh * dh;
	}
	const dz = left[2] - right[2];
	return dx * dx + dy * dy + dz * dz;
}

function nearestPaletteVector(
	vector: ColorVector,
	space: PaletteVectorSpace,
	settings: ProcessingSettings
) {
	let winner: PaletteVector | undefined;
	let best = Infinity;
	for (const candidate of space.colors) {
		const distance = vectorDistance(vector, candidate.vector, settings);
		if (distance < best) {
			best = distance;
			winner = candidate;
		}
	}
	return winner;
}

function nearestPaletteVectorIndex(
	vector: ColorVector,
	space: PaletteVectorSpace,
	settings: ProcessingSettings
) {
	return nearestPaletteVector(vector, space, settings)?.index ?? -1;
}

function colorSpaceThresholdIndex(
	rgb: Rgb,
	threshold: number,
	space: PaletteVectorSpace,
	settings: ProcessingSettings,
	strength: number
) {
	if (strength <= 0) return -1;
	const source = vectorForRgb(rgb.r, rgb.g, rgb.b, settings.colorSpace);
	const amount = threshold * strength * COLOR_SPACE_THRESHOLD_SCALE;
	const target: ColorVector = [
		source[0] + amount * space.ranges[0],
		source[1] + amount * space.ranges[1],
		source[2] + amount * space.ranges[2]
	];
	return nearestPaletteVectorIndex(target, space, settings);
}

function smoothstep(edge0: number, edge1: number, value: number) {
	if (edge0 === edge1) return value >= edge1 ? 1 : 0;
	const t = Math.max(0, Math.min(1, (value - edge0) / (edge1 - edge0)));
	return t * t * (3 - 2 * t);
}

function sourceVectorAt(
	source: Uint8ClampedArray,
	width: number,
	height: number,
	x: number,
	y: number,
	settings: ProcessingSettings
) {
	const xx = Math.min(width - 1, Math.max(0, x));
	const yy = Math.min(height - 1, Math.max(0, y));
	const offset = (yy * width + xx) * 4;
	return vectorForRgb(
		source[offset]!,
		source[offset + 1]!,
		source[offset + 2]!,
		settings.colorSpace
	);
}

function placementMask(
	source: Uint8ClampedArray,
	width: number,
	height: number,
	x: number,
	y: number,
	settings: ProcessingSettings,
	space: PaletteVectorSpace
) {
	const placement =
		settings.dither.placement ?? (settings.dither.coverage === 'full' ? 'everywhere' : 'adaptive');
	if (placement === 'everywhere') return 1;

	const radius = Math.max(1, Math.round(settings.dither.placementRadius ?? 3));
	const center = sourceVectorAt(source, width, height, x, y, settings);
	const offsets = [
		[-radius, 0],
		[radius, 0],
		[0, -radius],
		[0, radius],
		[-radius, -radius],
		[radius, -radius],
		[-radius, radius],
		[radius, radius]
	] as const;
	let total = 0;
	for (const [dx, dy] of offsets) {
		total += Math.sqrt(
			vectorDistance(
				center,
				sourceVectorAt(source, width, height, x + dx, y + dy, settings),
				settings
			)
		);
	}
	const diagonal = Math.max(
		Math.hypot(space.ranges[0], space.ranges[1], space.ranges[2]),
		Number.EPSILON
	);
	const variance = (total / offsets.length / diagonal) * 100;
	const threshold = settings.dither.placementThreshold ?? 12;
	const softness = settings.dither.placementSoftness ?? 8;
	return smoothstep(threshold - softness, threshold + softness, variance);
}

export function quantizeImage(
	image: ImageData,
	palette: EnabledPaletteColor[],
	settings: ProcessingSettings
): QuantizeResult {
	const warnings: string[] = [];
	const nextPalette = palette.slice(0, 256);
	if (palette.length > 256)
		warnings.push('Palette was truncated to 256 entries for indexed PNG export.');

	const tIndex = transparentIndex(nextPalette);
	const visible = nextPalette.filter((color) => color.rgb && color.kind !== 'transparent');
	if (visible.length === 0) {
		if (tIndex === -1) throw new Error('Enable at least one visible color or Transparent.');
		warnings.push('Only Transparent is enabled; every output pixel is transparent.');
		return {
			indices: new Uint8Array(image.width * image.height).fill(tIndex),
			palette: nextPalette,
			transparentIndex: tIndex,
			warnings
		};
	}

	const fallbackTransparent = tIndex === -1 ? darkestVisible(visible) : undefined;
	if (settings.output.alphaMode === 'preserve' && tIndex === -1) {
		warnings.push(
			'Transparent is disabled; alpha-thresholded pixels use the darkest enabled visible color.'
		);
	}

	const matte =
		nextPalette.find((color) => color.key === settings.output.matteKey)?.rgb ?? visible[0].rgb!;
	const matcher = createPaletteMatcher(nextPalette, settings.colorSpace);
	const pixels = image.width * image.height;
	const indices = new Uint8Array(pixels);
	const strength = settings.dither.strength / 100;
	const fallbackTransparentIndex = fallbackTransparent
		? nextPalette.indexOf(fallbackTransparent)
		: -1;

	if (settings.dither.algorithm in ERROR_KERNELS && strength > 0) {
		quantizeErrorDiffusion(
			image,
			indices,
			matcher,
			settings,
			matte,
			tIndex,
			fallbackTransparentIndex,
			strength
		);
	} else {
		quantizeDirect(
			image,
			indices,
			matcher,
			settings,
			matte,
			tIndex,
			fallbackTransparentIndex,
			strength
		);
	}

	return { indices, palette: nextPalette, transparentIndex: tIndex, warnings };
}

function quantizeDirect(
	image: ImageData,
	indices: Uint8Array,
	matcher: ReturnType<typeof createPaletteMatcher>,
	settings: ProcessingSettings,
	matte: Rgb,
	transparentIndexValue: number,
	fallbackTransparentIndex: number,
	strength: number
) {
	const random = mulberry32(settings.dither.seed);
	const bayerSize = bayerSizeForAlgorithm(settings.dither.algorithm);
	const bayer = bayerSize ? normalizedBayerMatrix(bayerSize) : undefined;
	const source = image.data;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const useBayer = Boolean(bayer && bayerSize && strength > 0);
	const useRandom = settings.dither.algorithm === 'random' && strength > 0;
	const noiseScale = 96 * strength;
	const vectorSpace = paletteVectorSpace(matcher, settings);

	for (let y = 0; y < image.height; y++) {
		const rowOffset = y * image.width;
		const bayerRow = bayerSize ? (y % bayerSize) * bayerSize : 0;
		for (let x = 0; x < image.width; x++) {
			const index = rowOffset + x;
			const offset = index * 4;
			const alpha = source[offset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			let r = source[offset]!;
			let g = source[offset + 1]!;
			let b = source[offset + 2]!;
			if (alphaMode === 'matte') {
				const opacity = alpha / 255;
				const background = 1 - opacity;
				r = clampByte(r * opacity + matte.r * background);
				g = clampByte(g * opacity + matte.g * background);
				b = clampByte(b * opacity + matte.b * background);
			} else if (alphaMode === 'premultiplied') {
				const opacity = alpha / 255;
				r = clampByte(r * opacity);
				g = clampByte(g * opacity);
				b = clampByte(b * opacity);
			}

			if (useBayer && bayer && bayerSize) {
				const mask = placementMask(source, image.width, image.height, x, y, settings, vectorSpace);
				const threshold = bayer[bayerRow + (x % bayerSize)]!;
				if (settings.dither.useColorSpace) {
					const match = colorSpaceThresholdIndex(
						{ r, g, b },
						threshold,
						vectorSpace,
						settings,
						strength * mask
					);
					indices[index] = match === -1 ? matcher.nearestRgb(r, g, b).index : match;
					continue;
				}
				r = clampByte(r + threshold * noiseScale * mask);
				g = clampByte(g + threshold * noiseScale * mask);
				b = clampByte(b + threshold * noiseScale * mask);
			} else if (useRandom) {
				const mask = placementMask(source, image.width, image.height, x, y, settings, vectorSpace);
				const noise = random() - 0.5;
				if (settings.dither.useColorSpace) {
					const match = colorSpaceThresholdIndex(
						{ r, g, b },
						noise,
						vectorSpace,
						settings,
						strength * mask
					);
					indices[index] = match === -1 ? matcher.nearestRgb(r, g, b).index : match;
					continue;
				}
				r = clampByte(r + noise * noiseScale * mask);
				g = clampByte(g + noise * noiseScale * mask);
				b = clampByte(b + noise * noiseScale * mask);
			}
			indices[index] = matcher.nearestRgb(r, g, b).index;
		}
	}
}

function quantizeVectorErrorDiffusion(
	image: ImageData,
	indices: Uint8Array,
	matcher: ReturnType<typeof createPaletteMatcher>,
	settings: ProcessingSettings,
	matte: Rgb,
	transparentIndexValue: number,
	fallbackTransparentIndex: number,
	strength: number
) {
	const kernel = ERROR_KERNELS[settings.dither.algorithm as keyof typeof ERROR_KERNELS];
	const width = image.width;
	const height = image.height;
	const source = image.data;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const work = new Float32Array(width * height * 3);
	const vectorSpace = paletteVectorSpace(matcher, settings);

	for (let index = 0; index < width * height; index++) {
		const sourceOffset = index * 4;
		const workOffset = index * 3;
		let r = source[sourceOffset]!;
		let g = source[sourceOffset + 1]!;
		let b = source[sourceOffset + 2]!;
		const alpha = source[sourceOffset + 3]!;
		if (alphaMode === 'matte') {
			const opacity = alpha / 255;
			const background = 1 - opacity;
			r = r * opacity + matte.r * background;
			g = g * opacity + matte.g * background;
			b = b * opacity + matte.b * background;
		} else if (alphaMode === 'premultiplied') {
			const opacity = alpha / 255;
			r *= opacity;
			g *= opacity;
			b *= opacity;
		}
		const vector = vectorForRgb(r, g, b, settings.colorSpace);
		work[workOffset] = vector[0];
		work[workOffset + 1] = vector[1];
		work[workOffset + 2] = vector[2];
	}

	for (let y = 0; y < height; y++) {
		const reverse = settings.dither.serpentine && y % 2 === 1;
		const start = reverse ? width - 1 : 0;
		const end = reverse ? -1 : width;
		const step = reverse ? -1 : 1;
		for (let x = start; x !== end; x += step) {
			const index = y * width + x;
			const sourceOffset = index * 4;
			const alpha = source[sourceOffset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			const workOffset = index * 3;
			const current: ColorVector = [
				work[workOffset]!,
				work[workOffset + 1]!,
				work[workOffset + 2]!
			];
			const match = nearestPaletteVector(current, vectorSpace, settings);
			if (!match) throw new Error('No visible palette colors are enabled');
			indices[index] = match.index;
			const mask = placementMask(source, width, height, x, y, settings, vectorSpace);
			const error0 = current[0] - match.vector[0];
			const error1 = current[1] - match.vector[1];
			const error2 = current[2] - match.vector[2];
			for (const [dxBase, dy, weight] of kernel) {
				const dx = reverse ? -dxBase : dxBase;
				const xx = x + dx;
				const yy = y + dy;
				if (xx < 0 || xx >= width || yy < 0 || yy >= height) continue;
				const target = (yy * width + xx) * 3;
				const scaledWeight = weight * strength * mask;
				work[target] += error0 * scaledWeight;
				work[target + 1] += error1 * scaledWeight;
				work[target + 2] += error2 * scaledWeight;
			}
		}
	}
}

function quantizeErrorDiffusion(
	image: ImageData,
	indices: Uint8Array,
	matcher: ReturnType<typeof createPaletteMatcher>,
	settings: ProcessingSettings,
	matte: Rgb,
	transparentIndexValue: number,
	fallbackTransparentIndex: number,
	strength: number
) {
	if (settings.dither.useColorSpace) {
		quantizeVectorErrorDiffusion(
			image,
			indices,
			matcher,
			settings,
			matte,
			transparentIndexValue,
			fallbackTransparentIndex,
			strength
		);
		return;
	}

	const kernel = ERROR_KERNELS[settings.dither.algorithm as keyof typeof ERROR_KERNELS];
	const width = image.width;
	const height = image.height;
	const source = image.data;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const work = new Float32Array(width * height * 3);
	const vectorSpace = paletteVectorSpace(matcher, settings);

	for (let index = 0; index < width * height; index++) {
		const sourceOffset = index * 4;
		const workOffset = index * 3;
		let r = source[sourceOffset]!;
		let g = source[sourceOffset + 1]!;
		let b = source[sourceOffset + 2]!;
		const alpha = source[sourceOffset + 3]!;
		if (alphaMode === 'matte') {
			const opacity = alpha / 255;
			const background = 1 - opacity;
			r = r * opacity + matte.r * background;
			g = g * opacity + matte.g * background;
			b = b * opacity + matte.b * background;
		} else if (alphaMode === 'premultiplied') {
			const opacity = alpha / 255;
			r *= opacity;
			g *= opacity;
			b *= opacity;
		}
		work[workOffset] = r;
		work[workOffset + 1] = g;
		work[workOffset + 2] = b;
	}

	for (let y = 0; y < height; y++) {
		const reverse = settings.dither.serpentine && y % 2 === 1;
		const start = reverse ? width - 1 : 0;
		const end = reverse ? -1 : width;
		const step = reverse ? -1 : 1;
		for (let x = start; x !== end; x += step) {
			const index = y * width + x;
			const sourceOffset = index * 4;
			const alpha = source[sourceOffset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			const workOffset = index * 3;
			const r = clampByte(work[workOffset]!);
			const g = clampByte(work[workOffset + 1]!);
			const b = clampByte(work[workOffset + 2]!);
			const match = matcher.nearestRgb(r, g, b);
			indices[index] = match.index;
			const chosen = paletteRgb(match.color);
			const mask = placementMask(source, width, height, x, y, settings, vectorSpace);
			const errorR = r - chosen.r;
			const errorG = g - chosen.g;
			const errorB = b - chosen.b;
			for (const [dxBase, dy, weight] of kernel) {
				const dx = reverse ? -dxBase : dxBase;
				const xx = x + dx;
				const yy = y + dy;
				if (xx < 0 || xx >= width || yy < 0 || yy >= height) continue;
				const target = (yy * width + xx) * 3;
				const scaledWeight = weight * strength * mask;
				work[target] += errorR * scaledWeight;
				work[target + 1] += errorG * scaledWeight;
				work[target + 2] += errorB * scaledWeight;
			}
		}
	}
}
