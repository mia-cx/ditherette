import { expect, test } from '@playwright/test';

test('home page loads with workbench skeleton', async ({ page }) => {
	await page.goto('/');
	// AppBar wordmark
	await expect(page.getByText('ditherette', { exact: true })).toBeVisible();
	// Initial state: no image loaded → upload primary button is visible
	await expect(page.getByRole('button', { name: 'Upload Image' })).toBeVisible();
});
