import { afterEach, expect, it, vi } from 'vitest';
import { settingsHash } from '$lib/processing/hash';

afterEach(() => vi.resetModules());

// Proves corrupted persisted bytes cannot prevent the editor's stores from loading.
it('recovers default dither settings when saved JSON is malformed', async () => {
	vi.resetModules();
	const { setPersistentEngine } = await import('@nanostores/persistent');
	setPersistentEngine(
		{ 'ditherette:dither': '{"algorithm":' },
		{ addEventListener() {}, removeEventListener() {} }
	);

	const { ditherSettings } = await import('./app');
	expect(ditherSettings.get()).toEqual({
		algorithm: 'none',
		strength: 100,
		placement: 'everywhere',
		placementSampleSize: 64,
		placementContrastThreshold: 12,
		serpentine: true,
		seed: 0xc0ffee42,
		useColorSpace: false
	});
});

// Proves the raw decoder preserves valid choices and still invalidates legacy cached outputs.
it.each([false, true])('loads saved dither settings with legacy=%s', async (legacy) => {
	vi.resetModules();
	const saved = {
		algorithm: 'bayer-16',
		strength: 30,
		placement: 'adaptive',
		...(legacy
			? { placementRadius: 6, placementThreshold: 12, placementSoftness: 12 }
			: { placementSampleSize: 96, placementContrastThreshold: 6 }),
		serpentine: false,
		seed: 7,
		useColorSpace: true
	};
	const { setPersistentEngine } = await import('@nanostores/persistent');
	setPersistentEngine(
		{ 'ditherette:dither': JSON.stringify(saved) },
		{ addEventListener() {}, removeEventListener() {} }
	);

	const { ditherSettings } = await import('./app');
	expect(ditherSettings.get()).toEqual({
		algorithm: 'bayer-16',
		strength: 30,
		placement: 'adaptive',
		placementSampleSize: legacy ? 64 : 96,
		placementContrastThreshold: legacy ? 12 : 6,
		serpentine: false,
		seed: 7,
		useColorSpace: true
	});
	expect(settingsHash(ditherSettings.get()) === settingsHash(saved)).toBe(!legacy);
});
