import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { expect, test } from '@playwright/test';

test.use({ viewport: { width: 900, height: 900 } });

test('exports an indexed PNG and restores the workbench after reload', async ({ page }) => {
	await page.goto('/');

	const dither = page.getByRole('region', { name: 'Dithering controls' });
	const algorithm = dither.getByRole('combobox');
	const strength = dither.getByRole('spinbutton', { name: 'Strength' });
	await expect(algorithm).toBeVisible();

	const [chooser] = await Promise.all([
		page.waitForEvent('filechooser'),
		page.getByRole('button', { name: 'Choose file' }).click()
	]);
	await chooser.setFiles(
		fileURLToPath(new URL('../src/lib/assets/guides/source.png', import.meta.url))
	);
	const source = page.getByRole('img', { name: 'Uploaded source' });
	await expect(source).toHaveJSProperty('naturalWidth', 384);

	await algorithm.click();
	await page.getByRole('option', { name: /^Bayer 2×2/ }).click();
	await strength.fill('50');

	const downloadButton = page.getByRole('button', { name: 'Download PNG' });
	const downloaded = page.waitForEvent('download');
	await downloadButton.click();
	const download = await downloaded;
	expect(download.suggestedFilename()).toMatch(/\.png$/);

	const png = await readFile(await download.path());
	expect(png.subarray(0, 8).toString('hex')).toBe('89504e470d0a1a0a');
	expect(png.toString('ascii', 12, 16)).toBe('IHDR');
	expect(png[25]).toBe(3);

	let paletteLength = 0;
	for (let offset = 8; offset + 12 <= png.length; ) {
		const length = png.readUInt32BE(offset);
		const end = offset + 12 + length;
		expect(end).toBeLessThanOrEqual(png.length);
		const type = png.toString('ascii', offset + 4, offset + 8);
		if (type === 'PLTE') paletteLength = length;
		if (type === 'IDAT') break;
		offset = end;
	}
	expect(paletteLength).toBeGreaterThan(0);
	expect(paletteLength % 3).toBe(0);

	await page.reload();
	await expect(source).toHaveJSProperty('naturalWidth', 384);
	await expect(algorithm).toContainText('Bayer 2×2');
	await expect(strength).toHaveValue('50');
	await expect(downloadButton).toBeEnabled();
});
