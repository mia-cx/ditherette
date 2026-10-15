import { sitemapXml } from '$lib/seo';
import type { RequestHandler } from './$types';

export const prerender = true;

export const GET: RequestHandler = () =>
	new Response(sitemapXml(), { headers: { 'content-type': 'application/xml' } });
