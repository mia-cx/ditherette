import { resolve } from '$app/paths';

/**
 * Applies SvelteKit's base path only to in-app absolute paths.
 * Other anchor hrefs must keep normal browser semantics.
 */
export function resolveButtonHref(href: string | undefined) {
	if (!href) return href;
	if (href.startsWith('/') && !href.startsWith('//')) return resolve(href as '/');
	return href;
}
