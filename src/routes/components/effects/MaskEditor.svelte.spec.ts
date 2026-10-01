import { isEffect, type MaskCurve } from 'ditherette';
import { beforeEach, expect, it } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import MaskEditor from './MaskEditor.svelte';

let mask: MaskCurve[];
let shown: boolean;

beforeEach(async () => {
	await page.viewport(1440, 900);
	mask = [];
	shown = false;
});

/** Render, and re-render after each change like the effect window does. */
async function editor() {
	const screen = await render(MaskEditor, {
		id: 'mask',
		mask,
		onchange: (next: MaskCurve[]) => {
			mask = next;
			void screen.rerender({ mask });
		},
		shown,
		onshow: (next: boolean) => {
			shown = next;
			void screen.rerender({ shown });
		}
	});
	return screen;
}

const valid = () => isEffect({ effect: 'exposure', enabled: true, stops: 1, mask });

it('sets a preset curve from its sliders and drops it back at 100%', async () => {
	await editor();
	const shadows = page.getByRole('spinbutton', { name: 'Shadows' });
	await shadows.fill('40');
	await userEvent.keyboard('{Tab}');
	expect(mask).toHaveLength(1);
	expect(mask[0]!.x).toEqual({ model: 'oklch', channel: 'lightness' });
	expect(valid()).toBe(true);
	await expect.element(shadows).toHaveValue(40);

	await page.getByRole('radio', { name: 'Colours' }).click();
	const reds = page.getByRole('spinbutton', { name: 'Reds' });
	await reds.fill('0');
	await userEvent.keyboard('{Tab}');
	expect(mask).toHaveLength(2);
	expect(valid()).toBe(true);

	await reds.fill('100');
	await userEvent.keyboard('{Tab}');
	expect(mask).toHaveLength(1);
});

it('toggles showing the mask', async () => {
	await editor();
	const toggle = page.getByRole('button', { name: 'Show mask' });
	await toggle.click();
	expect(shown).toBe(true);
	await expect.element(toggle).toHaveAttribute('aria-pressed', 'true');
	await toggle.click();
	expect(shown).toBe(false);
});

it('adds, edits, and removes mask curves, including a second input', async () => {
	await editor();
	await page.getByRole('radio', { name: 'Curves' }).click();
	await page.getByRole('button', { name: 'Curve', exact: true }).click();
	expect(mask).toEqual([
		{
			x: { model: 'oklch', channel: 'lightness' },
			points: [
				[0, 1],
				[1, 1]
			]
		}
	]);
	await expect
		.element(page.getByRole('button', { name: 'Tones' }))
		.toHaveAttribute('aria-pressed', 'true');

	// Full-strength points sit on the graph's top edge, half outside it, so focus rather than click.
	(page.getByRole('button', { name: /^Point 2:/ }).element() as SVGElement).focus();
	await userEvent.keyboard('{Shift>}{ArrowDown}{/Shift}');
	expect(mask[0]!.points![1]![1]).toBeCloseTo(239 / 255, 6);

	await page.getByLabelText('And', { exact: true }).click();
	await page.getByRole('option', { name: 'OKLCH · Hue' }).click();
	expect(mask[0]).toMatchObject({ x2: { model: 'oklch', channel: 'hue' } });
	expect(valid()).toBe(true);
	await expect.element(page.getByLabelText('Strength', { exact: true })).toHaveValue(255);

	await page.getByRole('button', { name: 'Remove curve' }).click();
	expect(mask).toEqual([]);
});
