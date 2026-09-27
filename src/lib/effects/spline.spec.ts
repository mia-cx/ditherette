import { describe, expect, it } from 'vitest';
import { evaluateCurve, type CurvePoint } from './spline';

describe('evaluateCurve', () => {
	it('is the identity for the neutral curve', () => {
		for (const v of [0, 0.2, 0.5, 0.93, 1])
			expect(
				evaluateCurve(
					[
						[0, 0],
						[1, 1]
					],
					v
				)
			).toBe(v);
	});

	it('passes through every point, never overshoots, and is flat past the ends', () => {
		const points: CurvePoint[] = [
			[0.1, 0.05],
			[0.4, 0.6],
			[0.6, 0.62],
			[0.9, 0.95]
		];
		for (const [x, y] of points) expect(evaluateCurve(points, x)).toBeCloseTo(y, 12);
		for (let v = 0.1; v <= 0.9; v += 0.01) {
			const y = evaluateCurve(points, v);
			expect(y).toBeGreaterThanOrEqual(0.05);
			expect(y).toBeLessThanOrEqual(0.95);
		}
		expect(evaluateCurve(points, 0)).toBe(0.05);
		expect(evaluateCurve(points, 1)).toBe(0.95);
	});
});
