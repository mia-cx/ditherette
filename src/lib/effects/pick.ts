import type { ColourChannel, Curve, CurvePoints } from 'ditherette';
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
export function pickPoint(curve: Curve, periodic: boolean, value: number) {
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
