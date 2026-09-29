import { describe, expect, it } from 'vitest';
import { FLAT, STRAIGHT, neutralCurve } from './catalog';
import { channelValue, pickPoint, setPointOutput } from './pick';

const lightness = neutralCurve(
	{ model: 'oklch', channel: 'lightness' },
	{ model: 'oklch', channel: 'lightness' }
);
const hueVsChroma = neutralCurve(
	{ model: 'oklch', channel: 'hue' },
	{ model: 'oklch', channel: 'chroma' }
);

describe('curve picking', () => {
	it('reads a colour on the curve input axis', () => {
		expect(channelValue({ model: 'srgb', channel: 'red' }, [255, 0, 0])).toBe(1);
		expect(channelValue(lightness.x, [255, 255, 255])).toBeCloseTo(1, 3);
		expect(channelValue(lightness.x, [0, 0, 0])).toBe(0);
	});

	it('adds a point on the curve, or grabs one close by', () => {
		const added = pickPoint(lightness, false, 0.5);
		expect(added.points).toEqual([STRAIGHT[0], [128 / 255, 128 / 255], STRAIGHT[1]]);
		expect(added.index).toBe(1);
		const grabbed = pickPoint({ ...lightness, points: added.points }, false, 0.51);
		expect(grabbed).toEqual({ points: added.points, index: 1 });
	});

	it('grabs across the hue seam and keeps it closed', () => {
		const { points, index } = pickPoint(hueVsChroma, true, 0.995);
		expect(points).toBe(hueVsChroma.points);
		expect(index).toBe(FLAT.length - 1);
		const pushed = setPointOutput(points, index, 0.8, true);
		expect(pushed[0]![1]).toBe(pushed[pushed.length - 1]![1]);
		expect(pushed[0]![1]).toBeCloseTo(0.8, 2);
	});
});
