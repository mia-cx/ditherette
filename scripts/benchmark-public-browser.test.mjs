import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import test from 'node:test';
import { allowedRequest, assetPaths, startAssetServer } from './benchmark-public-browser.mjs';
import { preflightOperation, runTrial, timerResolution } from './benchmark-public-page.mjs';

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

test('untimed preflight keeps exact mismatch bytes and checks dimensions, bytes, and warnings', async () => {
	let calls = 0;
	let closes = 0;
	const output = { width: 1, height: 1, data: new Uint8Array([10, 20, 30, 40]) };
	const operation = {
		prepare: async () => ({
			call: () => {
				calls++;
				return output;
			},
			close: () => closes++
		})
	};
	const reference = {
		dimensions: { width: 1, height: 1 },
		pixels: { format: 'rgba8', data: [10, 20, 30, 40] },
		warnings: []
	};
	assert.equal(await preflightOperation(operation, reference), undefined);
	for (const wrong of [
		{ ...reference, dimensions: { width: 2, height: 1 } },
		{ ...reference, pixels: { format: 'rgba8', data: [11, 20, 30, 40] } },
		{ ...reference, warnings: ['unexpected'] }
	])
		assert.deepEqual(await preflightOperation(operation, wrong), reference);
	assert.equal(calls, 4);
	assert.equal(closes, 4);
	await assert.rejects(preflightOperation(operation), /requires frozen/);
	assert.equal(calls, 4);
});

test('mismatch response performs one fake preflight call and never begins warmup or timing', async () => {
	const temporary = await mkdtemp(path.join(tmpdir(), 'ditherette-mismatch-fixture-'));
	const previousLocation = Object.getOwnPropertyDescriptor(globalThis, 'location');
	const previousIsolation = Object.getOwnPropertyDescriptor(globalThis, 'crossOriginIsolated');
	try {
		const entry = path.join(temporary, 'fixture.mjs');
		await writeFile(
			entry,
			'export let calls = 0; export function resize(request) { calls++; return { width: 1, height: 1, data: new Uint8Array([10,20,30,40]) }; }'
		);
		Object.defineProperty(globalThis, 'location', {
			configurable: true,
			value: { href: 'file:///' }
		});
		Object.defineProperty(globalThis, 'crossOriginIsolated', { configurable: true, value: false });
		const trial = {
			role: 'candidate',
			pair: 2,
			browser: { assets: { entries: { typescript: entry.slice(1) } } },
			case: {
				name: 'mismatch',
				source: { width: 1, height: 1 },
				rgba: [10, 20, 30, 40],
				identity: { input: [1], settings: [2], output: { width: 1, height: 1 } },
				measurement: {
					mode: 'single-call',
					scope: 'complete-call',
					application_cache: 'not-applicable',
					warmup_ms: 250,
					samples: 100
				},
				browser: {
					operation: { operation: 'resize-nearest', anchor: 'center' },
					candidate: 'typescript',
					preparation: 'primed-instance',
					cache: 'none'
				}
			},
			reference_output: {
				dimensions: { width: 1, height: 1 },
				pixels: { format: 'rgba8', data: [11, 20, 30, 40] },
				warnings: []
			}
		};
		const result = await runTrial(trial);
		assert.equal((await import(pathToFileURL(entry))).calls, 1);
		assert.equal(result.timing_skipped, 'reference-mismatch');
		assert.deepEqual(result.sample_ns, []);
		assert.equal(result.iterations_per_sample, 0);
		assert.equal(result.warmup_iterations, 0);
		assert.equal(result.warmup_elapsed_ns, 0);
		assert.deepEqual(result.output.pixels.data, [10, 20, 30, 40]);
		assert.equal(result.pair, 2);
	} finally {
		if (previousLocation) Object.defineProperty(globalThis, 'location', previousLocation);
		else delete globalThis.location;
		if (previousIsolation)
			Object.defineProperty(globalThis, 'crossOriginIsolated', previousIsolation);
		else delete globalThis.crossOriginIsolated;
		await rm(temporary, { recursive: true, force: true });
	}
});
