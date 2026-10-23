import { afterEach, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { outputPyramidBitmaps } from '$lib/processing/output-preview';
import type { ProcessedImage } from '$lib/processing/types';
import {
	outputSettings,
	previewSettings,
	processedImage,
	setOutputPreview,
	sourceImageData,
	sourceMeta,
	sourceObjectUrl
} from '$lib/stores/app';
import ComparisonPreview from './ComparisonPreview.svelte';

afterEach(() => {
	processedImage.set(undefined);
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
});

it.each([
	{ side: 4096, dpr: 1, mode: 'side-by-side', transparent: false },
	{ side: 4096, dpr: 2, mode: 'ab-reveal', transparent: false },
	{ side: 4097, dpr: 2, mode: 'side-by-side', transparent: true }
] as const)(
	'prefilters final $side-pixel output at 1800 device pixels, DPR $dpr, $mode',
	async ({ side, dpr, mode, transparent }) => {
		const target = 1800;
		vi.stubGlobal('devicePixelRatio', dpr);
		const indices = new Uint8Array(side * side);
		for (let y = 0; y < side; y++)
			for (let x = 0; x < side; x++) indices[y * side + x] = (x + y) % 2;
		const image: ProcessedImage = {
			width: side,
			height: side,
			indices,
			palette: [
				{
					name: 'Ink',
					key: transparent ? '#ff0000' : '#000000',
					rgb: { r: transparent ? 255 : 0, g: 0, b: 0 },
					kind: 'free',
					enabled: true
				},
				transparent
					? { name: 'Clear', key: 'transparent', kind: 'transparent', enabled: true }
					: {
							name: 'White',
							key: '#ffffff',
							rgb: { r: 255, g: 255, b: 255 },
							kind: 'free',
							enabled: true
						}
			],
			transparentIndex: transparent ? 1 : -1,
			warnings: [],
			settingsHash: 'consumer-rendering',
			updatedAt: 1
		};
		const bitmaps = outputPyramidBitmaps(image)!;
		const levels = structuredClone(bitmaps, { transfer: bitmaps });
		const allocations: { canvas: OffscreenCanvas; bytes: number }[] = [];
		const getContext = OffscreenCanvas.prototype.getContext;
		vi.spyOn(OffscreenCanvas.prototype, 'getContext').mockImplementation(function (
			this: OffscreenCanvas,
			...args
		) {
			allocations.push({ canvas: this, bytes: this.width * this.height * 4 });
			return getContext.apply(this, args);
		});
		setOutputPreview({ image, levels });
		processedImage.set(image);
		sourceImageData.set(undefined);
		sourceObjectUrl.set(undefined);
		sourceMeta.set({
			name: 'checker.png',
			width: side,
			height: side,
			type: 'image/png',
			updatedAt: 1
		});
		outputSettings.set({ ...outputSettings.get(), crop: undefined });
		previewSettings.set({
			mode,
			sourceEffects: false,
			zoom: target / side,
			frameWidth: target / dpr,
			frameHeight: target / dpr,
			originX: 0,
			originY: 0
		});
		render(ComparisonPreview, { hasImage: true });
		const canvas = () =>
			document.querySelector<HTMLCanvasElement>('canvas[aria-label="Processed dithered output"]');
		await expect.poll(() => canvas()?.width).toBe(target);
		const output = canvas()!;
		expect(output.style.width).toBe(`${target / dpr}px`);
		const context = output.getContext('2d')!;
		if (transparent) {
			const pixels = context.getImageData(0, 0, target, target).data;
			const hash = Array.from(
				new Uint8Array(await crypto.subtle.digest('SHA-256', pixels)),
				(byte) => byte.toString(16).padStart(2, '0')
			).join('');
			// Captured from the unmodified b89747d6 component and complete pyramid in Chromium.
			expect(hash).toBe('fadf49ad10483ac3eb197d91261715a06d235623c3260560de5d3129db322841');
			const paddedBytes = 4098 * 4098 * 4;
			const halfBytes = 2049 * 2049 * 4;
			expect(allocations.map(({ bytes }) => bytes)).toEqual([paddedBytes, halfBytes]);
		} else {
			const row = context.getImageData(0, 0, target, 1).data;
			// The old 2048 mip averages each opaque black/white 2x2 block to native Chromium 127.
			expect(new Set(row.filter((_, index) => index % 4 === 0))).toEqual(new Set([127]));
			expect(row.filter((_, index) => index % 4 === 3).every((alpha) => alpha === 255)).toBe(true);
			expect(allocations.map(({ bytes }) => bytes)).toEqual([16 * 1024 * 1024]);
		}
		expect(allocations.every(({ canvas }) => canvas.width === 0 && canvas.height === 0)).toBe(true);
		expect(
			levels.slice(1).reduce((sum, level) => sum + level.width * level.height * 4, 0)
		).toBeLessThanOrEqual(16 * 1024 * 1024);
		const allocationCount = allocations.length;
		previewSettings.set({ ...previewSettings.get(), panX: 1 });
		await expect.poll(() => canvas()?.style.left).toBe('1px');
		await new Promise(requestAnimationFrame);
		expect(allocations).toHaveLength(allocationCount);
		previewSettings.set({ ...previewSettings.get(), zoom: 1 });
		await expect.poll(() => canvas()?.width).toBe(side);
		expect(Array.from(context.getImageData(0, 0, 2, 1).data)).toEqual(
			transparent ? [255, 0, 0, 255, 0, 0, 0, 0] : [0, 0, 0, 255, 255, 255, 255, 255]
		);
		expect(output.style.width).toBe(`${side / dpr}px`);
		expect(output.style.imageRendering).toBe('pixelated');
		expect(allocations).toHaveLength(allocationCount);
		expect(image.indices.byteLength).toBe(side * side);
	}
);
