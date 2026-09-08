import test from 'node:test';
import { installedBrowserChecks } from './installed-browser.mjs';
import { threadedOwnershipDriver, threadTestAssets } from './thread-browser-driver.mjs';

test('installed threaded processors own separate pools, memory and durable outputs', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{
			methods: 5,
			callbackFailures: 10,
			customInputs: 9,
			independentMemories: 2,
			cleanup: true
		},
		{ isolated: true, assets: threadTestAssets, driver: threadedOwnershipDriver }
	));
