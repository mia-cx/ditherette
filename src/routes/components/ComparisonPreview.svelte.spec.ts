import { beforeEach, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { effectsDrawn } from '$lib/processing/effects-view.svelte';
import {
	compiledEffects,
	compiledMask,
	currentEffectsKey,
	effectsIndex,
	shownMask
} from '$lib/processing/live-effects';
import { previewSettings, sourceImageData, sourceMeta, sourceObjectUrl } from '$lib/stores/app';
import { addEffect, effectLayers, setEffectEnabled } from '$lib/stores/effects';
import ComparisonPreview from './ComparisonPreview.svelte';

const WIDTH = 4;
const HEIGHT = 3;
const PNG =
	'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAQAAAADCAYAAAC09K7GAAAADUlEQVR42mNk+M9QzwAEAAoBAf5/tXNPAAAAAElFTkSuQmCC';

function sourceCanvas() {
	return document.querySelector<HTMLCanvasElement>('canvas[aria-hidden="true"]');
}

type TexSpy = ReturnType<typeof vi.spyOn>;
function indexUploads(spy: TexSpy) {
	return spy.mock.calls.filter((args: Array<unknown>) => args[3] === WIDTH && args[4] === HEIGHT)
		.length;
}

beforeEach(() => {
	effectLayers.set([]);
	shownMask.set(undefined);
	effectsIndex.set(undefined);
	compiledEffects.set(undefined);
	compiledMask.set(undefined);
	effectsDrawn.set(false);
	previewSettings.set({ mode: 'side-by-side', sourceEffects: true });
	const data = new ImageData(WIDTH, HEIGHT);
	sourceImageData.set(data);
	sourceMeta.set({
		name: 'test.png',
		width: WIDTH,
		height: HEIGHT,
		type: 'image/png',
		updatedAt: 1
	});
	sourceObjectUrl.set(PNG);
});

it('keeps the Source canvas mounted while its index is loaded, hidden when nothing draws', async () => {
	expect(document.createElement('canvas').getContext('webgl2')).not.toBeNull();
	const spy = vi.spyOn(WebGL2RenderingContext.prototype, 'texImage2D');
	render(ComparisonPreview, { hasImage: true });
	const layer = addEffect('exposure');
	const source = sourceImageData.get()!;
	effectsIndex.set({ source, pixels: new Uint32Array(WIDTH * HEIGHT), count: 1 });
	compiledEffects.set({ source, key: currentEffectsKey(), results: new Uint32Array([0xff0000ff]) });

	await expect.element(page.getByText('Source with effects', { exact: true })).toBeVisible();
	await expect.poll(() => sourceCanvas()?.isConnected).toBe(true);
	const canvas = sourceCanvas()!;
	expect(canvas.hidden).toBe(false);
	await expect.poll(() => indexUploads(spy)).toBe(1);

	setEffectEnabled(layer.id, false);
	await expect.element(page.getByText('Source', { exact: true })).toBeVisible();
	expect(sourceCanvas()?.isConnected).toBe(true);
	expect(sourceCanvas()).toBe(canvas);
	expect(canvas.hidden).toBe(true);

	setEffectEnabled(layer.id, true);
	await expect.element(page.getByText('Source with effects', { exact: true })).toBeVisible();
	expect(sourceCanvas()).toBe(canvas);
	expect(canvas.hidden).toBe(false);
	await expect.poll(() => indexUploads(spy)).toBe(1);

	effectsIndex.set(undefined);
	await expect.poll(() => sourceCanvas()).toBeNull();
});
