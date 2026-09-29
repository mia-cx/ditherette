import type { ProcessedImage } from '$lib/processing/types';
import {
	chromaticity,
	chromaticityColour,
	coordinates,
	planePoint,
	SCOPE_MODELS,
	SPECTRAL_LOCUS,
	XY_BOUNDS,
	type ScopeModel,
	type VectorPlane
} from './colour';

/** The scopes the Scopes window offers, in its menu order. */
export const SCOPES = [
	{ id: 'histogram', label: 'Histogram' },
	{ id: 'waveform', label: 'Waveform' },
	{ id: 'parade', label: 'Parade' },
	{ id: 'vectorscope', label: 'Vectorscope' },
	{ id: 'chromaticity', label: 'CIE chromaticity' }
] as const;
export type ScopeKind = (typeof SCOPES)[number]['id'];

/** Pixels picked from an image: where each sits across the width, and its colour. */
export type Samples = {
	readonly count: number;
	readonly column: Float32Array;
	readonly rgb: Uint8Array;
};
type Rgb = readonly [number, number, number];

/** A quarter of a million samples fills every scope smoothly and keeps redraws quick. */
const MAX_SAMPLES = 1 << 18;
/** Enough sample columns to resolve the waveform at any window width. */
const MAX_COLUMNS = 1024;
export const LEVELS = 256;
export const WAVE_COLUMNS = 512;
export const PLANE_SIZE = 256;
/** How bright the xy diagram's colour backdrop glows, so samples stand out over it. */
const BACKDROP_STRENGTH = 0.45;

/** Mulberry32: a small seeded generator, so the same image always samples the same pixels. */
function random(seed: number) {
	return () => {
		seed = (seed + 0x6d2b79f5) | 0;
		let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/**
 * One pixel from a random spot in each cell of an even grid. Small images keep every pixel; on
 * large ones the jitter stops a regular dither pattern from lining up with the grid.
 * `read` copies pixel `index` into `rgb` at `at`, or returns false for a transparent pixel.
 */
export function samplePixels(
	width: number,
	height: number,
	maxSamples: number,
	read: (index: number, rgb: Uint8Array, at: number) => boolean
): Samples {
	// An image within the budget keeps every pixel, however wide it is.
	const whole = width * height <= maxSamples;
	const columns = whole ? width : Math.min(width, MAX_COLUMNS);
	const rows = whole ? height : Math.min(height, Math.max(1, Math.floor(maxSamples / columns)));
	const column = new Float32Array(columns * rows);
	const rgb = new Uint8Array(columns * rows * 3);
	const next = random(1);
	let count = 0;
	for (let row = 0; row < rows; row++) {
		for (let cell = 0; cell < columns; cell++) {
			const x = Math.floor(((cell + next()) * width) / columns);
			const y = Math.floor(((row + next()) * height) / rows);
			if (!read(y * width + x, rgb, count * 3)) continue;
			column[count++] = (x + 0.5) / width;
		}
	}
	return { count, column, rgb };
}

export function sampleSource(
	{ data, width, height }: Pick<ImageData, 'data' | 'width' | 'height'>,
	maxSamples = MAX_SAMPLES
) {
	return samplePixels(width, height, maxSamples, (index, rgb, at) => {
		const offset = index * 4;
		if (!data[offset + 3]) return false;
		rgb[at] = data[offset]!;
		rgb[at + 1] = data[offset + 1]!;
		rgb[at + 2] = data[offset + 2]!;
		return true;
	});
}

/** Samples the processed indices through their palette, so the colours are exactly the output's. */
export function sampleOutput({ indices, palette, width, height }: ProcessedImage) {
	return samplePixels(width, height, MAX_SAMPLES, (index, rgb, at) => {
		const colour = palette[indices[index]!];
		if (!colour?.rgb || colour.kind === 'transparent') return false;
		rgb[at] = colour.rgb.r;
		rgb[at + 1] = colour.rgb.g;
		rgb[at + 2] = colour.rgb.b;
		return true;
	});
}

/** Visits each sample's normalised coordinates in `model`, with how much its hue counts. */
function eachCoordinate(
	{ count, rgb }: Samples,
	model: ScopeModel,
	visit: (sample: number, point: Float64Array, hueWeight: number) => void
) {
	const point = new Float64Array(3);
	for (let sample = 0; sample < count; sample++) {
		const at = sample * 3;
		const hueWeight = coordinates(model, rgb[at]!, rgb[at + 1]!, rgb[at + 2]!, point);
		visit(sample, point, hueWeight);
	}
}

const hueChannel = (model: ScopeModel) =>
	SCOPE_MODELS.find(({ id }) => id === model)!.channels.findIndex(({ name }) => name === 'hue');
const level = (value: number) => Math.min(LEVELS - 1, Math.max(0, Math.floor(value * LEVELS)));
type Channels = readonly [Float32Array, Float32Array, Float32Array];
const channels = (length: number): Channels => [
	new Float32Array(length),
	new Float32Array(length),
	new Float32Array(length)
];

/** Samples per level for each of the model's channels. Greys count toward hue by its confidence. */
export function histogram(samples: Samples, model: ScopeModel): Channels {
	const bins = channels(LEVELS);
	const hue = hueChannel(model);
	eachCoordinate(samples, model, (_, point, hueWeight) => {
		for (let channel = 0; channel < 3; channel++)
			bins[channel]![level(point[channel]!)]! += channel === hue ? hueWeight : 1;
	});
	return bins;
}

/**
 * Samples per level and image column for each channel, as `WAVE_COLUMNS`-wide rows with the top
 * level first, ready to paint.
 */
export function waveform(samples: Samples, model: ScopeModel): Channels {
	const grid = channels(WAVE_COLUMNS * LEVELS);
	const hue = hueChannel(model);
	eachCoordinate(samples, model, (sample, point, hueWeight) => {
		const column = Math.min(WAVE_COLUMNS - 1, Math.floor(samples.column[sample]! * WAVE_COLUMNS));
		for (let channel = 0; channel < 3; channel++) {
			const row = LEVELS - 1 - level(point[channel]!);
			grid[channel]![row * WAVE_COLUMNS + column]! += channel === hue ? hueWeight : 1;
		}
	});
	return grid;
}

/** Samples per cell of a square plot, plus the sum of their colours, for a colourised trace. */
export type Scatter = { readonly density: Float32Array; readonly sums: Float32Array };

function scatter(
	{ count, rgb }: Samples,
	place: (r: number, g: number, b: number) => readonly [number, number] | undefined
): Scatter {
	const density = new Float32Array(PLANE_SIZE * PLANE_SIZE);
	const sums = new Float32Array(PLANE_SIZE * PLANE_SIZE * 3);
	for (let sample = 0; sample < count; sample++) {
		const at = sample * 3;
		const [r, g, b] = [rgb[at]!, rgb[at + 1]!, rgb[at + 2]!];
		const point = place(r, g, b);
		if (!point) continue;
		const x = Math.floor(point[0] * PLANE_SIZE);
		const y = Math.floor((1 - point[1]) * PLANE_SIZE);
		if (x < 0 || y < 0 || x >= PLANE_SIZE || y >= PLANE_SIZE) continue;
		const cell = y * PLANE_SIZE + x;
		density[cell]!++;
		sums[cell * 3]! += r;
		sums[cell * 3 + 1]! += g;
		sums[cell * 3 + 2]! += b;
	}
	return { density, sums };
}

/** How far the vectorscope's square reaches past its unit circle, leaving room for target labels. */
export const VECTOR_EXTENT = 1.25;

/**
 * The vectorscope: each sample's position on `plane`, magnified `zoom` times, in a square
 * reaching `VECTOR_EXTENT`.
 */
export function vectorscope(samples: Samples, plane: VectorPlane, zoom = 1) {
	const scratch = new Float64Array(3);
	const scale = zoom / VECTOR_EXTENT;
	return scatter(samples, (r, g, b) => {
		const [u, v] = planePoint(plane, r, g, b, scratch);
		return [(u * scale + 1) / 2, (v * scale + 1) / 2];
	});
}

/** The CIE 1931 chromaticity scope, over `XY_BOUNDS`. */
export function chromaticityScope(samples: Samples) {
	return scatter(samples, (r, g, b) => {
		const xy = chromaticity(r, g, b);
		return xy && [xy[0] / XY_BOUNDS.x, xy[1] / XY_BOUNDS.y];
	});
}

/**
 * The xy diagram's backdrop: every chromaticity inside the spectral locus in its own colour, as a
 * faint layer to paint under the samples. Built once.
 */
export const chromaticityBackdrop = (() => {
	let backdrop: Layer | undefined;
	const inside = (x: number, y: number) => {
		let crossings = 0;
		SPECTRAL_LOCUS.forEach(([, x1, y1], index) => {
			const [, x2, y2] = SPECTRAL_LOCUS[(index + 1) % SPECTRAL_LOCUS.length]!;
			if (y1 > y !== y2 > y && x < x1 + ((y - y1) * (x2 - x1)) / (y2 - y1)) crossings++;
		});
		return crossings % 2 === 1;
	};
	return () => {
		if (backdrop) return backdrop;
		const density = new Float32Array(PLANE_SIZE * PLANE_SIZE);
		const colour = new Float32Array(PLANE_SIZE * PLANE_SIZE * 3);
		for (let row = 0; row < PLANE_SIZE; row++) {
			for (let column = 0; column < PLANE_SIZE; column++) {
				const xy = [
					((column + 0.5) / PLANE_SIZE) * XY_BOUNDS.x,
					(1 - (row + 0.5) / PLANE_SIZE) * XY_BOUNDS.y
				] as const;
				if (!inside(...xy)) continue;
				const cell = row * PLANE_SIZE + column;
				density[cell] = 1;
				colour.set(chromaticityColour(xy), cell * 3);
			}
		}
		return (backdrop = { density, colour, strength: BACKDROP_STRENGTH });
	};
})();

const NEUTRAL: Rgb = [200, 200, 208];
/** How bright the source glows under the output, so the output's colours read on top. */
const UNDER = 0.6;

/** Source samples drawn dim and neutral under the output, so the output's colours read on top. */
export const underLayer = (density: Float32Array): Layer => ({
	density,
	colour: NEUTRAL,
	strength: UNDER
});

/** Adds each channel's density into one, for a source drawn neutral under the output. */
export function combined(grids: readonly Float32Array[]) {
	const sum = new Float32Array(grids[0]!.length);
	for (const grid of grids) for (let cell = 0; cell < sum.length; cell++) sum[cell]! += grid[cell]!;
	return sum;
}

/**
 * A density layer to paint: in one colour, or in the average colour of each cell's samples, at
 * `strength` of full brightness.
 */
export type Layer = {
	readonly density: Float32Array;
	readonly colour: Rgb | Float32Array;
	readonly strength?: number;
};

/** The density at which a cell glows at about two thirds of full: twice the typical busy cell. */
function exposure(density: Float32Array) {
	let total = 0;
	let filled = 0;
	for (const value of density) {
		if (!value) continue;
		total += value;
		filled++;
	}
	return filled ? (2 * total) / filled : 1;
}

/**
 * Adds the layers into one RGBA image, like light on a scope's screen. Each layer's brightness
 * rises smoothly with density, so sparse detail stays visible next to a crowded level.
 */
export function paint(width: number, height: number, layers: readonly Layer[]) {
	const light = new Float32Array(width * height * 3);
	for (const { density, colour, strength = 1 } of layers) {
		const scale = exposure(density);
		for (let cell = 0; cell < density.length; cell++) {
			const value = density[cell]!;
			if (!value) continue;
			const glow = strength * (1 - Math.exp(-value / scale));
			const at = cell * 3;
			// A colourised layer lifts each cell's average colour to full brightness, so dark
			// samples still show their hue; density alone sets how bright it glows.
			const lift =
				colour instanceof Float32Array
					? 255 / (Math.max(colour[at]!, colour[at + 1]!, colour[at + 2]!) || 1)
					: 1;
			for (let channel = 0; channel < 3; channel++)
				light[at + channel]! += colour[colour.length === 3 ? channel : at + channel]! * lift * glow;
		}
	}
	// Opaque, on black: the scope's screen.
	const image = new ImageData(width, height);
	for (let cell = 0; cell < width * height; cell++) {
		image.data.set(light.subarray(cell * 3, cell * 3 + 3), cell * 4);
		image.data[cell * 4 + 3] = 255;
	}
	return image;
}
