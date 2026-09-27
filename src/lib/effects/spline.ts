/** A curve control point: `[input, output]`, each from 0 through 1. */
export type CurvePoint = readonly [number, number];

/** The smallest x step between control points the package accepts. */
export const MIN_CURVE_POINT_GAP = 0.001;
export const MAX_CURVE_POINTS = 16;

/**
 * Evaluate the monotone cubic Hermite spline the curves effect applies, with Fritsch–Butland
 * tangents, as `spec/effects/curves.md` defines it. The editor draws with this; Wasm applies it.
 */
export function evaluateCurve(points: readonly CurvePoint[], value: number): number {
	const last = points.length - 1;
	const xs = points.map(([x]) => x);
	const ys = points.map(([, y]) => y);
	const v = Math.min(xs[last]!, Math.max(xs[0]!, value));
	if (v === xs[last]) return ys[last]!;
	const h = xs.slice(1).map((x, k) => x - xs[k]!);
	const d = h.map((width, k) => (ys[k + 1]! - ys[k]!) / width);
	const tangent = (k: number) => {
		if (k === 0) return d[0]!;
		if (k === last) return d[last - 1]!;
		const [before, after] = [d[k - 1]!, d[k]!];
		if (before === 0 || after === 0 || Math.sign(before) !== Math.sign(after)) return 0;
		const w1 = 2 * h[k]! + h[k - 1]!;
		const w2 = h[k]! + 2 * h[k - 1]!;
		return (w1 + w2) / (w1 / before + w2 / after);
	};
	const k = xs.findIndex((x, index) => index > 0 && x > v) - 1;
	const [m0, m1, width, slope] = [tangent(k), tangent(k + 1), h[k]!, d[k]!];
	const s = v - xs[k]!;
	const c2 = (3 * slope - 2 * m0 - m1) / width;
	const c3 = (m0 + m1 - 2 * slope) / (width * width);
	return ys[k]! + s * (m0 + s * (c2 + s * c3));
}

/** 2 to 16 `[x, y]` points in `[0, 1]`, each x at least the minimum gap above the previous one. */
export function isValidCurve(points: readonly (readonly number[])[]): boolean {
	return (
		points.length >= 2 &&
		points.length <= MAX_CURVE_POINTS &&
		points.every(
			(point, index) =>
				point.length === 2 &&
				point.every((coordinate) => coordinate >= 0 && coordinate <= 1) &&
				(index === 0 || point[0]! - points[index - 1]![0]! >= MIN_CURVE_POINT_GAP)
		)
	);
}
