import { CURVE_MODELS, type ChannelName, type CurveModel } from '$lib/effects/catalog';
import type { ProcessedImage } from '$lib/processing/types';

/** Every model the histogram shows: the curves models, plus CMY as inverted RGB. */
export type HistogramModel = CurveModel | 'cmy';
export type HistogramChannel = {
	readonly name: ChannelName | 'cyan' | 'magenta' | 'yellow';
	readonly label: string;
};

const [srgb, ...others] = CURVE_MODELS;
export const HISTOGRAM_MODELS: readonly {
	readonly id: HistogramModel;
	readonly label: string;
	readonly channels: readonly [HistogramChannel, HistogramChannel, HistogramChannel];
}[] = [
	srgb!,
	{
		id: 'cmy',
		label: 'CMY',
		channels: [
			{ name: 'cyan', label: 'Cyan' },
			{ name: 'magenta', label: 'Magenta' },
			{ name: 'yellow', label: 'Yellow' }
		]
	},
	...others
];

export const BINS = 256;
/** One weight per bin for each of a model's three channels. */
export type Histogram = readonly [Float64Array, Float64Array, Float64Array];

/**
 * Sources above this many pixels are sampled at an even stride. A quarter of a million samples
 * already fills 256 bins smoothly, and keeps a model switch on a large photo quick.
 */
const MAX_SAMPLES = 1 << 18;

const decode = Float64Array.from({ length: 256 }, (_, byte) => {
	const value = byte / 255;
	return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
});

const TURN = 2 * Math.PI;
const hueOf = (a: number, b: number) =>
	a === 0 && b === 0 ? 0 : (((Math.atan2(b, a) / TURN) % 1) + 1) % 1;
const unit = (value: number) => Math.min(1, Math.max(0, value));

function oklab(r: number, g: number, b: number) {
	const [lr, lg, lb] = [decode[r]!, decode[g]!, decode[b]!];
	const l = Math.cbrt(0.41222146 * lr + 0.53633255 * lg + 0.051445995 * lb);
	const m = Math.cbrt(0.2119035 * lr + 0.6806995 * lg + 0.10739696 * lb);
	const s = Math.cbrt(0.08830246 * lr + 0.28171885 * lg + 0.6299787 * lb);
	return [
		0.21045426 * l + 0.7936178 * m - 0.004072047 * s,
		1.9779985 * l - 2.4285922 * m + 0.4505937 * s,
		0.025904037 * l + 0.78277177 * m - 0.80867577 * s
	] as const;
}

const labF = (t: number) => (t > 216 / 24389 ? Math.cbrt(t) : ((24389 / 27) * t + 16) / 116);
function cielab(r: number, g: number, b: number) {
	const [lr, lg, lb] = [decode[r]!, decode[g]!, decode[b]!];
	const fx = labF((0.4124564 * lr + 0.3575761 * lg + 0.1804375 * lb) / 0.95047);
	const fy = labF(0.2126729 * lr + 0.7151522 * lg + 0.072175 * lb);
	const fz = labF((0.0193339 * lr + 0.119192 * lg + 0.9503041 * lb) / 1.08883);
	return [116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz)] as const;
}

/** HSL or HSV from encoded RGB bytes, with the hue confidence the package uses. */
function cylinder(r: number, g: number, b: number, lightness: boolean, out: Float64Array) {
	const [red, green, blue] = [r / 255, g / 255, b / 255];
	const max = Math.max(red, green, blue);
	const min = Math.min(red, green, blue);
	const chroma = max - min;
	let hue = 0;
	if (chroma > 0) {
		if (max === red) hue = ((green - blue) / chroma + 6) % 6;
		else if (max === green) hue = (blue - red) / chroma + 2;
		else hue = (red - green) / chroma + 4;
	}
	out[0] = hue / 6;
	if (lightness) {
		const l = (max + min) / 2;
		out[1] = l === 0 || l === 1 ? 0 : chroma / (1 - Math.abs(2 * l - 1));
		out[2] = l;
	} else {
		out[1] = max === 0 ? 0 : chroma / max;
		out[2] = max;
	}
	return unit(chroma / 0.02);
}

/**
 * Write a colour's coordinates in `model`, normalised the way the package normalises curve axes,
 * and return how much its hue counts: greys have no hue, so they add nothing to a hue plot.
 */
function coordinates(model: HistogramModel, r: number, g: number, b: number, out: Float64Array) {
	switch (model) {
		case 'srgb':
			out.set([r / 255, g / 255, b / 255]);
			return 1;
		case 'cmy':
			out.set([1 - r / 255, 1 - g / 255, 1 - b / 255]);
			return 1;
		case 'linear-rgb':
			out.set([decode[r]!, decode[g]!, decode[b]!]);
			return 1;
		case 'hsl':
			return cylinder(r, g, b, true, out);
		case 'hsv':
			return cylinder(r, g, b, false, out);
		case 'ycbcr': {
			const [red, green, blue] = [r / 255, g / 255, b / 255];
			const luma = 0.299 * red + 0.587 * green + 0.114 * blue;
			out.set([luma, (blue - luma) / 1.772 + 0.5, (red - luma) / 1.402 + 0.5]);
			return 1;
		}
		case 'oklab': {
			const [l, a, bb] = oklab(r, g, b);
			out.set([l, a / 0.8 + 0.5, bb / 0.8 + 0.5]);
			return 1;
		}
		case 'oklch': {
			const [l, a, bb] = oklab(r, g, b);
			const chroma = Math.hypot(a, bb);
			out.set([l, chroma / 0.4, hueOf(a, bb)]);
			return r === g && g === b ? 0 : unit(chroma / 0.02);
		}
		case 'cielab': {
			const [l, a, bb] = cielab(r, g, b);
			out.set([l / 100, a / 250 + 0.5, bb / 250 + 0.5]);
			return 1;
		}
		case 'cielch': {
			const [l, a, bb] = cielab(r, g, b);
			const chroma = Math.hypot(a, bb);
			out.set([l / 100, chroma / 150, hueOf(a, bb)]);
			return r === g && g === b ? 0 : unit(chroma / 2);
		}
	}
}

/** Accumulates weighted colours into a model's three channel histograms. */
function binner(model: HistogramModel) {
	const bins: Histogram = [new Float64Array(BINS), new Float64Array(BINS), new Float64Array(BINS)];
	const hue = HISTOGRAM_MODELS.find(({ id }) => id === model)!.channels.findIndex(
		({ name }) => name === 'hue'
	);
	const point = new Float64Array(3);
	const add = (r: number, g: number, b: number, weight: number) => {
		const hueWeight = coordinates(model, r, g, b, point);
		for (let channel = 0; channel < 3; channel++) {
			const bin = Math.min(BINS - 1, Math.max(0, Math.floor(point[channel]! * BINS)));
			bins[channel]![bin] += channel === hue ? weight * hueWeight : weight;
		}
	};
	return { bins, add };
}

/** The source's histogram in `model`. Transparent pixels don't count. */
export function sourceHistogram(image: Pick<ImageData, 'data'>, model: HistogramModel): Histogram {
	const { bins, add } = binner(model);
	const { data } = image;
	const pixels = data.length / 4;
	const stride = Math.max(1, Math.ceil(pixels / MAX_SAMPLES));
	for (let pixel = 0; pixel < pixels; pixel += stride) {
		const offset = pixel * 4;
		if (data[offset + 3]) add(data[offset]!, data[offset + 1]!, data[offset + 2]!, 1);
	}
	return bins;
}

/**
 * The output's histogram in `model`, exact: it counts each palette index in the processed
 * indices, then bins each palette colour once with its count.
 */
export function outputHistogram(image: ProcessedImage, model: HistogramModel): Histogram {
	const counts = new Float64Array(image.palette.length);
	for (const index of image.indices) counts[index] = counts[index]! + 1;
	const { bins, add } = binner(model);
	image.palette.forEach(({ kind, rgb }, index) => {
		const count = counts[index]!;
		if (kind !== 'transparent' && rgb && count) add(rgb.r, rgb.g, rgb.b, count);
	});
	return bins;
}
