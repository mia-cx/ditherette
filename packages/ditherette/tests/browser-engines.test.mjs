import assert from 'node:assert/strict';
import test from 'node:test';
import { browserEngines, browserLaunchOptions } from './browser-engines.mjs';

test('browser selection defaults to all engines and rejects empty or unknown coverage', () => {
	assert.deepEqual(
		browserEngines({}).map(([name]) => name),
		['chromium', 'firefox', 'webkit']
	);
	assert.deepEqual(
		browserEngines({ DITHERETTE_TEST_ENGINES: 'firefox,chromium' }).map(([name]) => name),
		['firefox', 'chromium']
	);
	for (const value of ['', 'chrome', 'chromium,', 'webkit,webkit', 'firefox, webkit'])
		assert.throws(() => browserEngines({ DITHERETTE_TEST_ENGINES: value }), /unique/);
	assert.deepEqual(
		browserLaunchOptions('firefox', { DITHERETTE_TEST_FIREFOX_EXECUTABLE: '/owned/firefox' }),
		{ executablePath: '/owned/firefox' }
	);
	assert.deepEqual(browserLaunchOptions('webkit', {}), { executablePath: undefined });
});
