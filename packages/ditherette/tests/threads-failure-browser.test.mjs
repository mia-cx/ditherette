import test from 'node:test';
import { installedBrowserChecks } from './installed-browser.mjs';
import { threadedFailureDriver, threadTestAssets } from './thread-browser-driver.mjs';

test('installed threaded startup cleans real partial workers before fallback or error', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{
			partialWorkersObserved: 4,
			preferredFallback: true,
			requiredInitializationError: true
		},
		{ isolated: true, assets: threadTestAssets, driver: threadedFailureDriver }
	));
