import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
	cp,
	mkdir,
	mkdtemp,
	readFile,
	readdir,
	realpath,
	rm,
	stat,
	writeFile
} from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { chromium, firefox, webkit } from 'playwright';
import { prepareTypeScript } from './prepare-benchmark-typescript.mjs';
import { exchangeTrial, restrictContext, startAssetServer } from './benchmark-public-browser.mjs';

// No runTrial/collectCalls call occurs here. Real operations have no timer around them.
test('installed package and actual TypeScript adapter conformance, without measurements', async (t) => {
	const tarball = process.env.DITHERETTE_BENCH_TEST_TARBALL;
	assert.ok(tarball, 'Set DITHERETTE_BENCH_TEST_TARBALL to an already-built package tarball.');
	const quantizePath = process.env.DITHERETTE_BENCH_QUANTIZE_FIXTURES;
	assert.ok(
		quantizePath,
		'Set DITHERETTE_BENCH_QUANTIZE_FIXTURES to the frozen quantize_conformance output.'
	);
	const quantizeFixtures = JSON.parse(await readFile(quantizePath, 'utf8'));
	assert.equal(quantizeFixtures.length, 47);
	assert.equal(new Set(quantizeFixtures.map((fixture) => fixture.settings.matching)).size, 15);
	const temporary = await mkdtemp(path.join(tmpdir(), 'ditherette-public-conformance-'));
	t.after(() => rm(temporary, { recursive: true, force: true }));
	const consumer = path.join(temporary, 'consumer');
	await mkdir(consumer);
	await writeFile(
		path.join(consumer, 'package.json'),
		JSON.stringify({
			private: true,
			type: 'module',
			dependencies: { ditherette: `file:${path.resolve(tarball)}` }
		})
	);
	execFileSync('pnpm', ['install', '--offline', '--ignore-scripts', '--lockfile=false'], {
		cwd: consumer,
		stdio: 'pipe'
	});
	const installed = await realpath(path.join(consumer, 'node_modules/ditherette'));
	const root = path.join(temporary, 'assets');
	const compiled = await prepareTypeScript(root);
	await writeFile(
		path.join(root, 'scripts/ipc-echo.mjs'),
		'export async function runTrial(request) { return { marker: request.marker, data: request.data }; }'
	);
	await cp(installed, path.join(root, 'package'), { recursive: true, dereference: true });
	for (const name of [
		'benchmark-public-page.mjs',
		'benchmark-public-timing.mjs',
		'benchmark-public-browser.mjs',
		'benchmark-transport.mjs'
	])
		await cp(fileURLToPath(new URL(name, import.meta.url)), path.join(root, 'scripts', name));
	const files = [];
	async function walk(relative = '') {
		for (const entry of await readdir(path.join(root, relative), { withFileTypes: true })) {
			const name = path.posix.join(relative, entry.name);
			if (entry.isDirectory()) await walk(name);
			else {
				const bytes = await readFile(path.join(root, name));
				const metadata = await stat(path.join(root, name));
				files.push({
					path: name,
					bytes: bytes.length,
					mode: metadata.mode & 0o777,
					digest: [...createHash('sha256').update(bytes).digest()]
				});
			}
		}
	}
	await walk();
	const assets = {
		tree: { root, files },
		entries: {
			package: 'package/dist/index.js',
			typescript: compiled.entry,
			transport: 'scripts/benchmark-public-browser.mjs',
			page: 'scripts/benchmark-public-page.mjs',
			wasm: 'package/dist/wasm/scalar/ditherette_wasm_bg.wasm'
		}
	};
	for (const [engineName, engine] of Object.entries({ chromium, firefox, webkit })) {
		await t.test(engineName, async () => {
			const server = await startAssetServer(assets, true, {
				case: { identity: { output: { width: 1, height: 1 } }, measurement: { samples: 1 } },
				marker: 'untimed-conformance',
				data: [10, 20, 30, 40]
			});
			let browser;
			try {
				browser = await engine.launch({
					executablePath:
						engineName === 'webkit' ? process.env.DITHERETTE_TEST_WEBKIT_EXECUTABLE : undefined
				});
				const context = await browser.newContext({ serviceWorkers: 'block' });
				await restrictContext(context, server);
				const page = await context.newPage();
				await page.goto(server.url);
				const report = await page.evaluate(
					async ({ assets, quantizeFixtures }) => {
						const { prepareOperation, preflightOperation, outputStability } = await import(
							`/${assets.entries.page}`
						);
						const equal = (actual, expected, label) => {
							if (JSON.stringify(actual) !== JSON.stringify(expected))
								throw new Error(
									`${label}: actual ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`
								);
						};
						const trial = (
							backend,
							preparation = 'primed-instance',
							width = 4,
							outputWidth = 3
						) => ({
							role: 'candidate',
							browser: { assets },
							case: {
								source: { width, height: 1 },
								rgba: Array.from({ length: width }, (_, i) => [i + 10, 30, 70, i * 11]).flat(),
								identity: { output: { width: outputWidth, height: 1 } },
								measurement: {
									scope: preparation.startsWith('initialization')
										? 'initialization'
										: 'complete-call',
									mode: 'single-call',
									application_cache: 'not-applicable'
								},
								browser: {
									operation: { operation: 'resize-nearest', anchor: 'center' },
									candidate: backend,
									preparation,
									cache: 'none'
								}
							}
						});
						const expected = [10, 30, 70, 0, 12, 30, 70, 22, 13, 30, 70, 33];
						const reference = {
							dimensions: { width: 3, height: 1 },
							pixels: { format: 'rgba8', data: expected },
							warnings: []
						};
						for (const backend of ['typescript', 'package']) {
							const operation = await prepareOperation(trial(backend));
							try {
								equal(
									await preflightOperation(operation, reference),
									undefined,
									`${backend} reference preflight`
								);
								equal(Array.from(operation.call().data), expected, `${backend} known 4→3`);
							} finally {
								operation.close();
							}
							const identity = await prepareOperation(trial(backend, 'primed-instance', 4, 4));
							try {
								const result = identity.call();
								identity.request.source.data[0] = 255;
								equal(result.data[0], 10, `${backend} durable identity`);
							} finally {
								identity.close();
							}
						}
						for (const [filter, expectedRed] of [
							['area', [16, 96, 176]],
							['bilinear', [17, 96, 175]]
						]) {
							for (const backend of ['typescript', 'package']) {
								const request = trial(backend);
								request.case.rgba = [0, 64, 128, 192].flatMap((red) => [red, 30, 70, 255]);
								request.case.browser.operation = {
									operation: `resize-${filter}`,
									...(filter === 'area' ? {} : { anchor: 'center' })
								};
								const operation = await prepareOperation(request);
								try {
									// Website area is an unweighted inclusive box, not fractional-overlap area.
									// Website bilinear keeps two taps; frozen/landed triangle support widens on reduction.
									const actualRed =
										backend === 'typescript'
											? filter === 'area'
												? [21, 96, 171]
												: [11, 96, 181]
											: expectedRed;
									equal(
										Array.from(operation.call().data),
										actualRed.flatMap((red) => [red, 30, 70, 255]),
										`${backend} ${filter} known 4→3`
									);
									if (backend === 'typescript') {
										const mismatch = await preflightOperation(operation, {
											dimensions: { width: 3, height: 1 },
											pixels: {
												format: 'rgba8',
												data: expectedRed.flatMap((red) => [red, 30, 70, 255])
											},
											warnings: []
										});
										equal(
											mismatch.pixels.data,
											actualRed.flatMap((red) => [red, 30, 70, 255]),
											`retain website ${filter} semantic difference`
										);
									}
								} finally {
									operation.close();
								}
							}
						}
						// Nontrivial convolution vectors live in the native and package suites.
						// This checks every actual benchmark adapter recipe without collecting timings.
						for (const filter of ['bicubic', 'lanczos2', 'lanczos3']) {
							for (const support of ['fixed', 'scale-aware']) {
								for (const backend of filter === 'bicubic'
									? ['package']
									: ['typescript', 'package']) {
									const request = trial(backend, 'primed-instance', 7, 3);
									request.case.rgba = Array.from({ length: 7 }, () => [83, 147, 219, 255]).flat();
									request.case.browser.operation = {
										operation: `resize-${filter}`,
										anchor: 'center',
										support
									};
									const operation = await prepareOperation(request);
									try {
										equal(
											Array.from(operation.call().data),
											Array.from({ length: 3 }, () => [83, 147, 219, 255]).flat(),
											`${backend} ${filter} ${support} adapter`
										);
									} finally {
										operation.close();
									}
								}
							}
						}
						const freshTypeScript = await prepareOperation(trial('typescript', 'fresh-instance'));
						try {
							const prepared = await freshTypeScript.prepare();
							try {
								equal(
									Array.from(prepared.call().data),
									expected,
									'stateless TypeScript per-call preparation'
								);
							} finally {
								prepared.close();
							}
						} finally {
							freshTypeScript.close();
						}
						for (const preparation of [
							'fresh-instance',
							'initialization-bytes',
							'initialization-compiled'
						]) {
							const operation = await prepareOperation(trial('package', preparation));
							try {
								if (operation.create) {
									const instance = await operation.create();
									try {
										equal(Array.from(operation.probe(instance).data), expected, preparation);
									} finally {
										instance.dispose();
									}
								} else {
									const one = await operation.prepare();
									const result = one.call();
									one.close();
									const two = await operation.prepare();
									try {
										equal(Array.from(two.call().data), expected, preparation);
										equal(
											Array.from(result.data),
											expected,
											'durable after disposal and later instance'
										);
									} finally {
										two.close();
									}
								}
							} finally {
								operation.close();
							}
						}
						for (const [anchor, red] of [
							['left', 68],
							['center', 85],
							['right', 103]
						]) {
							const request = trial('package', 'primed-instance', 3, 1);
							request.case.rgba = [0, 0, 255].flatMap((r) => [r, 0, 0, 255]);
							request.case.browser.operation = { operation: 'resize-trilinear', anchor };
							const operation = await prepareOperation(request);
							try {
								equal(
									await preflightOperation(operation, {
										dimensions: { width: 1, height: 1 },
										pixels: { format: 'rgba8', data: [red, 0, 0, 255] },
										warnings: []
									}),
									undefined,
									`trilinear odd mip ${anchor}`
								);
							} finally {
								operation.close();
							}
						}
						const drift = [];
						// Exact center samples the first source pixel 24 times, then the second 25 times.
						const driftReference = {
							dimensions: { width: 49, height: 1 },
							pixels: {
								format: 'rgba8',
								data: Array.from({ length: 49 }, (_, x) =>
									x < 24 ? [10, 30, 70, 0] : [11, 30, 70, 11]
								).flat()
							},
							warnings: []
						};
						for (const backend of ['typescript', 'package']) {
							const operation = await prepareOperation(trial(backend, 'primed-instance', 2, 49));
							try {
								drift.push(operation.call().data[24 * 4]);
								const mismatch = await preflightOperation(operation, driftReference);
								if (backend === 'package') equal(mismatch, undefined, 'exact package preflight');
								else {
									equal(mismatch.pixels.data[24 * 4], 10, 'mismatch retains actual byte');
									equal(mismatch.pixels.data.length, 49 * 4, 'mismatch retains complete output');
								}
							} finally {
								operation.close();
							}
						}
						equal(drift, [10, 11], 'Retain actual TS 2→49 rounding mismatch at x=24.');
						for (const fixture of quantizeFixtures) {
							for (const preparation of ['primed-instance', 'fresh-instance']) {
								const request = trial('package', preparation);
								request.case.source = fixture.source;
								request.case.rgba = fixture.rgba;
								request.case.identity.output = fixture.source;
								request.case.browser.operation = {
									operation: 'quantize',
									settings: fixture.settings
								};
								const operation = await prepareOperation(request);
								try {
									const stability = outputStability(
										fixture.source.width * fixture.source.height + 1024,
										'indexed8'
									);
									equal(
										await preflightOperation(operation, fixture.reference, stability.observe),
										undefined,
										`quantize ${fixture.settings.matching} ${fixture.settings.alpha.mode} ${preparation}`
									);
									const one = await operation.prepare();
									const first = one.call();
									stability.observe([first]);
									one.close();
									const retained = Array.from(first.indices);
									const two = await operation.prepare();
									try {
										stability.observe([two.call()]);
									} finally {
										two.close();
									}
									equal(
										Array.from(first.indices),
										retained,
										'durable quantize output after later call/disposal'
									);
								} finally {
									operation.close();
								}
							}
						}
						const noncenter = trial('typescript');
						noncenter.case.browser.operation.anchor = 'top-left';
						let rejected = false;
						try {
							await prepareOperation(noncenter);
						} catch (error) {
							rejected = error.message.includes('non-center');
						}
						if (!rejected) throw new Error('Non-center TS must be unavailable.');
						return {
							isolated: crossOriginIsolated,
							drift,
							checked: [
								'known-vector',
								'area-bilinear-known-vectors-and-drift',
								'convolution-support-recipes',
								'47-frozen-quantize-fixtures-all-15-modes-primed-and-fresh',
								'identity-copy',
								'fresh-instance',
								'initialization-bytes',
								'initialization-compiled',
								'typescript-drift',
								'noncenter-unavailable'
							]
						};
					},
					{ assets, quantizeFixtures }
				);
				assert.equal(report.isolated, true);
				assert.deepEqual(report.drift, [10, 11]);
				assert.deepEqual(await exchangeTrial(page, server, 'scripts/ipc-echo.mjs'), {
					marker: 'untimed-conformance',
					data: [10, 20, 30, 40]
				});
				assert.deepEqual(server.failures, []);
				await assert.rejects(page.evaluate(() => import('/undeclared.js')));
				assert.equal(server.failures.length, 1);
				t.diagnostic(
					`${engineName} ${browser.version()}: ${report.checked.join(', ')}; no measurements`
				);
			} finally {
				await browser?.close();
				server.instance.closeAllConnections();
				await new Promise((resolve) => server.instance.close(resolve));
			}
		});
	}
	t.diagnostic(
		`fixture tarball sha256 ${createHash('sha256')
			.update(await readFile(tarball))
			.digest('hex')}`
	);
});
