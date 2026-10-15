import { readdirSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { GUIDES } from '$lib/guides';
import { canonicalUrl, jsonLdScript, SITEMAP_PATHS, sitemapXml } from '$lib/seo';

describe('canonicalUrl', () => {
	it('maps / to the production origin', () => {
		expect(canonicalUrl('/')).toBe('https://ditherette.mia.cx/');
	});

	it('drops query and hash', () => {
		expect(canonicalUrl('/guides/dithering?x=1#y')).toBe(
			'https://ditherette.mia.cx/guides/dithering'
		);
	});
});

describe('sitemapXml', () => {
	it('lists every sitemap path on the production origin', () => {
		const xml = sitemapXml();
		const locs = [...xml.matchAll(/<loc>([^<]+)<\/loc>/g)].map((match) => match[1]);
		expect(locs).toEqual(SITEMAP_PATHS.map(canonicalUrl));
	});
});

describe('guide registry', () => {
	it('has a +page.svelte for every registered slug and nothing else', () => {
		const slugs = readdirSync('src/routes/guides', { withFileTypes: true })
			.filter((entry) => entry.isDirectory())
			.map((entry) => entry.name)
			.sort();
		expect(slugs).toEqual(GUIDES.map((guide) => guide.slug).sort());
		for (const guide of GUIDES) {
			expect(readdirSync(`src/routes/guides/${guide.slug}`)).toContain('+page.svelte');
		}
	});
});

describe('robots.txt', () => {
	const robots = readFileSync('static/robots.txt', 'utf8');

	it('names the sitemap', () => {
		expect(robots).toContain('Sitemap: https://ditherette.mia.cx/sitemap.xml');
	});

	it('disallows only /api/', () => {
		const disallows = robots
			.split('\n')
			.map((line) => line.trim())
			.filter((line) => line.toLowerCase().startsWith('disallow:'));
		expect(disallows).toEqual(['Disallow: /api/']);
	});
});

describe('jsonLdScript', () => {
	it('escapes </script> inside a value', () => {
		const html = jsonLdScript({ name: 'a</script><b>' });
		expect(html).toContain('a\\u003c/script>');
		expect(html.split('<script')).toHaveLength(2);
	});
});
