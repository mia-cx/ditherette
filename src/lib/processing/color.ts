import type { ColorSpaceId, EnabledPaletteColor, Rgb } from './types';

export type PaletteMatch = { color: EnabledPaletteColor; index: number };

export type PaletteMatcherMemoStats = {
	rgbHits: number;
	rgbMisses: number;
	rgbSets: number;
	rgbEvictions: number;
	candidateEvaluations: number;
	denseRgbMemoBytes: number;
	distanceTableBytes: number;
};

export type PaletteMatcherOptions = {
	rgbMemoEntries?: number;
	denseRgbMemo?: boolean;
	distanceTables?: boolean;
};

export type PaletteMatcher = {
	colors: EnabledPaletteColor[];
	nearest(rgb: Rgb): PaletteMatch;
	nearestRgb(r: number, g: number, b: number): PaletteMatch;
	nearestIndexRgb(r: number, g: number, b: number): number;
	nearestIndexByteRgb(r: number, g: number, b: number): number;
	paletteRgbAt(index: number): Rgb;
	paletteRedAt(index: number): number;
	paletteGreenAt(index: number): number;
	paletteBlueAt(index: number): number;
	memoStats(): PaletteMatcherMemoStats;
};

type Vector = readonly [number, number, number];

const REF_X = 0.95047;
const REF_Y = 1;
const REF_Z = 1.08883;
const RGB24_SIZE = 256 * 256 * 256;
const DEFAULT_RGB_MEMO_ENTRIES = 0;
const FALLBACK_RGB = { r: 0, g: 0, b: 0 } satisfies Rgb;
const WEIGHTED_RGB_601_R = Math.sqrt(0.299);
const WEIGHTED_RGB_601_G = Math.sqrt(0.587);
const WEIGHTED_RGB_601_B = Math.sqrt(0.114);
const WEIGHTED_RGB_709_R = Math.sqrt(0.2126);
const WEIGHTED_RGB_709_G = Math.sqrt(0.7152);
const WEIGHTED_RGB_709_B = Math.sqrt(0.0722);
const SRGB_TO_LINEAR = Float64Array.from({ length: 256 }, (_, value) => {
	const channel = value / 255;
	return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
});

export function clampByte(value: number) {
	return Math.min(255, Math.max(0, Math.round(value)));
}

export function srgbByteToLinear(value: number) {
	return SRGB_TO_LINEAR[value]!;
}

export function srgbToLinearByte(value: number) {
	if (value >= 0 && value <= 255 && Number.isInteger(value)) return srgbByteToLinear(value);
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

export function vectorForRgb(r: number, g: number, b: number, mode: ColorSpaceId): Vector {
	switch (mode) {
		case 'srgb':
		case 'weighted-rgb':
			return [r, g, b];
		case 'weighted-rgb-601':
			return [r * WEIGHTED_RGB_601_R, g * WEIGHTED_RGB_601_G, b * WEIGHTED_RGB_601_B];
		case 'weighted-rgb-709':
			return [r * WEIGHTED_RGB_709_R, g * WEIGHTED_RGB_709_G, b * WEIGHTED_RGB_709_B];
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
	mode: ColorSpaceId,
	options: PaletteMatcherOptions = {}
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
			},
			nearestIndexRgb() {
				throw new Error('No visible palette colors are enabled');
			},
			nearestIndexByteRgb() {
				throw new Error('No visible palette colors are enabled');
			},
			paletteRgbAt() {
				throw new Error('No visible palette colors are enabled');
			},
			paletteRedAt() {
				throw new Error('No visible palette colors are enabled');
			},
			paletteGreenAt() {
				throw new Error('No visible palette colors are enabled');
			},
			paletteBlueAt() {
				throw new Error('No visible palette colors are enabled');
			},
			memoStats() {
				return {
					rgbHits: 0,
					rgbMisses: 0,
					rgbSets: 0,
					rgbEvictions: 0,
					candidateEvaluations: 0,
					denseRgbMemoBytes: 0,
					distanceTableBytes: 0
				};
			}
		};
	}

	if (colors.length > 256) throw new Error('Palette matching supports at most 256 colors.');

	const count = visible.length;
	const paletteIndex = new Uint8Array(count);
	const red = new Uint8Array(count);
	const green = new Uint8Array(count);
	const blue = new Uint8Array(count);
	const paletteRedByIndex = new Uint8Array(colors.length);
	const paletteGreenByIndex = new Uint8Array(colors.length);
	const paletteBlueByIndex = new Uint8Array(colors.length);
	const v0 = new Float64Array(count);
	const v1 = new Float64Array(count);
	const v2 = new Float64Array(count);

	for (let ordinal = 0; ordinal < count; ordinal++) {
		const color = visible[ordinal]!;
		const rgb = color.rgb!;
		const index = colors.indexOf(color);
		paletteIndex[ordinal] = index;
		red[ordinal] = rgb.r;
		green[ordinal] = rgb.g;
		blue[ordinal] = rgb.b;
		paletteRedByIndex[index] = rgb.r;
		paletteGreenByIndex[index] = rgb.g;
		paletteBlueByIndex[index] = rgb.b;
		const vector = vectorForRgb(rgb.r, rgb.g, rgb.b, mode);
		v0[ordinal] = vector[0];
		v1[ordinal] = vector[1];
		v2[ordinal] = vector[2];
	}

	const rgbMemoMaxEntries = Math.max(0, options.rgbMemoEntries ?? DEFAULT_RGB_MEMO_ENTRIES);
	const maxPaletteIndex = paletteIndex.reduce((max, index) => Math.max(max, index), 0);
	const denseRgbMemo = options.denseRgbMemo
		? maxPaletteIndex < 255
			? new Uint8Array(RGB24_SIZE)
			: new Uint16Array(RGB24_SIZE)
		: undefined;
	const denseRgbMemoBytes = denseRgbMemo?.byteLength ?? 0;
	let distanceTableBytes = 0;
	const rgbMemo = new Map<number, number>();
	let lastRgbKey = -1;
	let lastRgbIndex = -1;
	const stats: PaletteMatcherMemoStats = {
		rgbHits: 0,
		rgbMisses: 0,
		rgbSets: 0,
		rgbEvictions: 0,
		candidateEvaluations: 0,
		denseRgbMemoBytes,
		distanceTableBytes: 0
	};
	function matchIndex(index: number): PaletteMatch {
		return { color: colors[index]!, index };
	}

	const nearestOrdinal = (() => {
		if (options.distanceTables && mode === 'srgb') {
			const rTerms = new Float64Array(count * 256);
			const gTerms = new Float64Array(count * 256);
			const bTerms = new Float64Array(count * 256);
			distanceTableBytes += rTerms.byteLength + gTerms.byteLength + bTerms.byteLength;
			for (let i = 0; i < count; i++) {
				const base = i << 8;
				const pr = red[i]!;
				const pg = green[i]!;
				const pb = blue[i]!;
				for (let value = 0; value < 256; value++) {
					const dr = value - pr;
					const dg = value - pg;
					const db = value - pb;
					rTerms[base + value] = dr * dr;
					gTerms[base + value] = dg * dg;
					bTerms[base + value] = db * db;
				}
			}
			return (r: number, g: number, b: number) => {
				let winner = 0;
				let best = Number.POSITIVE_INFINITY;
				for (let i = 0; i < count; i++) {
					const base = i << 8;
					const distance = rTerms[base + r]! + gTerms[base + g]! + bTerms[base + b]!;
					if (distance < best) {
						best = distance;
						winner = i;
					}
				}
				return winner;
			};
		}

		if (options.distanceTables && mode === 'linear-rgb') {
			const rTerms = new Float64Array(count * 256);
			const gTerms = new Float64Array(count * 256);
			const bTerms = new Float64Array(count * 256);
			distanceTableBytes += rTerms.byteLength + gTerms.byteLength + bTerms.byteLength;
			for (let i = 0; i < count; i++) {
				const base = i << 8;
				const pr = v0[i]!;
				const pg = v1[i]!;
				const pb = v2[i]!;
				for (let value = 0; value < 256; value++) {
					const linear = srgbByteToLinear(value);
					const dr = linear - pr;
					const dg = linear - pg;
					const db = linear - pb;
					rTerms[base + value] = dr * dr;
					gTerms[base + value] = dg * dg;
					bTerms[base + value] = db * db;
				}
			}
			return (r: number, g: number, b: number) => {
				let winner = 0;
				let best = Number.POSITIVE_INFINITY;
				for (let i = 0; i < count; i++) {
					const base = i << 8;
					const distance = rTerms[base + r]! + gTerms[base + g]! + bTerms[base + b]!;
					if (distance < best) {
						best = distance;
						winner = i;
					}
				}
				return winner;
			};
		}

		if (options.distanceTables && (mode === 'weighted-rgb-601' || mode === 'weighted-rgb-709')) {
			const rw = mode === 'weighted-rgb-601' ? 0.299 : 0.2126;
			const gw = mode === 'weighted-rgb-601' ? 0.587 : 0.7152;
			const bw = mode === 'weighted-rgb-601' ? 0.114 : 0.0722;
			const rTerms = new Float64Array(count * 256);
			const gTerms = new Float64Array(count * 256);
			const bTerms = new Float64Array(count * 256);
			distanceTableBytes += rTerms.byteLength + gTerms.byteLength + bTerms.byteLength;
			for (let i = 0; i < count; i++) {
				const base = i << 8;
				const pr = red[i]!;
				const pg = green[i]!;
				const pb = blue[i]!;
				for (let value = 0; value < 256; value++) {
					const dr = value - pr;
					const dg = value - pg;
					const db = value - pb;
					rTerms[base + value] = rw * dr * dr;
					gTerms[base + value] = gw * dg * dg;
					bTerms[base + value] = bw * db * db;
				}
			}
			return (r: number, g: number, b: number) => {
				let winner = 0;
				let best = Number.POSITIVE_INFINITY;
				for (let i = 0; i < count; i++) {
					const base = i << 8;
					const distance = rTerms[base + r]! + gTerms[base + g]! + bTerms[base + b]!;
					if (distance < best) {
						best = distance;
						winner = i;
					}
				}
				return winner;
			};
		}

		if (mode === 'srgb') {
			return (r: number, g: number, b: number) => {
				let winner = 0;
				let best = Number.POSITIVE_INFINITY;
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
				return winner;
			};
		}

		if (mode === 'weighted-rgb') {
			return (r: number, g: number, b: number) => {
				let winner = 0;
				let best = Number.POSITIVE_INFINITY;
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
				return winner;
			};
		}

		if (mode === 'weighted-rgb-601' || mode === 'weighted-rgb-709') {
			const rw = mode === 'weighted-rgb-601' ? 0.299 : 0.2126;
			const gw = mode === 'weighted-rgb-601' ? 0.587 : 0.7152;
			const bw = mode === 'weighted-rgb-601' ? 0.114 : 0.0722;
			return (r: number, g: number, b: number) => {
				let winner = 0;
				let best = Number.POSITIVE_INFINITY;
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
				return winner;
			};
		}

		if (mode === 'oklch') {
			return (r: number, g: number, b: number) => {
				let winner = 0;
				let best = Number.POSITIVE_INFINITY;
				const [x, y, z] = vectorForRgb(r, g, b, mode);
				for (let i = 0; i < count; i++) {
					const dl = x - v0[i]!;
					const dc = y - v1[i]!;
					let hue = Math.abs(z - v2[i]!);
					if (hue > Math.PI) hue = Math.PI * 2 - hue;
					const dh = Math.min(y, v1[i]!) * hue;
					const distance = dl * dl + dc * dc + dh * dh;
					if (distance < best) {
						best = distance;
						winner = i;
					}
				}
				return winner;
			};
		}

		return (r: number, g: number, b: number) => {
			let winner = 0;
			let best = Number.POSITIVE_INFINITY;
			const [x, y, z] = vectorForRgb(r, g, b, mode);
			for (let i = 0; i < count; i++) {
				const dx = x - v0[i]!;
				const dy = y - v1[i]!;
				const dz = z - v2[i]!;
				const distance = dx * dx + dy * dy + dz * dz;
				if (distance < best) {
					best = distance;
					winner = i;
				}
			}
			return winner;
		};
	})();

	function nearestIndexByteRgb(r: number, g: number, b: number): number {
		const key = (r << 16) | (g << 8) | b;
		if (key === lastRgbKey) {
			stats.rgbHits++;
			return lastRgbIndex;
		}
		if (denseRgbMemo) {
			const stored = denseRgbMemo[key]!;
			if (stored !== 0) {
				const cached = stored - 1;
				lastRgbKey = key;
				lastRgbIndex = cached;
				stats.rgbHits++;
				return cached;
			}
		} else {
			const cached = rgbMemo.get(key);
			if (cached !== undefined) {
				lastRgbKey = key;
				lastRgbIndex = cached;
				stats.rgbHits++;
				return cached;
			}
		}
		stats.rgbMisses++;
		stats.candidateEvaluations += count;

		const index = paletteIndex[nearestOrdinal(r, g, b)]!;
		lastRgbKey = key;
		lastRgbIndex = index;
		stats.rgbSets++;
		if (denseRgbMemo) {
			denseRgbMemo[key] = index + 1;
		} else if (rgbMemoMaxEntries > 0) {
			if (rgbMemo.size >= rgbMemoMaxEntries) {
				stats.rgbEvictions += rgbMemo.size;
				rgbMemo.clear();
			}
			rgbMemo.set(key, index);
		}
		return index;
	}

	function nearestIndexRgb(r: number, g: number, b: number): number {
		if (isByte(r) && isByte(g) && isByte(b)) return nearestIndexByteRgb(r, g, b);
		return paletteIndex[nearestOrdinal(r, g, b)]!;
	}

	return {
		colors,
		nearest(rgb) {
			return matchIndex(nearestIndexRgb(rgb.r, rgb.g, rgb.b));
		},
		nearestRgb(r, g, b) {
			return matchIndex(nearestIndexRgb(r, g, b));
		},
		nearestIndexRgb,
		nearestIndexByteRgb,
		paletteRgbAt(index) {
			return colors[index]?.rgb ?? FALLBACK_RGB;
		},
		paletteRedAt(index) {
			return paletteRedByIndex[index] ?? 0;
		},
		paletteGreenAt(index) {
			return paletteGreenByIndex[index] ?? 0;
		},
		paletteBlueAt(index) {
			return paletteBlueByIndex[index] ?? 0;
		},
		memoStats() {
			stats.distanceTableBytes = distanceTableBytes;
			return { ...stats };
		}
	};
}

function isByte(value: number) {
	return Number.isInteger(value) && value >= 0 && value <= 255;
}
