import { describe, expect, it } from 'vitest';
import { resolveButtonHref } from './href';

describe('resolveButtonHref', () => {
	it('resolves root-relative app paths through SvelteKit', () => {
		expect(resolveButtonHref('/settings')).toBe('/settings');
	});

	it('preserves non-app anchor hrefs', () => {
		expect(resolveButtonHref('https://example.com')).toBe('https://example.com');
		expect(resolveButtonHref('mailto:hello@example.com')).toBe('mailto:hello@example.com');
		expect(resolveButtonHref('#export')).toBe('#export');
		expect(resolveButtonHref('relative/path')).toBe('relative/path');
		expect(resolveButtonHref('//cdn.example.com/file.png')).toBe('//cdn.example.com/file.png');
	});
});
