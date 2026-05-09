import { describe, expect, it } from 'vitest';
import {
	blendOverMatte,
	compositedRgb,
	premultiplyRgb,
	unpremultiplyRgb,
	unpremultiplySample
} from './compositing';
import type { Rgb } from './types';

function expectRgbClose(actual: Rgb, expected: Rgb, tolerance = 1) {
	expect(Math.abs(actual.r - expected.r)).toBeLessThanOrEqual(tolerance);
	expect(Math.abs(actual.g - expected.g)).toBeLessThanOrEqual(tolerance);
	expect(Math.abs(actual.b - expected.b)).toBeLessThanOrEqual(tolerance);
}

describe('alpha compositing helpers', () => {
	it('blends straight-alpha RGB over a matte', () => {
		expect(blendOverMatte({ r: 255, g: 0, b: 0 }, 128, { r: 0, g: 0, b: 255 })).toEqual({
			r: 128,
			g: 0,
			b: 127
		});
	});

	it('premultiplies and unpremultiplies opaque colors without changing them', () => {
		const rgb = { r: 24, g: 128, b: 240 };

		expect(premultiplyRgb(rgb, 255)).toEqual(rgb);
		expect(unpremultiplyRgb(rgb, 255)).toEqual(rgb);
	});

	it('round trips semitransparent premultiplied colors within byte rounding tolerance', () => {
		const rgb = { r: 200, g: 100, b: 50 };
		const premultiplied = premultiplyRgb(rgb, 128);

		expectRgbClose(unpremultiplyRgb(premultiplied, 128), rgb);
	});

	it('returns transparent black when unpremultiplying zero alpha', () => {
		expect(unpremultiplyRgb({ r: 100, g: 50, b: 25 }, 0)).toEqual({ r: 0, g: 0, b: 0 });
		expect(unpremultiplySample(100, 50, 25, 0)).toEqual([0, 0, 0, 0]);
	});

	it('dispatches output alpha modes through one compositing contract', () => {
		const rgb = { r: 200, g: 100, b: 50 };
		const matte = { r: 0, g: 0, b: 0 };

		expect(compositedRgb(rgb, 128, 'preserve', matte)).toEqual(rgb);
		expect(compositedRgb(rgb, 128, 'premultiplied', matte)).toEqual(premultiplyRgb(rgb, 128));
		expect(compositedRgb(rgb, 128, 'matte', matte)).toEqual(blendOverMatte(rgb, 128, matte));
	});
});
