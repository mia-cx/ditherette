import type { CurvesEffect } from 'ditherette';
import { beforeEach, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { EFFECTS } from '$lib/effects/catalog';
import { curvePicker } from '$lib/stores/curve-pick';
import CurvesEditor from './CurvesEditor.svelte';

let step: CurvesEffect;
const onchange = (next: CurvesEffect) => (step = next);

beforeEach(() => {
	step = EFFECTS.curves.create();
	curvePicker.set(undefined);
});

/** Render, and re-render after each change like the effect window does. */
async function editor() {
	const screen = await render(CurvesEditor, {
		id: 'curves',
		step,
		onchange: (next: CurvesEffect) => {
			onchange(next);
			void screen.rerender({ step });
		}
	});
	return screen;
}

it('adds, selects, and reorders curves, which apply in list order', async () => {
	await editor();
	await page.getByRole('button', { name: 'Curve', exact: true }).click();
	expect(step.curves.map(({ kind }) => kind)).toEqual(['remap', 'adjust']);
	await expect
		.element(page.getByRole('button', { name: 'Hue vs Chroma (OKLCH)' }))
		.toHaveAttribute('aria-pressed', 'true');

	await page.getByRole('button', { name: 'Move curve earlier' }).click();
	expect(step.curves.map(({ kind }) => kind)).toEqual(['adjust', 'remap']);
	await expect.element(page.getByRole('button', { name: 'Move curve earlier' })).toBeDisabled();
});

it('turns a remap into an adjustment when it changes another channel', async () => {
	await editor();
	await page.getByLabelText('Changes').click();
	await page.getByRole('option', { name: 'OKLCH · Chroma' }).click();
	expect(step.curves[0]).toMatchObject({
		kind: 'adjust',
		y: { model: 'oklch', channel: 'chroma' }
	});
	await expect.element(page.getByRole('radio', { name: 'Remap' })).toBeDisabled();
});

it('picks a point from the preview and pushes it with a vertical drag', async () => {
	await editor();
	await page.getByRole('button', { name: 'Pick from preview' }).click();
	const picker = curvePicker.get();
	expect(picker).toBeDefined();

	picker!.pick([119, 119, 119]);
	const [, picked] = step.curves[0]!.points;
	expect(picked![0]).toBeGreaterThan(0.4);
	expect(picked![0]).toBeLessThan(0.6);

	picker!.push(40);
	expect(step.curves[0]!.points[1]![1]).toBeCloseTo(picked![1] + 0.2, 1);

	await page.getByRole('button', { name: 'Pick from preview' }).click();
	expect(curvePicker.get()).toBeUndefined();
});
