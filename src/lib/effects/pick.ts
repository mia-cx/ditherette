import type { ColourChannel, CurvePoints, OneInputCurve, TwoInputCurve } from 'ditherette';
import { wraps } from './grid';
import { coordinates } from '$lib/scopes/colour';
import { CURVE_MODELS } from './catalog';
import { MAX_CURVE_POINTS, evaluateCurve, evaluatePeriodicCurve } from './spline';

/** Curve points sit on the byte grid, like the editor's. */
const BYTE = 255;
/** A picked input this many bytes from a point grabs that point instead of adding one. */
const GRAB_BYTES = 6;

const clamp = (value: number) => Math.min(1, Math.max(0, value));
const onGrid = (value: number) => Math.round(clamp(value) * BYTE) / BYTE;

/** An sRGB colour's value on a curve input, from 0 through 1, as the curve axes read it. */
export function channelValue(channel: ColourChannel, [r, g, b]: readonly [number, number, number]) {
	const out = new Float64Array(3);
	coordinates(channel.model, r, g, b, out);
	const model = CURVE_MODELS.find(({ id }) => id === channel.model)!;
	return clamp(out[model.channels.findIndex(({ name }) => name === channel.channel)]!);
}

/**
 * Grab the point nearest a picked input value, or add one on the curve there. A full curve always
 * grabs. Returns the new points and the grabbed point's index.
 */
export function pickPoint(curve: OneInputCurve, periodic: boolean, value: number) {
	const x = onGrid(value);
	const { points } = curve;
	const distance = ([px]: readonly [number, number]) => {
		const bytes = Math.abs(px - x) * BYTE;
		return periodic ? Math.min(bytes, BYTE - bytes) : bytes;
	};
	const nearest = points.reduce(
		(best, point, index) => (distance(point) < distance(points[best]!) ? index : best),
		0
	);
	if (distance(points[nearest]!) <= GRAB_BYTES || points.length >= MAX_CURVE_POINTS)
		return { points, index: nearest };
	const y = onGrid(periodic ? evaluatePeriodicCurve(points, x) : evaluateCurve(points, x));
	const after = points.findIndex(([px]) => px > x);
	const index = after < 0 ? points.length : after;
	const next: CurvePoints = [...points.slice(0, index), [x, y], ...points.slice(index)];
	return { points: next, index };
}

/** Set one point's output, keeping a periodic curve's seam closed. */
export function setPointOutput(
	points: CurvePoints,
	index: number,
	value: number,
	periodic: boolean
): CurvePoints {
	const y = onGrid(value);
	const seam = periodic && (index === 0 || index === points.length - 1);
	return points.map(([x, py], other) =>
		other === index || (seam && (other === 0 || other === points.length - 1)) ? [x, y] : [x, py]
	);
}

/** The grid point nearest a picked colour on both of a two-input curve's axes. */
export function pickCell(curve: TwoInputCurve, rgb: readonly [number, number, number]) {
	const a = channelValue(curve.x, rgb);
	const b = channelValue(curve.x2, rgb);
	const distance = (position: number, value: number, closed: boolean) => {
		const d = Math.abs(position - value);
		return closed ? Math.min(d, 1 - d) : d;
	};
	let best = { row: 0, column: 0, distance: Infinity };
	curve.grid.rows.forEach((r, row) =>
		curve.grid.columns.forEach((c, column) => {
			const d = Math.hypot(distance(c, a, wraps(curve.x)), distance(r, b, wraps(curve.x2)));
			if (d < best.distance) best = { row, column, distance: d };
		})
	);
	return { row: best.row, column: best.column };
}
