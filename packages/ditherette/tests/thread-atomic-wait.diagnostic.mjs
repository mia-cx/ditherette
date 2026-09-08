import test from 'node:test';
import { installedBrowserChecks } from './installed-browser.mjs';
import { atomicWaitDriver, threadTestAssets } from './thread-browser-driver.mjs';

test('engine terminates a worker in the upstream WebKit atomic.wait reproducer', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{ wasmWaitWorkerTerminated: true },
		{
			isolated: true,
			assets: threadTestAssets,
			driver: atomicWaitDriver
		}
	));
