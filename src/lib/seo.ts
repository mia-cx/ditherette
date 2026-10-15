import { GUIDES, guidePath } from '$lib/guides';

/** Canonical production origin; every absolute URL the site emits uses it. */
export const SITE_ORIGIN = 'https://ditherette.mia.cx';
export const SITE_NAME = 'Ditherette';

/** Open Graph/Twitter card image served from `static/`. */
export const SOCIAL_IMAGE = {
	path: '/social-preview.png',
	width: 1200,
	height: 630,
	alt: 'The ditherette wordmark beside a sunset reduced to Wplace colours with dithering.'
} as const;

/** Absolute production URL for a site path, with query and hash dropped. */
export function canonicalUrl(path: string): string {
	const [pathname] = path.split(/[?#]/);
	return new URL(pathname, SITE_ORIGIN).href;
}

/** Every path listed in the sitemap: the editor, the hub and each guide. */
export const SITEMAP_PATHS = ['/', '/guides', ...GUIDES.map((guide) => guidePath(guide.slug))];

function escapeXml(value: string): string {
	return value
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;')
		.replaceAll("'", '&apos;');
}

/** Sitemap document with a `<loc>` per path; no lastmod/priority/changefreq. */
export function sitemapXml(paths: readonly string[] = SITEMAP_PATHS): string {
	const urls = paths
		.map((path) => `\t<url>\n\t\t<loc>${escapeXml(canonicalUrl(path))}</loc>\n\t</url>`)
		.join('\n');
	return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>\n`;
}

/**
 * Serialises structured data for embedding in a `<script type="application/ld+json">`
 * element. `<` is serialised as a unicode escape so a `</script>` inside a value
 * cannot close the tag.
 */
export function jsonLdScript(data: Record<string, unknown>): string {
	const json = JSON.stringify(data).replaceAll('<', '\\u003c');
	return `<script type="application/ld+json">${json}</script>`;
}
