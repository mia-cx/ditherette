import { TRANSPARENT_KEY, darkestVisible } from '$lib/palette/wplace';
import { clampByte, createPaletteMatcher } from './color';
import type { EnabledPaletteColor, ProcessingSettings, Rgb } from './types';

const BAYER_4 = makeBayer(4);
const BAYER_8 = makeBayer(8);
const BAYER_16 = makeBayer(16);

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

function makeBayer(size: number): number[] {
	let matrix = [0];
	let current = 1;
	while (current < size) {
		const next = current * 2;
		const output = new Array(next * next).fill(0);
		for (let y = 0; y < current; y++) {
			for (let x = 0; x < current; x++) {
				const value = matrix[y * current + x] * 4;
				output[y * next + x] = value;
				output[y * next + x + current] = value + 2;
				output[(y + current) * next + x] = value + 3;
				output[(y + current) * next + x + current] = value + 1;
			}
		}
		matrix = output;
		current = next;
	}
	return matrix.map((value) => (value + 0.5) / (size * size) - 0.5);
}

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

function lumaAt(source: Uint8ClampedArray, width: number, height: number, x: number, y: number) {
	const xx = Math.min(width - 1, Math.max(0, x));
	const yy = Math.min(height - 1, Math.max(0, y));
	const offset = (yy * width + xx) * 4;
	return source[offset]! * 0.299 + source[offset + 1]! * 0.587 + source[offset + 2]! * 0.114;
}

function coverageMask(
	source: Uint8ClampedArray,
	width: number,
	height: number,
	x: number,
	y: number,
	coverage: ProcessingSettings['dither']['coverage']
) {
	if (coverage === 'full') return 1;
	const dx = Math.abs(
		lumaAt(source, width, height, x + 1, y) - lumaAt(source, width, height, x - 1, y)
	);
	const dy = Math.abs(
		lumaAt(source, width, height, x, y + 1) - lumaAt(source, width, height, x, y - 1)
	);
	const gradient = Math.sqrt(dx * dx + dy * dy);
	return coverage === 'edges' ? (gradient >= 48 ? 1 : 0) : Math.min(1, gradient / 64);
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
	const bayer =
		settings.dither.algorithm === 'bayer-4'
			? BAYER_4
			: settings.dither.algorithm === 'bayer-8'
				? BAYER_8
				: BAYER_16;
	const bayerSize =
		settings.dither.algorithm === 'bayer-4' ? 4 : settings.dither.algorithm === 'bayer-8' ? 8 : 16;
	const source = image.data;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const useBayer = settings.dither.algorithm.startsWith('bayer') && strength > 0;
	const useRandom = settings.dither.algorithm === 'random' && strength > 0;
	const noiseScale = 96 * strength;
	const coverage = settings.dither.coverage;

	for (let y = 0; y < image.height; y++) {
		const rowOffset = y * image.width;
		const bayerRow = (y % bayerSize) * bayerSize;
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

			if (useBayer) {
				const mask = coverageMask(source, image.width, image.height, x, y, coverage);
				const threshold = bayer[bayerRow + (x % bayerSize)]! * noiseScale * mask;
				r = clampByte(r + threshold);
				g = clampByte(g + threshold);
				b = clampByte(b + threshold);
			} else if (useRandom) {
				const mask = coverageMask(source, image.width, image.height, x, y, coverage);
				const noise = (random() - 0.5) * noiseScale * mask;
				r = clampByte(r + noise);
				g = clampByte(g + noise);
				b = clampByte(b + noise);
			}
			indices[index] = matcher.nearestRgb(r, g, b).index;
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
	const kernel = ERROR_KERNELS[settings.dither.algorithm as keyof typeof ERROR_KERNELS];
	const width = image.width;
	const height = image.height;
	const source = image.data;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const work = new Float32Array(width * height * 3);
	const coverage = settings.dither.coverage;

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
			const mask = coverageMask(source, width, height, x, y, coverage);
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
