import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, join, relative, resolve as resolvePath } from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { browserEngines, browserLaunchOptions } from './browser-engines.mjs';
import { browserChecks } from './browser-fixture.mjs';
import { fieldBrowserChecks } from './field-browser-fixture.mjs';
import { diffusionBrowserChecks } from './diffusion-browser-fixture.mjs';
import { processBrowserChecks } from './process-browser-fixture.mjs';
import { yiluomaBrowserChecks } from './yiluoma-browser-fixture.mjs';
import { yiluomaBenchmarkChecks } from './yiluoma-benchmark-fixture.mjs';
import { prepareYliluomaOracle, yiluomaOracleChecks } from './yiluoma-oracle-fixture.mjs';
import { stageCacheBrowserChecks } from './stage-cache-browser-fixture.mjs';
import { progressBrowserChecks } from './progress-browser-fixture.mjs';
import { sourceSnapshotBrowserChecks } from './source-snapshot-browser-fixture.mjs';

test('installed tarball loads only scalar assets and runs the public contract in browser engines', async (t) => {
	const directory = await mkdtemp(join(tmpdir(), 'ditherette-tarball-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const oracle = await prepareYliluomaOracle(directory);
	const packageDirectory = fileURLToPath(new URL('../', import.meta.url));
	const vectors = JSON.parse(await readFile(new URL('./fixtures/fields.json', import.meta.url)));
	const diffusionVectors = JSON.parse(
		await readFile(new URL('./fixtures/diffusion.json', import.meta.url))
	);
	const yiluoma = JSON.parse(
		await readFile(new URL('./fixtures/yiluoma-wasm.json', import.meta.url))
	);
	const manifest = JSON.parse(await readFile(join(packageDirectory, 'package.json'), 'utf8'));
	const tarball = process.env.DITHERETTE_TEST_TARBALL
		? resolvePath(process.env.DITHERETTE_TEST_TARBALL)
		: join(directory, `ditherette-${manifest.version}.tgz`);
	if (!process.env.DITHERETTE_TEST_TARBALL)
		execFileSync('pnpm', ['pack', '--out', tarball], { cwd: packageDirectory, stdio: 'pipe' });
	const digest = createHash('sha256')
		.update(await readFile(tarball))
		.digest('hex');
	if (process.env.DITHERETTE_TEST_TARBALL_SHA256)
		assert.equal(digest, process.env.DITHERETTE_TEST_TARBALL_SHA256);
	t.diagnostic(`Installed tarball SHA-256 ${digest}`);
	const files = execFileSync('tar', ['-tzf', tarball], { encoding: 'utf8' }).trim().split('\n');
	for (const file of [
		'package/dist/index.js',
		'package/dist/index.d.ts',
		'package/dist/wasm/scalar/ditherette_wasm.factory.js',
		'package/dist/wasm/scalar/ditherette_wasm_bg.wasm',
		'package/dist/wasm/threads/ditherette_wasm_bg.wasm',
		'package/README.md',
		'package/LICENSE'
	])
		assert.ok(files.includes(file), file);
	assert.ok(
		files.some((file) =>
			/dist\/wasm\/threads\/snippets\/.*workerHelpers\.no-bundler\.js$/.test(file)
		),
		'threaded worker helper included'
	);
	assert.ok(
		files.every(
			(file) => !/\/(src|tests|scripts|target|bench)\//.test(file) || file.includes('/snippets/')
		),
		'no source/build/test/benchmark directories'
	);
	assert.ok(
		files.every((file) => !file.endsWith('.rs')),
		'no Rust source'
	);
	const consumer = join(directory, 'consumer');
	await mkdir(consumer);
	await writeFile(
		join(consumer, 'package.json'),
		JSON.stringify({
			private: true,
			type: 'module',
			dependencies: { ditherette: `file:${tarball}` }
		})
	);
	execFileSync('pnpm', ['install', '--offline', '--ignore-scripts', '--lockfile=false'], {
		cwd: consumer,
		stdio: 'pipe'
	});
	const resolve = createRequire(join(consumer, 'package.json'));
	assert.throws(() => resolve.resolve('ditherette/dist/wasm/scalar/ditherette_wasm.js'), {
		code: 'ERR_PACKAGE_PATH_NOT_EXPORTED'
	});
	const requests = [];
	const server = createServer(async (request, response) => {
		const pathname = new URL(request.url, 'http://localhost').pathname;
		requests.push(pathname);
		if (
			[
				'/benchmark/benchmark-public-page.mjs',
				'/benchmark/benchmark-public-timing.mjs',
				'/benchmark/benchmark-indexed-wire.mjs',
				'/benchmark/benchmark-stage-cache.mjs',
				'/benchmark/benchmark-progress.mjs',
				'/benchmark/benchmark-row-policy.mjs'
			].includes(pathname)
		) {
			response.writeHead(200, { 'Content-Type': 'text/javascript' });
			response.end(
				await readFile(new URL(`../../../scripts/${pathname.split('/').at(-1)}`, import.meta.url))
			);
			return;
		}
		if (pathname === '/') {
			response.writeHead(200, { 'Content-Type': 'text/html' });
			response.end(
				'<!doctype html><title>Installed ditherette fixture</title><script type="importmap">{"imports":{"ditherette":"/node_modules/ditherette/dist/index.js"}}</script>'
			);
			return;
		}
		const file = join(consumer, decodeURIComponent(pathname));
		if (relative(consumer, file).startsWith('..')) {
			response.writeHead(404).end();
			return;
		}
		try {
			const data = await readFile(file);
			response.writeHead(200, {
				'Content-Type': extname(file) === '.wasm' ? 'application/wasm' : 'text/javascript'
			});
			response.end(data);
		} catch {
			response.writeHead(404).end();
		}
	});
	await new Promise((resolve, reject) => {
		server.once('error', reject);
		server.listen(0, '127.0.0.1', resolve);
	});
	t.after(
		() =>
			new Promise((resolve) => {
				server.closeAllConnections();
				server.close(resolve);
			})
	);
	const origin = `http://127.0.0.1:${server.address().port}`;
	for (const [name, engine] of browserEngines()) {
		await t.test(name, async () => {
			const browser = await engine.launch(browserLaunchOptions(name));
			try {
				const yiluomaReference = await yiluomaOracleChecks(browser, name, oracle, yiluoma, tarball);
				const page = await browser.newPage();
				await page.goto(origin);
				requests.length = 0;
				const exports = await page.evaluate(async () => {
					const wasm = globalThis.WebAssembly;
					const fetch = globalThis.fetch;
					const worker = globalThis.Worker;
					globalThis.WebAssembly = undefined;
					globalThis.fetch = globalThis.Worker = () => {
						throw new Error('Root import has a runtime side effect.');
					};
					try {
						return Object.keys(await import('ditherette')).sort();
					} finally {
						globalThis.WebAssembly = wasm;
						globalThis.fetch = fetch;
						globalThis.Worker = worker;
					}
				});
				assert.deepEqual(exports, ['DitheretteError', 'createDitherette']);
				assert.ok(
					requests.every((path) => !/factory|\.wasm|threads/.test(path)),
					'inert root import'
				);
				const result = await page.evaluate(
					browserChecks,
					`${origin}/node_modules/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm`
				);
				assert.deepEqual(result, {
					anchors: 9,
					convolutionCases: 54,
					trilinearCases: 27,
					quantizeCases: 15,
					customInputs: 8,
					scalarWithoutIsolation: true
				});
				assert.deepEqual(
					await page.evaluate(sourceSnapshotBrowserChecks, {
						wasmUrl: `${origin}/node_modules/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm`,
						vectors
					}),
					{ methods: 5 }
				);
				assert.deepEqual(
					await page.evaluate(stageCacheBrowserChecks, {
						wasmUrl: `${origin}/node_modules/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm`,
						vectors
					}),
					{
						methods: 5,
						compositions: 18,
						settingsChanges: 4,
						caughtCopies: 1,
						isolatedInstances: 2
					}
				);
				assert.deepEqual(
					await page.evaluate(progressBrowserChecks, {
						wasmUrl: `${origin}/node_modules/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm`,
						vectors
					}),
					{
						methods: 5,
						callbackFailures: 15,
						reentrantAttempts: 6,
						finalCopyFailures: 2,
						controlledClock: true
					}
				);
				assert.ok(
					requests.every((path) => !path.includes('/threads/')),
					'scalar never loads threaded artifacts'
				);
				assert.deepEqual(
					await page.evaluate(fieldBrowserChecks, {
						vectors,
						wasmUrl: `${origin}/node_modules/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm`
					}),
					{ fields: 110, compositions: 1650, caughtFailures: 5 }
				);
				t.diagnostic(`${name} ${browser.version()}: installed-tarball checks pass`);
				assert.deepEqual(
					await page.evaluate(processBrowserChecks, {
						wasmUrl: `${origin}/node_modules/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm`
					}),
					{
						compositions: 423,
						strictRequests: 9,
						caughtCopies: 3,
						exactBudget: true,
						scalarWithoutIsolation: true
					}
				);
				assert.deepEqual(
					await page.evaluate(diffusionBrowserChecks, {
						vectors: diffusionVectors,
						wasmUrl: `${origin}/node_modules/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm`
					}),
					{ diffusion: 360, scalarWithoutIsolation: true }
				);
				assert.deepEqual(
					await page.evaluate(yiluomaBrowserChecks, {
						vectors: yiluomaReference,
						wasmUrl: `${origin}/node_modules/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm`
					}),
					{ vectors: 367, caughtFailures: 3, strictControls: 8, exactBudget: true }
				);
				assert.equal(
					await page.evaluate(yiluomaBenchmarkChecks, { vectors: yiluomaReference }),
					734
				);
				t.diagnostic(
					`${name}: 367 frozen Wasm Yliluoma vectors and 734 untimed actual benchmark-adapter calls pass`
				);
			} finally {
				await browser.close();
			}
		});
	}
});
