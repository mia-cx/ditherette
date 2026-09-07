import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { allowedRequest, assetPaths, startAssetServer } from './benchmark-public-browser.mjs';
import { timerResolution } from './benchmark-public-page.mjs';

test('manifest rejects traversal and duplicates; routing rejects external and undeclared dependencies', () => {
	for (const file of ['../outside', '/absolute', 'a/../b', 'a\\b'])
		assert.throws(
			() => assetPaths({ root: '/tmp/example', files: [{ path: file }] }),
			/Noncanonical/
		);
	assert.throws(
		() => assetPaths({ root: '/tmp/example', files: [{ path: 'a' }, { path: 'a' }] }),
		/Duplicate/
	);
	const files = assetPaths({ root: '/tmp/example', files: [{ path: 'asset.js' }] });
	assert.equal(allowedRequest('http://127.0.0.1:99/asset.js', 'http://127.0.0.1:99', files), true);
	for (const url of [
		'https://example.org/asset.js',
		'http://127.0.0.1:99/missing.js',
		'http://127.0.0.1:99/asset.js?bust=1',
		'http://127.0.0.1:99/%invalid'
	])
		assert.equal(allowedRequest(url, 'http://127.0.0.1:99', files), false);
});

test('asset server enforces closure and advertises requested isolation without running operations', async () => {
	const root = await mkdtemp(path.join(tmpdir(), 'ditherette-browser-assets-'));
	let server;
	try {
		await writeFile(path.join(root, 'entry.js'), 'export const fixture = true;');
		await writeFile(path.join(root, 'undeclared.js'), 'throw new Error("must not load")');
		server = await startAssetServer(
			{ tree: { root, files: [{ path: 'entry.js' }] }, entries: { page: 'entry.js' } },
			true
		);
		const response = await fetch(`${server.url}/entry.js`);
		assert.equal(await response.text(), 'export const fixture = true;');
		assert.equal(response.headers.get('cross-origin-embedder-policy'), 'require-corp');
		assert.equal(response.headers.get('cross-origin-opener-policy'), 'same-origin');
		assert.equal((await fetch(`${server.url}/undeclared.js`)).status, 404);
		assert.equal(server.failures.length, 1);
	} finally {
		if (server) {
			server.instance.closeAllConnections();
			await new Promise((resolve) => server.instance.close(resolve));
		}
		await rm(root, { recursive: true, force: true });
	}
});

test('clock quantum observation preserves coarse ticks and fails visibly for a stalled clock', () => {
	let calls = 0;
	assert.equal(
		timerResolution(() => Math.floor(calls++ / 10) * 2),
		2e6
	);
	assert.throws(() => timerResolution(() => 0), /Could not observe/);
});
