import type { CurvesEffect } from 'ditherette';
import { beforeEach, expect, it } from 'vitest';
import { page, userEvent } from 'vitest/browser';
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

/** The first curve's points; it starts as a one-input curve. */
function firstPoints() {
	const [curve] = step.curves;
	if (!curve || curve.x2) throw new Error('Expected a one-input curve.');
	return curve.points;
}

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
	const [, picked] = firstPoints();
	expect(picked![0]).toBeGreaterThan(0.4);
	expect(picked![0]).toBeLessThan(0.6);
	await expect
		.element(page.getByRole('button', { name: /^Point 2:/ }))
		.toHaveAttribute('aria-pressed', 'true');

	picker!.push(40);
	expect(firstPoints()[1]![1]).toBeCloseTo(picked![1] + 0.2, 1);

	await page.getByRole('button', { name: 'Pick from preview' }).click();
	expect(curvePicker.get()).toBeUndefined();
});

it('adds a second input as a grid, edits it by keyboard, and picks on both axes', async () => {
	await editor();
	await page.getByRole('button', { name: 'Curve', exact: true }).click();
	await page.getByLabelText('And', { exact: true }).click();
	await page.getByRole('option', { name: 'OKLCH · Lightness' }).click();
	const [, curve] = step.curves;
	expect(curve).toMatchObject({ kind: 'adjust', x2: { model: 'oklch', channel: 'lightness' } });
	if (!curve?.x2) throw new Error('Expected a two-input curve.');
	expect(curve.grid.columns).toHaveLength(12);
	await expect.element(page.getByRole('radio', { name: 'Remap' })).toBeDisabled();

	const node = page.getByRole('button', { name: /OKLCH · Hue 30°, OKLCH · Lightness 64/ });
	await node.click();
	await userEvent.keyboard('{Shift>}{ArrowUp}{/Shift}');
	const edited = step.curves[1]!;
	if (!edited.x2) throw new Error('Expected a two-input curve.');
	// Neutral 0.5 sits on byte 128, so Shift+Up lands on 144.
	expect(edited.grid.values[1]![1]).toBeCloseTo(144 / 255, 6);

	await page.getByRole('button', { name: 'Pick from preview' }).click();
	curvePicker.get()!.pick([30, 10, 10]);
	curvePicker.get()!.push(100);
	const picked = step.curves[1]!;
	if (!picked.x2) throw new Error('Expected a two-input curve.');
	// Dark red lands on the same 30°, 25% point, and the drag pushes it to the top.
	expect(picked.grid.values[1]![1]).toBe(1);
});

it('expands into the analysis view and comes back with the same curve selected', async () => {
	await editor();
	await page.getByRole('button', { name: 'Curve', exact: true }).click();
	await page.getByRole('button', { name: 'Expand' }).click();
	const dialog = page.getByRole('dialog', { name: 'Curves' });
	await expect.element(dialog).toBeVisible();
	await expect
		.element(dialog.getByRole('button', { name: 'Hue vs Chroma (OKLCH)' }))
		.toHaveAttribute('aria-pressed', 'true');
	await dialog.getByRole('button', { name: 'Lightness (OKLCH)' }).click();
	await dialog.getByRole('button', { name: 'Back' }).click();
	await expect.element(dialog).not.toBeInTheDocument();
	await expect
		.element(page.getByRole('button', { name: 'Lightness (OKLCH)' }))
		.toHaveAttribute('aria-pressed', 'true');
});
