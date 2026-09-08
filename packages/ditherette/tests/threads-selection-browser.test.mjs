import test from 'node:test';
import { installedBrowserChecks } from './installed-browser.mjs';
import { scalarSelectionDriver } from './thread-browser-driver.mjs';

test('installed scalar selection stays thread-free and required reports missing capability', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{ scalarSelections: 3, requiredCapabilityError: true },
		{
			driver: scalarSelectionDriver
		}
	));
