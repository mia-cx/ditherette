import type { ColorSpaceId, EnabledPaletteColor, Rgb } from './types';

export type PaletteMatch = { color: EnabledPaletteColor; index: number };

export type PaletteMatcher = {
	colors: EnabledPaletteColor[];
	nearest(rgb: Rgb): PaletteMatch;
	nearestRgb(r: number, g: number, b: number): PaletteMatch;
};

type Vector = readonly [number, number, number];

const REF_X = 0.95047;
const REF_Y = 1;
const REF_Z = 1.08883;
const SRGB_TO_LINEAR = Float64Array.from({ length: 256 }, (_, value) => {
	const channel = value / 255;
	return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
});

export function clampByte(value: number) {
	return Math.min(255, Math.max(0, Math.round(value)));
}

export function srgbToLinearByte(value: number) {
	if (value >= 0 && value <= 255 && Number.isInteger(value)) return SRGB_TO_LINEAR[value]!;
	const channel = value / 255;
	return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
}

function rgbToXyz(r: number, g: number, b: number): Vector {
	const rr = srgbToLinearByte(r);
	const gg = srgbToLinearByte(g);
	const bb = srgbToLinearByte(b);
	return [
		rr * 0.4124564 + gg * 0.3575761 + bb * 0.1804375,
		rr * 0.2126729 + gg * 0.7151522 + bb * 0.072175,
		rr * 0.0193339 + gg * 0.119192 + bb * 0.9503041
	];
}

function labPivot(value: number) {
	return value > 0.008856 ? Math.cbrt(value) : 7.787 * value + 16 / 116;
}

function rgbToLab(r: number, g: number, b: number): Vector {
	const [x, y, z] = rgbToXyz(r, g, b);
	const fx = labPivot(x / REF_X);
	const fy = labPivot(y / REF_Y);
	const fz = labPivot(z / REF_Z);
	return [116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz)];
}

function rgbToOklab(r: number, g: number, b: number): Vector {
	const rr = srgbToLinearByte(r);
	const gg = srgbToLinearByte(g);
	const bb = srgbToLinearByte(b);

	const l = 0.4122214708 * rr + 0.5363325363 * gg + 0.0514459929 * bb;
	const m = 0.2119034982 * rr + 0.6806995451 * gg + 0.1073969566 * bb;
	const s = 0.0883024619 * rr + 0.2817188376 * gg + 0.6299787005 * bb;

	const lRoot = Math.cbrt(l);
	const mRoot = Math.cbrt(m);
	const sRoot = Math.cbrt(s);

	return [
		0.2104542553 * lRoot + 0.793617785 * mRoot - 0.0040720468 * sRoot,
		1.9779984951 * lRoot - 2.428592205 * mRoot + 0.4505937099 * sRoot,
		0.0259040371 * lRoot + 0.7827717662 * mRoot - 0.808675766 * sRoot
	];
}

function rgbToOklch(r: number, g: number, b: number): Vector {
	const [l, a, blue] = rgbToOklab(r, g, b);
	const c = Math.hypot(a, blue);
	const h = Math.atan2(blue, a);
	return [l, c, h];
}

function vectorForRgb(r: number, g: number, b: number, mode: ColorSpaceId): Vector {
	switch (mode) {
		case 'srgb':
		case 'weighted-rgb':
		case 'weighted-rgb-601':
		case 'weighted-rgb-709':
			return [r, g, b];
		case 'linear-rgb':
			return [srgbToLinearByte(r), srgbToLinearByte(g), srgbToLinearByte(b)];
		case 'cielab':
			return rgbToLab(r, g, b);
		case 'oklch':
			return rgbToOklch(r, g, b);
		case 'oklab':
		default:
			return rgbToOklab(r, g, b);
	}
}

export function createPaletteMatcher(
	colors: EnabledPaletteColor[],
	mode: ColorSpaceId
): PaletteMatcher {
	const visible = colors.filter((color) => color.rgb && color.kind !== 'transparent');
	if (visible.length === 0) {
		return {
			colors,
			nearest() {
				throw new Error('No visible palette colors are enabled');
			},
			nearestRgb() {
				throw new Error('No visible palette colors are enabled');
			}
		};
	}

	const count = visible.length;
	const paletteIndex = new Uint8Array(count);
	const red = new Uint8Array(count);
	const green = new Uint8Array(count);
	const blue = new Uint8Array(count);
	const v0 = new Float64Array(count);
	const v1 = new Float64Array(count);
	const v2 = new Float64Array(count);

	for (let ordinal = 0; ordinal < count; ordinal++) {
		const color = visible[ordinal]!;
		const rgb = color.rgb!;
		paletteIndex[ordinal] = colors.indexOf(color);
		red[ordinal] = rgb.r;
		green[ordinal] = rgb.g;
		blue[ordinal] = rgb.b;
		const vector = vectorForRgb(rgb.r, rgb.g, rgb.b, mode);
		v0[ordinal] = vector[0];
		v1[ordinal] = vector[1];
		v2[ordinal] = vector[2];
	}

	function matchOrdinal(ordinal: number): PaletteMatch {
		const index = paletteIndex[ordinal]!;
		return { color: colors[index]!, index };
	}

	function nearestRgb(r: number, g: number, b: number): PaletteMatch {
		let winner = 0;
		let best = Number.POSITIVE_INFINITY;

		if (mode === 'srgb') {
			for (let i = 0; i < count; i++) {
				const dr = r - red[i]!;
				const dg = g - green[i]!;
				const db = b - blue[i]!;
				const distance = dr * dr + dg * dg + db * db;
				if (distance < best) {
					best = distance;
					winner = i;
				}
			}
			return matchOrdinal(winner);
		}

		if (mode === 'weighted-rgb') {
			for (let i = 0; i < count; i++) {
				const pr = red[i]!;
				const dr = r - pr;
				const dg = g - green[i]!;
				const db = b - blue[i]!;
				const meanRed = (r + pr) / 2;
				const distance =
					(2 + meanRed / 256) * dr * dr + 4 * dg * dg + (2 + (255 - meanRed) / 256) * db * db;
				if (distance < best) {
					best = distance;
					winner = i;
				}
			}
			return matchOrdinal(winner);
		}

		if (mode === 'weighted-rgb-601' || mode === 'weighted-rgb-709') {
			const rw = mode === 'weighted-rgb-601' ? 0.299 : 0.2126;
			const gw = mode === 'weighted-rgb-601' ? 0.587 : 0.7152;
			const bw = mode === 'weighted-rgb-601' ? 0.114 : 0.0722;
			for (let i = 0; i < count; i++) {
				const dr = r - red[i]!;
				const dg = g - green[i]!;
				const db = b - blue[i]!;
				const distance = rw * dr * dr + gw * dg * dg + bw * db * db;
				if (distance < best) {
					best = distance;
					winner = i;
				}
			}
			return matchOrdinal(winner);
		}

		const vector = vectorForRgb(r, g, b, mode);
		const x = vector[0];
		const y = vector[1];
		const z = vector[2];
		for (let i = 0; i < count; i++) {
			let distance: number;
			if (mode === 'oklch') {
				const dl = x - v0[i]!;
				const dc = y - v1[i]!;
				const hue = Math.atan2(Math.sin(z - v2[i]!), Math.cos(z - v2[i]!));
				const dh = Math.min(y, v1[i]!) * hue;
				distance = dl * dl + dc * dc + dh * dh;
			} else {
				const dx = x - v0[i]!;
				const dy = y - v1[i]!;
				const dz = z - v2[i]!;
				distance = dx * dx + dy * dy + dz * dz;
			}
			if (distance < best) {
				best = distance;
				winner = i;
			}
		}
		return matchOrdinal(winner);
	}

	return {
		colors,
		nearest(rgb) {
			return nearestRgb(rgb.r, rgb.g, rgb.b);
		},
		nearestRgb
	};
}

export function blendOverMatte(rgb: Rgb, alpha: number, matte: Rgb): Rgb {
	const opacity = alpha / 255;
	return {
		r: clampByte(rgb.r * opacity + matte.r * (1 - opacity)),
		g: clampByte(rgb.g * opacity + matte.g * (1 - opacity)),
		b: clampByte(rgb.b * opacity + matte.b * (1 - opacity))
	};
}
