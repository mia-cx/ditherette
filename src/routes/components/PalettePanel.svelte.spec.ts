import { beforeEach, expect, it } from 'vitest';
import { page, type Locator } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import {
	activePalette,
	colorSpace,
	ditherSettings,
	outputSettings,
	processedImage,
	processingProgress,
	selectedPalette,
	sourceMeta
} from '$lib/stores/app';
import { activeEffectSteps, effectLayers } from '$lib/stores/effects';
import { processingIdentityHash } from '$lib/processing/hash';
import type { ProcessedImage } from '$lib/processing/types';
import PalettePanel from './PalettePanel.svelte';
import '../layout.css';

function exportableImage(indices: number[] | Uint8Array): ProcessedImage {
	const settingsHash = processingIdentityHash({
		output: outputSettings.get(),
		dither: ditherSettings.get(),
		colorSpace: colorSpace.get(),
		effects: activeEffectSteps.get(),
		paletteName: activePalette.get().name,
		paletteSource: activePalette.get().source,
		palette: selectedPalette.get(),
		source: sourceMeta.get()
	});
	const palette = selectedPalette.get();
	return {
		width: indices.length,
		height: 1,
		indices: indices instanceof Uint8Array ? indices : new Uint8Array(indices),
		palette,
		transparentIndex: palette.findIndex((color) => color.kind === 'transparent'),
		warnings: [],
		settingsHash,
		updatedAt: 0
	};
}

beforeEach(() => {
	effectLayers.set([]);
	sourceMeta.set({ name: 'test.png', width: 4, height: 1, type: 'image/png', updatedAt: 1 });
	processingProgress.set(undefined);
	const transparent = selectedPalette.get().findIndex((color) => color.kind === 'transparent');
	processedImage.set(exportableImage([0, 0, 0, transparent]));
});

const pixelText = (colourName: string) =>
	[...document.querySelectorAll('section[aria-label="Palette editor"] tr')]
		.find((tr) => tr.textContent?.includes(colourName))
		?.querySelector('td.tabular-nums')
		?.textContent?.trim();

it('shows per-colour pixel counts and a dash while the output is out of date', async () => {
	render(PalettePanel);
	await expect
		.element(page.getByText('Pixels', { exact: true }), { timeout: 10_000 })
		.toBeInTheDocument();

	// The first Wplace colour covers 3 of the 4 output pixels; Transparent covers the last.
	const first = selectedPalette.get()[0]!;
	await expect.poll(() => pixelText(first.name)).toBe('3');
	await expect.poll(() => pixelText('Transparent')).toBe('1');
	const other = selectedPalette
		.get()
		.find((color) => color.name !== first.name && color.kind !== 'transparent')!;
	await expect.poll(() => pixelText(other.name)).toBe('0');

	processingProgress.set({ stage: 'Matching', progress: 0.5 });
	await expect.poll(() => pixelText(first.name)).toBe('—');
	await expect.poll(() => pixelText('Transparent')).toBe('—');
	await expect.poll(() => pixelText(other.name)).toBe('—');
});

const viewportRect = () =>
	document
		.querySelector('section[aria-label="Palette editor"] [data-slot="scroll-area-viewport"]')!
		.getBoundingClientRect();

/** Element must render and sit horizontally inside the palette scroll viewport. */
async function expectInside(locator: Locator, what: string) {
	await expect.element(locator, { timeout: 10_000 }).toBeVisible();
	const element = locator.element() as HTMLElement;
	const rect = element.getBoundingClientRect();
	const view = viewportRect();
	expect(rect.width, what).toBeGreaterThan(0);
	expect(rect.left, what).toBeGreaterThanOrEqual(view.left - 0.5);
	expect(rect.right, what).toBeLessThanOrEqual(view.right + 0.5);
}

/** Some content renders twice (compact + wide); at least one match must be laid out inside. */
async function expectAnyInside(locator: Locator, what: string) {
	const matches = locator.all();
	expect(matches.length, what).toBeGreaterThan(0);
	const view = viewportRect();
	const inside = matches.some((match) => {
		const rect = (match.element() as HTMLElement).getBoundingClientRect();
		return rect.width > 0 && rect.left >= view.left - 0.5 && rect.right <= view.right + 0.5;
	});
	expect(inside, what).toBe(true);
}

it.each([276, 358, 376, 600])(
	'keeps counts, tag buttons and row controls reachable at %ipx',
	async (width) => {
		const palette = selectedPalette.get();
		const transparent = palette.findIndex((color) => color.kind === 'transparent');
		const indices = new Uint8Array(12_345_679);
		indices[12_345_678] = transparent;
		processedImage.set(exportableImage(indices));

		const host = document.createElement('div');
		host.style.width = `${width}px`;
		host.style.height = '700px';
		document.body.appendChild(host);
		try {
			render(PalettePanel, { fillHeight: true }, { baseElement: host });
			await expect.element(page.getByRole('table'), { timeout: 10_000 }).toBeInTheDocument();

			const black = palette[0]!;
			const silver = palette.find((color) => color.kind === 'premium')!;
			const premiumCount = palette.filter((color) => color.kind === 'premium').length;
			const rows = [
				{ name: black.name, pixels: '12,345,678', tag: 'Select all Free colours', hex: black.key },
				{
					name: silver.name,
					pixels: '0',
					tag: 'Select all Premium colours',
					hex: silver.key
				},
				{
					name: 'Transparent',
					pixels: '1',
					tag: 'Select all Transparent colours',
					hex: undefined
				}
			];
			for (const { name, pixels, tag, hex } of rows) {
				const row = page.getByRole('row').filter({ hasText: name });
				await expectInside(row.getByRole('checkbox', { name: `Select ${name}` }), name);
				await expectInside(
					row.getByRole('checkbox', { name: `Visible: ${name}` }),
					`${name} visibility`
				);
				await expectInside(row.getByText(pixels, { exact: true }), `${name} pixels`);
				await expectInside(row.getByRole('button', { name: tag }), `${name} tag`);
				await expectInside(row.getByRole('button', { name: 'Edit' }), `${name} edit`);
				if (hex) await expectAnyInside(row.getByText(hex, { exact: true }), `${name} hex`);
			}

			await expectInside(
				page.getByRole('button', { name: 'Duplicate selected colours' }),
				'selection duplicate'
			);
			await expectInside(
				page.getByRole('button', { name: 'Delete selected colours' }),
				'selection delete'
			);
			if (width === 600) {
				const blackRow = page.getByRole('row').filter({ hasText: black.name });
				await expectInside(
					blackRow.getByRole('button', { name: `Duplicate ${black.name}` }),
					'row duplicate'
				);
				await expectInside(
					blackRow.getByRole('button', { name: `Delete ${black.name}` }),
					'row delete'
				);
			}

			// The tag button must work, not just render: selecting Premium marks that many rows.
			const silverRow = page.getByRole('row').filter({ hasText: silver.name });
			await silverRow.getByRole('button', { name: 'Select all Premium colours' }).click();
			await expect
				.element(page.getByText(`${premiumCount} selected`, { exact: true }))
				.toBeVisible();
		} finally {
			host.remove();
		}
	}
);
