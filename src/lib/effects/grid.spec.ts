import type { TwoInputCurve } from 'ditherette';
import { describe, expect, it } from 'vitest';
import { evaluateGrid, insertColumn, neutralGrid, removeColumn, setGridValue } from './grid';

const hue = { model: 'oklch', channel: 'hue' } as const;
const lightness = { model: 'oklch', channel: 'lightness' } as const;
const chroma = { model: 'oklch', channel: 'chroma' } as const;

function redShadows(): TwoInputCurve {
	let curve: TwoInputCurve = {
		kind: 'adjust',
		x: hue,
		x2: lightness,
		y: chroma,
		grid: neutralGrid(hue, lightness)
	};
	curve = setGridValue(curve, 0, 1, 0.95);
	curve = setGridValue(curve, 1, 1, 0.9);
	return setGridValue(curve, 1, 2, 0.7);
}

describe('two-input curve grids', () => {
	it('starts neutral, a column every 30° on hue and five rows', () => {
		const { grid } = redShadows();
		expect(grid.columns).toHaveLength(12);
		expect(grid.rows).toEqual([0, 0.25, 0.5, 0.75, 1]);
		expect(evaluateGrid({ ...redShadows(), grid: neutralGrid(hue, lightness) }, 0.3, 0.6)).toBe(
			0.5
		);
	});

	it('passes through its points and wraps across the hue seam', () => {
		const curve = redShadows();
		expect(evaluateGrid(curve, curve.grid.columns[1]!, 0)).toBeCloseTo(0.95, 6);
		expect(evaluateGrid(curve, 0.999999, 0.3)).toBeCloseTo(evaluateGrid(curve, 0, 0.3), 4);
	});

	it('inserts a column without changing the surface', () => {
		const curve = redShadows();
		const { curve: inserted, index } = insertColumn(curve, 0.11);
		expect(inserted.grid.columns[index]).toBe(0.11);
		for (const [a, b] of [
			[0.05, 0.1],
			[0.11, 0.3],
			[0.2, 0.45],
			[0.93, 0.8]
		] as const)
			expect(evaluateGrid(inserted, a, b)).toBeCloseTo(evaluateGrid(curve, a, b), 2);
		expect(removeColumn(inserted, index).grid.columns).toEqual(curve.grid.columns);
	});
});
