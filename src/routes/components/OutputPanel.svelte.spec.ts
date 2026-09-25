import { beforeEach, expect, it } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { outputSettings, sourceMeta } from '$lib/stores/app';
import OutputPanel from './OutputPanel.svelte';

beforeEach(() => {
	sourceMeta.set({ name: 'test.png', width: 1000, height: 500, type: 'image/png', updatedAt: 1 });
	outputSettings.set({
		...outputSettings.get(),
		width: 1000,
		height: 500,
		scaleFactor: 1,
		crop: undefined
	});
});

async function enter(label: string, value: string) {
	const input = page.getByRole('spinbutton', { name: label, exact: true });
	await input.fill(value);
	await userEvent.keyboard('{Enter}');
}

it('upscales from scale and dimension inputs while keeping the slider in its original range', async () => {
	render(OutputPanel, { hasImage: true });
	await enter('Scale', '2');
	await expect.element(page.getByLabelText('Width', { exact: true })).toHaveValue(2000);
	await expect.element(page.getByLabelText('Height', { exact: true })).toHaveValue(1000);
	const slider = page.getByRole('slider');
	await expect.element(slider).toHaveAttribute('aria-valuemin', '0.05');
	await expect.element(slider).toHaveAttribute('aria-valuemax', '1');
	await expect.element(slider).toHaveAttribute('aria-valuenow', '1');
	await expect.element(page.getByLabelText('Scale', { exact: true })).toHaveValue(2);
	await enter('Width', '3000');
	await expect.element(page.getByLabelText('Scale', { exact: true })).toHaveValue(3);
	await enter('Height', '2000');
	await expect.element(page.getByLabelText('Width', { exact: true })).toHaveValue(4000);
	await expect.element(page.getByLabelText('Scale', { exact: true })).toHaveValue(4);
});

it('keeps tiny positive scales and lets the slider resume controlling the scale', async () => {
	render(OutputPanel, { hasImage: true });
	await enter('Scale', '0.000001');
	await expect.element(page.getByLabelText('Scale', { exact: true })).toHaveValue(0.000001);
	await expect.element(page.getByLabelText('Width', { exact: true })).toHaveValue(1);
	await expect.element(page.getByLabelText('Height', { exact: true })).toHaveValue(1);
	await expect.element(page.getByRole('slider')).toHaveAttribute('aria-valuenow', '0.05');
	(page.getByRole('slider').element() as HTMLElement).focus();
	await userEvent.keyboard('{End}');
	await expect.element(page.getByLabelText('Scale', { exact: true })).toHaveValue(1);
	await expect.element(page.getByLabelText('Width', { exact: true })).toHaveValue(1000);
});

it('restores the current scale after zero, negative, or empty input', async () => {
	render(OutputPanel, { hasImage: true });
	await enter('Scale', '2');
	for (const value of ['0', '-1', '']) {
		await enter('Scale', value);
		await expect.element(page.getByLabelText('Scale', { exact: true })).toHaveValue(2);
	}
	expect(outputSettings.get().scaleFactor).toBe(2);
});

it('caps huge scales at the pixel budget and fits wide crops within the side limit', async () => {
	render(OutputPanel, { hasImage: true });
	await enter('Scale', '1e308');
	await expect.poll(() => outputSettings.get().width).toBe(11584);
	expect(outputSettings.get().height).toBe(5792);
	expect(outputSettings.get().scaleFactor).toBeCloseTo(11.5852375);
	outputSettings.set({
		...outputSettings.get(),
		crop: { x: 0, y: 0, width: 1000, height: 100 }
	});
	await enter('Scale', '1e308');
	await expect.element(page.getByLabelText('Width', { exact: true })).toHaveValue(16384);
	await expect.element(page.getByLabelText('Height', { exact: true })).toHaveValue(1638);
	await expect.element(page.getByLabelText('Scale', { exact: true })).toHaveValue(16.384);
});
