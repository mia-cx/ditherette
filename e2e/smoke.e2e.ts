import { expect, test } from '@playwright/test';

test('home page loads with workbench skeleton', async ({ page }) => {
	await page.goto('/');
	// AppBar wordmark
	await expect(page.getByText('ditherette', { exact: true })).toBeVisible();
	// Initial state: no image loaded, so the preview offers a file picker.
	await expect(
		page.getByRole('button', { name: 'Choose file' }).filter({ visible: true }).first()
	).toBeVisible();
});
