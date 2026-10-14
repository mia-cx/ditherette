import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { startBenchmarkServer } from './benchmark-wasm-resize.mjs';

test('benchmark HTTP errors stay local and assets still load before shutdown', async (t) => {
	const pkgDir = await mkdtemp(path.join(tmpdir(), 'benchmark-http-'));
	t.after(() => rm(pkgDir, { recursive: true, force: true }));
	const asset = 'export const benchmarkFixture = true;\n';
	await writeFile(path.join(pkgDir, 'fixture.js'), asset);
	const server = await startBenchmarkServer({
		pkgDir,
		fixtures: [{ name: 'fixture.js', file: path.join(pkgDir, 'fixture.js') }]
	});
	const close = () => {
		server.instance.closeAllConnections();
		return new Promise((resolve, reject) => {
			server.instance.close((error) => (error ? reject(error) : resolve()));
		});
	};
	t.after(() => {
		if (server.instance.listening) return close();
	});
	const stderr = [];
	t.mock.method(process.stderr, 'write', (chunk) => {
		stderr.push(String(chunk));
		return true;
	});

	for (const requestPath of ['pkg/%ZZ', 'pkg/missing.js']) {
		const response = await fetch(new URL(requestPath, server.url));
		assert.equal(response.status, 500);
		assert.equal(response.headers.get('content-type'), 'text/plain; charset=utf-8');
		assert.equal(await response.text(), 'Internal server error');
	}
	assert.match(stderr.join(''), /URIError: URI malformed/);
	assert.match(stderr.join(''), /benchmark-wasm-resize\.mjs:\d+/);
	assert.match(stderr.join(''), /ENOENT/);
	assert.ok(stderr.join('').includes(path.join(pkgDir, 'missing.js')));

	for (const requestPath of ['pkg/fixture.js', 'fixtures/fixture.js']) {
		const response = await fetch(new URL(requestPath, server.url));
		assert.equal(response.status, 200);
		assert.equal(response.headers.get('content-type'), 'text/javascript; charset=utf-8');
		assert.equal(response.headers.get('cross-origin-opener-policy'), 'same-origin');
		assert.equal(response.headers.get('cross-origin-embedder-policy'), 'require-corp');
		assert.equal(response.headers.get('cross-origin-resource-policy'), 'same-origin');
		assert.equal(await response.text(), asset);
	}
	await close();
	assert.equal(server.instance.listening, false);
});
