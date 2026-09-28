import type { ColourChannel, CurveGrid, TwoInputCurve } from 'ditherette';
import { evaluateClosedCurve, evaluateCurve } from './spline';

/** A hue axis wraps; every other axis runs from 0 to 1. */
export const wraps = (channel: ColourChannel) => channel.channel === 'hue';

/** New grids: a column every 30° on a hue axis, nine on others, and five rows. */
const HUE_COLUMNS = 12;
const OPEN_COLUMNS = 9;
const ROWS = 5;
const NEUTRAL = 0.5;

const positions = (count: number, closed: boolean) =>
	Array.from({ length: count }, (_, index) => (closed ? index / count : index / (count - 1)));

/** A neutral grid for a curve reading `x` across and `x2` down. */
export function neutralGrid(x: ColourChannel, x2: ColourChannel): CurveGrid {
	const columns = positions(wraps(x) ? HUE_COLUMNS : OPEN_COLUMNS, wraps(x));
	const rows = positions(ROWS, wraps(x2));
	return { columns, rows, values: rows.map(() => columns.map(() => NEUTRAL)) };
}

function along(axis: readonly number[], values: readonly number[], closed: boolean, at: number) {
	if (closed) return evaluateClosedCurve(axis, values, at);
	return evaluateCurve(
		axis.map((position, index) => [position, values[index]!] as const),
		at
	);
}

/**
 * The grid's value at `a` on x and `b` on x2, from 0 through 1: along each row first, then down
 * the row results, as `spec/effects/curves.md` defines it.
 */
export function evaluateGrid({ x, x2, grid }: TwoInputCurve, a: number, b: number) {
	const rows = grid.values.map((row) => along(grid.columns, row, wraps(x), a));
	return Math.min(1, Math.max(0, along(grid.rows, rows, wraps(x2), b)));
}

/** Insert a column at `a` without changing the surface: its values come from the grid there. */
export function insertColumn(
	curve: TwoInputCurve,
	a: number
): { curve: TwoInputCurve; index: number } {
	const { grid } = curve;
	const after = grid.columns.findIndex((position) => position > a);
	const index = after < 0 ? grid.columns.length : after;
	const splice = <T>(list: readonly T[], item: T) => [
		...list.slice(0, index),
		item,
		...list.slice(index)
	];
	const values = grid.values.map((row, r) => splice(row, evaluateGrid(curve, a, grid.rows[r]!)));
	return {
		curve: { ...curve, grid: { ...grid, columns: splice(grid.columns, a), values } },
		index
	};
}

/** Remove a column, keeping at least two. */
export function removeColumn(curve: TwoInputCurve, index: number): TwoInputCurve {
	const { grid } = curve;
	if (grid.columns.length <= 2) return curve;
	const drop = <T>(list: readonly T[]) => list.filter((_, other) => other !== index);
	return {
		...curve,
		grid: { ...grid, columns: drop(grid.columns), values: grid.values.map(drop) }
	};
}

/** Set one grid value. */
export function setGridValue(
	curve: TwoInputCurve,
	row: number,
	column: number,
	value: number
): TwoInputCurve {
	const values = curve.grid.values.map((cells, r) =>
		r === row ? cells.map((cell, c) => (c === column ? value : cell)) : cells
	);
	return { ...curve, grid: { ...curve.grid, values } };
}
