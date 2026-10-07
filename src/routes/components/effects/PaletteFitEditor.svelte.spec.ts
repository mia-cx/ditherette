import type { Curve, PaletteFitEffect } from 'ditherette';
import { beforeEach, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { EFFECTS } from '$lib/effects/catalog';
import { analysedFits } from '$lib/processing/live-effects';
import { effectLayers } from '$lib/stores/effects';
import { fitReverts } from '$lib/stores/fit-revert';
import PaletteFitEditor from './PaletteFitEditor.svelte';

const analysed: Curve[] = [
	{
		kind: 'remap',
		x: { model: 'oklch', channel: 'lightness' },
		y: { model: 'oklch', channel: 'lightness' },
		points: [
			[0, 0],
			[0.5, 0.62],
			[1, 1]
		]
	},
	{
		kind: 'adjust',
		x: { model: 'oklch', channel: 'lightness' },
		y: { model: 'oklch', channel: 'chroma' },
		points: [
			[0, 0.8],
			[1, 0.8]
		]
	}
];

let step: PaletteFitEffect;
let emitted: PaletteFitEffect[];

const openAdvanced = () => page.getByLabelText('Advanced').click();

beforeEach(async () => {
	await page.viewport(1440, 900);
	step = EFFECTS['palette-fit'].create();
	emitted = [];
	analysedFits.set(new Map([['fit-1', analysed]]));
	fitReverts.set(new Map());
	effectLayers.set([{ id: 'fit-1', name: 'Palette fit', step: EFFECTS['palette-fit'].create() }]);
});

async function editor() {
	const screen = await render(PaletteFitEditor, {
		id: 'fit',
		layerId: 'fit-1',
		step,
		onchange: (next: PaletteFitEffect) => {
			emitted.push(next);
			void screen.rerender({ step: next });
		}
	});
	return screen;
}

it('shows the analysed curves in Advanced and locks the fit on the first edit', async () => {
	await editor();
	await expect.element(page.getByText('Edited')).not.toBeInTheDocument();
	await openAdvanced();
	// The analysis wrote two curves; the editor lists them by name.
	await expect
		.element(page.getByRole('button', { name: 'Lightness vs Chroma (OKLCH)' }))
		.toBeInTheDocument();

	await page.getByRole('button', { name: 'Curve', exact: true }).click();
	const next = emitted.at(-1)!;
	expect(next.curves).toHaveLength(3);
	expect(next.curves?.slice(0, 2)).toEqual(analysed);
	await expect.element(page.getByText('Edited')).toBeInTheDocument();
});

it('Reset clears the edited curves back to automatic', async () => {
	step = { ...step, curves: [...analysed] };
	await editor();
	await expect.element(page.getByText('Edited')).toBeInTheDocument();
	await page.getByRole('button', { name: 'Reset' }).click();
	expect(emitted.at(-1)!.curves).toBeNull();
});

it('asks once before changing space on an edited fit, and cancels', async () => {
	step = { ...step, curves: [...analysed] };
	await editor();
	await page.getByLabelText('Space').click();
	await page.getByRole('option', { name: 'CIELab' }).click();
	await expect.element(page.getByText('Re-analyse in CIELab?')).toBeInTheDocument();
	await expect.element(page.getByText('Your curve edits will be lost.')).toBeInTheDocument();

	await page.getByRole('button', { name: 'Cancel' }).click();
	expect(emitted).toHaveLength(0);

	await page.getByLabelText('Space').click();
	await page.getByRole('option', { name: 'CIELab' }).click();
	await page.getByRole('button', { name: 'Re-analyse' }).click();
	expect(emitted.at(-1)).toMatchObject({ space: 'cielab', curves: null });
});

it('switches space without asking while unedited', async () => {
	await editor();
	await page.getByLabelText('Space').click();
	await page.getByRole('option', { name: 'CIELab' }).click();
	await expect.element(page.getByText('Re-analyse in CIELab?')).not.toBeInTheDocument();
	expect(emitted.at(-1)).toMatchObject({ space: 'cielab', curves: null });
});

it('offers Revert when the fit was re-analysed for new inputs', async () => {
	await editor();
	fitReverts.set(
		new Map([
			[
				'fit-1',
				{
					inputs: {
						paletteName: 'Wplace (Default)',
						enabled: {},
						values: {},
						crop: undefined,
						before: []
					},
					curves: analysed
				}
			]
		])
	);
	await expect.element(page.getByText('Re-analysed for the new inputs.')).toBeInTheDocument();
	await page.getByRole('button', { name: 'Revert' }).click();
	const step = effectLayers.get()[0]!.step;
	expect(step.effect === 'palette-fit' && step.curves).toEqual(analysed);
});
