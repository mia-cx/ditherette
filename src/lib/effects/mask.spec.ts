import { isEffect } from 'ditherette';
import { describe, expect, it } from 'vitest';
import { MASK_PRESETS, presetCurve, presetValues, setPreset, type MaskPreset } from './mask';

const [tones, colours, saturation] = MASK_PRESETS as [MaskPreset, MaskPreset, MaskPreset];

describe('mask presets', () => {
	it('are a no-op at 100% and leave the mask', () => {
		for (const preset of MASK_PRESETS) {
			const full = preset.ranges.map(() => 1);
			expect(presetCurve(preset, full)).toBeUndefined();
			const half = preset.ranges.map(() => 0.5);
			expect(setPreset(setPreset([], preset, half), preset, full)).toEqual([]);
		}
	});

	it('generate curves the package accepts, whose centres read back as the sliders', () => {
		for (const preset of MASK_PRESETS)
			for (let trial = 0; trial < 20; trial++) {
				const values = preset.ranges.map((_, index) => ((trial * 7 + index * 3) % 11) / 10);
				if (values.every((value) => value === 1)) continue;
				const mask = setPreset([], preset, values);
				expect(isEffect({ effect: 'exposure', enabled: true, stops: 1, mask })).toBe(true);
				expect(mask[0]!.points!.length).toBeLessThanOrEqual(16);
				presetValues(mask, preset).forEach((value, index) =>
					expect(value).toBeCloseTo(values[index]!, 6)
				);
			}
	});

	it('blend neighbouring ranges along raised cosines that sum to 1', () => {
		const shadowsOff = presetCurve(tones, [0, 1, 1])!;
		const at = (x: number) => shadowsOff.points.find(([px]) => px === x)![1];
		expect(at(0)).toBe(0);
		expect(at(0.25)).toBeCloseTo(0.5, 12);
		expect(at(0.5)).toBe(1);
		expect(at(1)).toBe(1);
		const mutedOff = presetCurve(saturation, [0, 1])!;
		expect(mutedOff.points.at(-1)).toEqual([1, 1]);
		expect(mutedOff.points.find(([x]) => x === 0.25)![1]).toBeCloseTo(0.5, 12);
	});

	it('centre Colours on the OKLCH hues of the eight primaries and wrap at the seam', () => {
		const degrees = colours.ranges.map(({ centre }) => Math.round(centre * 360));
		expect(degrees).toEqual([29, 56, 110, 142, 195, 264, 297, 328]);
		const redsOff = presetCurve(colours, [0, 1, 1, 1, 1, 1, 1, 1])!;
		expect(redsOff.points[0]![1]).toBe(redsOff.points.at(-1)![1]);
		expect(redsOff.points[0]![1]).toBeLessThan(1);
	});

	it('replace their own curve in place and keep other curves', () => {
		const custom = {
			x: { model: 'oklab', channel: 'a' } as const,
			points: [
				[0, 1],
				[1, 0]
			] as const
		};
		const first = setPreset([custom], tones, [0.2, 1, 1]);
		expect(first).toHaveLength(2);
		const second = setPreset(first, tones, [0.4, 1, 1]);
		expect(second[0]).toBe(custom);
		expect(presetValues(second, tones)[0]).toBeCloseTo(0.4, 6);
	});
});
