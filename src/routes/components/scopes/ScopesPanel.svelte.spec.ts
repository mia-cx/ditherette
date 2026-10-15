import { beforeEach, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { compiledEffects, currentEffectsKey, effectsIndex } from '$lib/processing/live-effects';
import { sourceImageData, uiSettings } from '$lib/stores/app';
import { addEffect, effectLayers, updateEffect } from '$lib/stores/effects';
import ScopesPanel from './ScopesPanel.svelte';

const image = (width: number, height: number, pixels: number[][]) => {
	const data = new Uint8ClampedArray(width * height * 4);
	pixels.flat().forEach((value, index) => (data[index] = value));
	return new ImageData(data, width, height);
};

const blackWhite = image(4, 1, [
	[0, 0, 0, 255],
	[255, 255, 255, 255],
	[128, 128, 128, 255],
	[0, 0, 0, 0]
]);

const pressed = (name: string) => {
	const element = page.getByRole('button', { name, exact: true }).element();
	return String(element.getAttribute('aria-pressed') ?? element.dataset.state === 'on');
};

beforeEach(() => {
	effectLayers.set([]);
	effectsIndex.set(undefined);
	compiledEffects.set(undefined);
	sourceImageData.set(undefined);
	uiSettings.set({
		...uiSettings.get(),
		scopes: { scope: 'histogram', model: 'srgb', source: true, output: true, vectorZoom: 1 }
	});
});

it('loads old saved settings: Source and Output pressed, Effects not', async () => {
	sourceImageData.set(blackWhite);
	render(ScopesPanel);
	await expect.poll(() => pressed('Source')).toBe('true');
	await expect.poll(() => pressed('Output')).toBe('true');
	await expect.poll(() => pressed('Effects')).toBe('false');
});

it('turning Effects on with both shown drops the least recently turned on', async () => {
	sourceImageData.set(blackWhite);
	render(ScopesPanel);
	await expect.poll(() => pressed('Source')).toBe('true');
	await page.getByRole('button', { name: 'Effects', exact: true }).click();
	// Source was the earlier of the saved two, so it drops.
	await expect.poll(() => pressed('Effects')).toBe('true');
	await expect.poll(() => pressed('Output')).toBe('true');
	await expect.poll(() => pressed('Source')).toBe('false');
});

it('shows the Effects stage clipping shares with no effect on', async () => {
	sourceImageData.set(blackWhite);
	uiSettings.set({
		...uiSettings.get(),
		scopes: { ...uiSettings.get().scopes!, effects: true, source: false, output: false }
	});
	render(ScopesPanel);
	await expect.element(page.getByText('Effects clipping')).toBeInTheDocument();
	await expect.element(page.getByText('Black 33%')).toBeInTheDocument();
	await expect.element(page.getByText('White 33%')).toBeInTheDocument();
});

it('dims and hides values for stale results until the current key arrives', async () => {
	sourceImageData.set(blackWhite);
	uiSettings.set({
		...uiSettings.get(),
		scopes: { ...uiSettings.get().scopes!, effects: true, source: false, output: false }
	});
	const layer = addEffect('exposure');
	const source = sourceImageData.get()!;
	const pixels = new Uint32Array(4).fill((255 << 24) | 0);
	effectsIndex.set({ source, pixels, count: 1 });
	// Results for an older settings key: a stand-in, never current.
	compiledEffects.set({ source, key: 'stale', results: new Uint32Array([0xffffff]) });
	render(ScopesPanel);
	await expect.element(page.getByText('Updating…').first()).toBeInTheDocument();
	await expect.element(page.getByText('Black —').first()).toBeInTheDocument();
	// The current key's results arrive: values replace the dashes.
	compiledEffects.set({ source, key: currentEffectsKey(), results: new Uint32Array([0xffffff]) });
	await expect.element(page.getByText('Effects clipping')).toBeInTheDocument();
	expect(page.getByText('Updating…').elements()).toHaveLength(0);
	await expect.element(page.getByText('White 100%').first()).toBeInTheDocument();
	updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 0 });
});
