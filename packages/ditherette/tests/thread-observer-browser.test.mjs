import test from 'node:test';
import { installedBrowserChecks } from './installed-browser.mjs';
import { lifetimeObserverChecks, threadTestAssets } from './thread-browser-driver.mjs';

test('real worker lifetime observer sees nested workers terminate', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{ actualWorkers: 2, releasedAfterHostTermination: 2 },
		{
			isolated: true,
			assets: threadTestAssets,
			driver: lifetimeObserverChecks
		}
	));
