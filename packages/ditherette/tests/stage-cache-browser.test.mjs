import test from 'node:test';
import { installedBrowserChecks } from './installed-browser.mjs';
import { stageCacheBrowserChecks } from './stage-cache-browser-fixture.mjs';

test('installed package preserves stage ownership across calls', (t) =>
	installedBrowserChecks(t, stageCacheBrowserChecks, {
		methods: 5,
		compositions: 18,
		settingsChanges: 4,
		caughtCopies: 1,
		isolatedInstances: 2
	}));
