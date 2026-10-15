import { describe, expect, it } from 'vitest';
import { render } from 'svelte/server';
import { GUIDES, guidePath } from '$lib/guides';
import { canonicalUrl } from '$lib/seo';
import Hub from './+page.svelte';
import WplaceTemplate from './wplace-template/+page.svelte';
import WplaceColours from './wplace-colours/+page.svelte';
import Dithering from './dithering/+page.svelte';
import ColourQuantisation from './colour-quantisation/+page.svelte';

const pages = [
	{ path: '/guides', component: Hub },
	{ path: '/guides/wplace-template', component: WplaceTemplate },
	{ path: '/guides/wplace-colours', component: WplaceColours },
	{ path: '/guides/dithering', component: Dithering },
	{ path: '/guides/colour-quantisation', component: ColourQuantisation }
] as const;

function metaContent(head: string, key: string, attr = 'name') {
	return head.match(new RegExp(`<meta ${attr}="${key}" content="([^"]*)"`))?.[1];
}

describe('guide pages', () => {
	it('each page emits exactly one canonical matching og:url and a description', () => {
		for (const { path, component } of pages) {
			const { head } = render(component);
			const canonicals = head.match(/<link rel="canonical"/g) ?? [];
			expect(canonicals, path).toHaveLength(1);
			const canonical = canonicalUrl(path);
			expect(head, path).toContain(`<link rel="canonical" href="${canonical}"`);
			expect(metaContent(head, 'og:url', 'property')).toBe(canonical);
			expect(metaContent(head, 'description')?.length, path).toBeGreaterThan(0);
		}
	});

	it('titles and descriptions are unique across the hub and guides', () => {
		const titles = new Set<string>();
		const descriptions = new Set<string>();
		for (const { component } of pages) {
			const { head } = render(component);
			const title = head.match(/<title>([^<]*)<\/title>/)?.[1];
			const description = metaContent(head, 'description');
			expect(title).toBeTruthy();
			expect(description).toBeTruthy();
			expect(titles.has(title!)).toBe(false);
			expect(descriptions.has(description!)).toBe(false);
			titles.add(title!);
			descriptions.add(description!);
		}
	});

	it('each page has one h1 and a link back to the editor', () => {
		for (const { path, component } of pages) {
			const { body } = render(component);
			expect(body.match(/<h1/g)?.length, path).toBe(1);
			expect(body, path).toContain('href="/"');
		}
	});

	it('each guide links to the other three', () => {
		for (const guide of GUIDES) {
			const entry = pages.find((p) => p.path === guidePath(guide.slug))!;
			const { body } = render(entry.component);
			for (const other of GUIDES.filter((g) => g.slug !== guide.slug)) {
				expect(body, `${guide.slug} -> ${other.slug}`).toContain(`href="${guidePath(other.slug)}"`);
			}
		}
	});
});
