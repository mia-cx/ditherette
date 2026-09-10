import test from 'node:test';
import { installedBrowserChecks } from './installed-browser.mjs';
import { sourceSnapshotBrowserChecks } from './source-snapshot-browser-fixture.mjs';

test('installed package verifies current source bytes before reusing its snapshot', (t) =>
	installedBrowserChecks(t, sourceSnapshotBrowserChecks, { methods: 5 }));
