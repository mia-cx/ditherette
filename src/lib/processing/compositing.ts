import type { Rgb } from './types';
import { clampByte } from './color';

export type RgbaSample = readonly [number, number, number, number];

export function opacityFromAlpha(alpha: number) {
	return Math.min(1, Math.max(0, alpha / 255));
}

export function blendOverMatte(rgb: Rgb, alpha: number, matte: Rgb): Rgb {
	const opacity = opacityFromAlpha(alpha);
	const background = 1 - opacity;
	return {
		r: clampByte(rgb.r * opacity + matte.r * background),
		g: clampByte(rgb.g * opacity + matte.g * background),
		b: clampByte(rgb.b * opacity + matte.b * background)
	};
}

export function premultiplyRgb(rgb: Rgb, alpha: number): Rgb {
	const opacity = opacityFromAlpha(alpha);
	return {
		r: clampByte(rgb.r * opacity),
		g: clampByte(rgb.g * opacity),
		b: clampByte(rgb.b * opacity)
	};
}

export function unpremultiplyRgb(rgb: Rgb, alpha: number): Rgb {
	const opacity = opacityFromAlpha(alpha);
	if (opacity <= 0) return { r: 0, g: 0, b: 0 };
	return {
		r: clampByte(rgb.r / opacity),
		g: clampByte(rgb.g / opacity),
		b: clampByte(rgb.b / opacity)
	};
}

export function compositedRgb(
	rgb: Rgb,
	alpha: number,
	mode: 'preserve' | 'premultiplied' | 'matte',
	matte: Rgb
): Rgb {
	if (mode === 'matte') return blendOverMatte(rgb, alpha, matte);
	if (mode === 'premultiplied') return premultiplyRgb(rgb, alpha);
	return rgb;
}

export function unpremultiplySample(r: number, g: number, b: number, a: number): RgbaSample {
	if (a <= 0) return [0, 0, 0, 0];
	const opacity = a / 255;
	return [clampByte(r / opacity), clampByte(g / opacity), clampByte(b / opacity), clampByte(a)];
}
