import { beforeEach, expect, it } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { effectLayers } from '$lib/stores/effects';
import Studio from './Studio.svelte';
import { studioLayout } from './workspace';

const props = { onChooseImage: () => {}, onSelectFile: () => {} };
/** Dockview hides windows in a zero-size container, so give the studio a desktop-sized box. */
function renderStudio() {
	const target = document.body.appendChild(document.createElement('div'));
	target.style.cssText = 'width: 1280px; height: 800px';
	return render(Studio, { props, target });
}

const tabTitles = () =>
	[...document.querySelectorAll('.dv-tab')].map((tab) => tab.textContent?.trim());

beforeEach(async () => {
	await page.viewport(1440, 900);
	effectLayers.set([]);
	studioLayout.set(null);
});

/** Add through the keyboard, which the menu supports alongside the pointer. */
async function addEffect(label: string) {
	(page.getByRole('button', { name: 'Add effect' }).element() as HTMLElement).focus();
	await userEvent.keyboard('{Enter}');
	(page.getByRole('menuitem', { name: label }).element() as HTMLElement).focus();
	await userEvent.keyboard('{Enter}');
}

it('opens one window per effect instance and titles it with the instance name', async () => {
	await renderStudio();
	await addEffect('Levels');
	await addEffect('Levels');
	await expect.poll(tabTitles).toEqual(expect.arrayContaining(['Levels', 'Levels 2']));

	const name = page.getByRole('textbox', { name: 'Effect name' });
	await name.fill('Shadows');
	await userEvent.keyboard('{Enter}');
	await expect.poll(tabTitles).toEqual(expect.arrayContaining(['Levels', 'Shadows']));

	(page.getByRole('button', { name: 'Remove Shadows' }).element() as HTMLElement).focus();
	await userEvent.keyboard('{Enter}');
	await expect.poll(tabTitles).not.toContain('Shadows');
});

it('restores the latest layout, even when the studio closes before saving', async () => {
	const first = await renderStudio();
	await addEffect('Curves');
	await first.unmount();
	expect(JSON.stringify(studioLayout.get())).toContain('effect:');

	await renderStudio();
	await expect.poll(tabTitles).toEqual(expect.arrayContaining(['Preview', 'Effects', 'Curves']));
});
