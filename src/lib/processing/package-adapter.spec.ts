import { describe, expect, it } from 'vitest';
import { packageProcessRequest, packageQuantizeResult } from './package-adapter';
import type { ColorSpaceId, EnabledPaletteColor, ProcessingSettings, ResizeId } from './types';

const palette: EnabledPaletteColor[] = [
	{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'custom', enabled: true }
];
const settings: ProcessingSettings = {
	output: {
		width: 1,
		height: 1,
		lockAspect: true,
		resize: 'nearest',
		alphaMode: 'preserve',
		alphaThreshold: 0,
		matteKey: '#000000',
		autoSizeOnUpload: false,
		scaleFactor: 1
	},
	dither: {
		algorithm: 'none',
		strength: 100,
		placement: 'everywhere',
		placementRadius: 3,
		placementThreshold: 12,
		placementSoftness: 8,
		serpentine: true,
		seed: 1,
		useColorSpace: false
	},
	colorSpace: 'srgb'
};
const source = { width: 1, height: 1, data: new Uint8ClampedArray([0, 0, 0, 255]) };

describe('website package request', () => {
	it('maps the existing settings to the public process contract', () => {
		const { request, warnings } = packageProcessRequest(source, palette, settings, settings.output);
		expect(request).toEqual({
			source: { width: 1, height: 1, data: new Uint8Array([0, 0, 0, 255]) },
			palette: [{ kind: 'color', rgb: [0, 0, 0] }],
			recipe: {
				version: 1,
				output: { width: 1, height: 1, resize: { algorithm: 'nearest', anchor: 'center' } },
				alpha: { mode: 'preserve', threshold: 0 },
				match: 'srgb-euclidean',
				dither: { family: 'none' }
			}
		});
		expect(warnings).toEqual([]);
	});
	it.each([
		['nearest', { algorithm: 'nearest', anchor: 'center' }],
		['bilinear', { algorithm: 'bilinear', anchor: 'center' }],
		['area', { algorithm: 'area' }],
		['lanczos2', { algorithm: 'lanczos2', anchor: 'center', support: 'fixed' }],
		['lanczos2-scale-aware', { algorithm: 'lanczos2', anchor: 'center', support: 'scale-aware' }],
		['lanczos3', { algorithm: 'lanczos3', anchor: 'center', support: 'fixed' }],
		['lanczos3-scale-aware', { algorithm: 'lanczos3', anchor: 'center', support: 'scale-aware' }]
	] satisfies [ResizeId, object][])('maps resize %s', (resize, expected) => {
		expect(
			packageProcessRequest(
				source,
				palette,
				{ ...settings, output: { ...settings.output, resize } },
				settings.output
			).request.recipe.output.resize
		).toEqual(expected);
	});
	it.each([
		['srgb', 'srgb-euclidean', 'srgb'],
		['linear-rgb', 'linear-rgb-euclidean', 'linear-rgb'],
		['oklab', 'oklab-euclidean', 'oklab'],
		['cielab', 'cielab-euclidean', 'cielab'],
		['oklch', 'oklch-hue-arc', 'oklch'],
		['weighted-rgb', 'srgb-compuphase', 'srgb'],
		['weighted-rgb-601', 'srgb-rec601', 'srgb'],
		['weighted-rgb-709', 'srgb-rec709', 'srgb']
	] satisfies [ColorSpaceId, string, string][])(
		'maps %s matching and vector dither coordinates',
		(colorSpace, match, space) => {
			const { recipe } = packageProcessRequest(
				source,
				palette,
				{
					...settings,
					colorSpace,
					dither: { ...settings.dither, algorithm: 'bayer-4', useColorSpace: true, strength: 50 }
				},
				settings.output
			).request;
			expect(recipe.match).toBe(match);
			expect(recipe.dither).toEqual({
				family: 'separable',
				perturb: {
					field: { algorithm: 'bayer', size: '4' },
					space,
					strength: colorSpace === 'weighted-rgb' ? (0.5 * 64) / 63.75 : 0.5,
					placement: { mode: 'everywhere' }
				}
			});
		}
	);
	it.each(['bayer-2', 'bayer-4', 'bayer-8', 'bayer-16'] as const)(
		'converts the historical 64-byte field scale for %s',
		(algorithm) => {
			const { recipe } = packageProcessRequest(
				source,
				palette,
				{ ...settings, dither: { ...settings.dither, algorithm, strength: 25 } },
				settings.output
			).request;
			expect(recipe.dither).toEqual({
				family: 'separable',
				perturb: {
					field: { algorithm: 'bayer', size: algorithm.slice(6) },
					space: 'srgb',
					strength: (0.25 * 64) / 63.75,
					placement: { mode: 'everywhere' }
				}
			});
		}
	);
	it('normalizes random seed and adaptive radius while retaining placement percentage points', () => {
		const { recipe } = packageProcessRequest(
			source,
			palette,
			{
				...settings,
				dither: {
					...settings.dither,
					algorithm: 'random',
					seed: -1,
					placement: 'adaptive',
					placementRadius: 0.4
				}
			},
			settings.output
		).request;
		expect(recipe.dither).toEqual({
			family: 'separable',
			perturb: {
				field: { algorithm: 'random', seed: 4294967295 },
				space: 'srgb',
				strength: 64 / 63.75,
				placement: { mode: 'adaptive', radius: 1, threshold: 12, softness: 8 }
			}
		});
	});
	it.each(['floyd-steinberg', 'sierra', 'sierra-lite'] as const)(
		'maps %s diffusion without the field strength adjustment',
		(algorithm) => {
			for (const useColorSpace of [true, false]) {
				const { recipe } = packageProcessRequest(
					source,
					palette,
					{ ...settings, dither: { ...settings.dither, algorithm, strength: 50, useColorSpace } },
					settings.output
				).request;
				expect(recipe.dither).toEqual({
					family: 'diffusion',
					kernel: algorithm,
					feedback: useColorSpace ? 'matching' : 'srgb-bytes',
					strength: 0.5,
					serpentine: true,
					placement: { mode: 'everywhere' }
				});
			}
		}
	);
	it('disables zero-strength dithering and CompuPhase vector feedback', () => {
		const base = {
			...settings,
			colorSpace: 'weighted-rgb',
			dither: { ...settings.dither, algorithm: 'sierra', useColorSpace: true }
		} satisfies ProcessingSettings;
		expect(
			packageProcessRequest(source, palette, base, settings.output).request.recipe.dither
		).toMatchObject({ feedback: 'srgb-bytes' });
		expect(
			packageProcessRequest(
				source,
				palette,
				{ ...base, dither: { ...base.dither, strength: 0 } },
				settings.output
			).request.recipe.dither
		).toEqual({ family: 'none' });
	});
	it('packs integer crop rows without mutating source bytes and clamps the crop at the image edge', () => {
		const source = {
			width: 3,
			height: 2,
			data: Uint8ClampedArray.from({ length: 24 }, (_, index) => index)
		};
		const before = source.data.slice();
		const cropped = {
			...settings,
			output: { ...settings.output, crop: { x: 1, y: 0, width: 9, height: 2 } }
		};
		const { request } = packageProcessRequest(source, palette, cropped, settings.output);
		expect(request.source).toEqual({
			width: 2,
			height: 2,
			data: new Uint8Array([4, 5, 6, 7, 8, 9, 10, 11, 16, 17, 18, 19, 20, 21, 22, 23])
		});
		expect(source.data).toEqual(before);
	});
	it('rejects fractional crop origins and extents after the existing clamp', () => {
		for (const crop of [
			{ x: 0.5, y: 0, width: 1, height: 1 },
			{ x: 0, y: 0, width: 1.5, height: 1 }
		]) {
			expect(() =>
				packageProcessRequest(
					{ ...source, width: 2, data: new Uint8ClampedArray(8) },
					palette,
					{ ...settings, output: { ...settings.output, crop } },
					settings.output
				)
			).toThrow(/fractional crop/);
		}
	});
	it('uses the existing disabled and missing matte policy', () => {
		for (const matteKey of ['#FFFFFF', 'missing']) {
			const mapped = packageProcessRequest(
				source,
				palette,
				{ ...settings, output: { ...settings.output, alphaMode: 'matte', matteKey } },
				settings.output
			);
			expect(mapped.request.recipe.alpha).toEqual({ mode: 'matte', rgb: [0, 0, 0] });
			expect(mapped.warnings).toHaveLength(1);
		}
		expect(
			packageProcessRequest(
				source,
				palette,
				{ ...settings, output: { ...settings.output, alphaMode: 'premultiplied' } },
				settings.output
			).request.recipe.alpha
		).toEqual({ mode: 'premultiplied' });
	});
	it('restores ordered metadata, duplicate colors, transparency, truncation, and package warnings', () => {
		const colors: EnabledPaletteColor[] = [
			...palette,
			{ ...palette[0]!, name: 'Duplicate', key: 'duplicate', tags: ['kept'] },
			{ name: 'Clear', key: 'clear', kind: 'transparent', enabled: true },
			...Array.from({ length: 254 }, () => palette[0]!)
		];
		const rgba = new Uint8Array(256 * 4);
		const result = packageQuantizeResult(
			{
				width: 1,
				height: 1,
				indices: new Uint8Array([2]),
				palette: { rgba, transparentIndex: 2 },
				warnings: [{ code: 'palette-truncated', message: 'truncated' }]
			},
			colors,
			['matte']
		);
		expect(result.palette).toHaveLength(256);
		expect(result.palette[1]).toEqual(colors[1]);
		expect(result.palette[2]).toEqual(colors[2]);
		expect(result.transparentIndex).toBe(2);
		expect(result.warnings).toEqual(['truncated', 'matte']);
		expect(
			packageProcessRequest(source, colors, settings, settings.output).request.palette
		).toHaveLength(257);
	});
});
