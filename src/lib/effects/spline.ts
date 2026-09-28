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
	const tangentAt = (k: number) => {
		if (k === 0) return d[0]!;
		if (k === last) return d[last - 1]!;
		return tangent(d[k - 1]!, d[k]!, h[k - 1]!, h[k]!);
	};
	const k = xs.findIndex((x, index) => index > 0 && x > v) - 1;
	const [m0, m1, width, slope] = [tangentAt(k), tangentAt(k + 1), h[k]!, d[k]!];
	const s = v - xs[k]!;
	const c2 = (3 * slope - 2 * m0 - m1) / width;
	const c3 = (m0 + m1 - 2 * slope) / (width * width);
	return ys[k]! + s * (m0 + s * (c2 + s * c3));
}

/** The Fritsch–Butland tangent between two secants, zero at a turning point. */
function tangent(before: number, after: number, widthBefore: number, widthAfter: number) {
	if (before * after <= 0) return 0;
	const w1 = 2 * widthAfter + widthBefore;
	const w2 = widthAfter + 2 * widthBefore;
	return (w1 + w2) / (w1 / before + w2 / after);
}

/**
 * Evaluate the cyclic spline an arbitrary XY curve uses on a hue x axis, as
 * `spec/effects/channel_curve.md` defines it. Points run from x 0 to x 1 with the same y at both,
 * the seam's tangent joins the last and first segments, and x wraps.
 */
export function evaluatePeriodicCurve(points: readonly CurvePoint[], value: number): number {
	const last = points.length - 1;
	const secant = (k: number) =>
		(points[k + 1]![1] - points[k]![1]) / (points[k + 1]![0] - points[k]![0]);
	const width = (k: number) => points[k + 1]![0] - points[k]![0];
	const seam = tangent(secant(last - 1), secant(0), width(last - 1), width(0));
	const tangentAt = (k: number) =>
		k === 0 || k === last ? seam : tangent(secant(k - 1), secant(k), width(k - 1), width(k));
	const x = ((value % 1) + 1) % 1;
	let k = points.findIndex((point, index) => index > 0 && x < point[0]) - 1;
	if (k < 0) k = last - 1;
	const [m0, m1, h, slope] = [tangentAt(k), tangentAt(k + 1), width(k), secant(k)];
	const s = x - points[k]![0];
	const c2 = (3 * slope - 2 * m0 - m1) / h;
	const c3 = (m0 + m1 - 2 * slope) / (h * h);
	return points[k]![1] + s * (m0 + s * (c2 + s * c3));
}
