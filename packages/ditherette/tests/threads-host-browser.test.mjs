import test from 'node:test';
import { installedBrowserChecks } from './installed-browser.mjs';
import { processingHostDriver, threadTestAssets } from './thread-browser-driver.mjs';

test('terminating a processing host releases its actual worker tree', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{
			hostTerminationDuringCall: true,
			hostTerminationDuringStartup: true
		},
		{ isolated: true, assets: threadTestAssets, driver: processingHostDriver }
	));
