import test from 'node:test';
import { installedBrowserChecks } from './installed-browser.mjs';
import { progressBrowserChecks } from './progress-browser-fixture.mjs';

test('installed package reports progress and contains callback failures', (t) =>
	installedBrowserChecks(t, progressBrowserChecks, {
		methods: 5,
		callbackFailures: 15,
		reentrantAttempts: 6,
		finalCopyFailures: 2,
		controlledClock: true
	}));
