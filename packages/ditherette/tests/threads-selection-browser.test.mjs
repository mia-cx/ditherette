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

test('disabled selection stays thread-free even when isolated', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{ scalarSelections: 2, requiredCapabilityError: false },
		{
			isolated: true,
			driver: (args) =>
				scalarSelectionDriver({ ...args, input: { ...args.input, capabilityUnavailable: false } })
		}
	));
