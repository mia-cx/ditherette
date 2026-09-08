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
import { requireMatchingComposition } from './benchmark-public-page.mjs';
import { chromium, firefox, webkit } from 'playwright';
import { prepareTypeScript } from './prepare-benchmark-typescript.mjs';
import {
	exchangeTrial,
	frozenBrowserReference,
	restrictContext,
	startAssetServer
} from './benchmark-public-browser.mjs';

// No runTrial/collectCalls call occurs here. Real operations have no timer around them.
test('installed package and actual TypeScript adapter conformance, without measurements', async (t) => {
	const tarball = process.env.DITHERETTE_BENCH_TEST_TARBALL;
	const oracle = process.env.DITHERETTE_BENCH_ORACLE;
	assert.ok(oracle, 'Set DITHERETTE_BENCH_ORACLE to a built frozen-only oracle directory.');
	assert.ok(tarball, 'Set DITHERETTE_BENCH_TEST_TARBALL to an already-built package tarball.');
	const quantizePath = process.env.DITHERETTE_BENCH_QUANTIZE_FIXTURES;
	assert.ok(
		quantizePath,
		'Set DITHERETTE_BENCH_QUANTIZE_FIXTURES to the frozen quantize_conformance output.'
	);
	const quantizeFixtures = JSON.parse(await readFile(quantizePath, 'utf8'));
	assert.equal(quantizeFixtures.length, 47);
	assert.equal(new Set(quantizeFixtures.map((fixture) => fixture.settings.matching)).size, 15);
	const fieldPath = process.env.DITHERETTE_BENCH_FIELD_FIXTURES;
	assert.ok(
		fieldPath,
		'Set DITHERETTE_BENCH_FIELD_FIXTURES to the frozen field_conformance output.'
	);
	const fieldFixtures = JSON.parse(await readFile(fieldPath, 'utf8'));
	assert.equal(fieldFixtures.length, 12);
	assert.equal(
		fieldFixtures.filter((fixture) => fixture.operation.operation === 'perturb').length,
		7
	);
	const blueNoisePath = process.env.DITHERETTE_BENCH_BLUE_NOISE_FIXTURES;
	if (blueNoisePath) {
		const blueNoiseFixtures = JSON.parse(await readFile(blueNoisePath, 'utf8'));
		assert.equal(blueNoiseFixtures.length, 4);
		for (const fixture of blueNoiseFixtures) {
			assert.deepEqual(fixture.source, { width: 65, height: 33 });
			const settings = fixture.operation.settings;
			assert.deepEqual((settings.perturb ?? settings).field, { algorithm: 'blue-noise' });
		}
		fieldFixtures.push(...blueNoiseFixtures);
	}
	const diffusionPath = process.env.DITHERETTE_BENCH_DIFFUSION_FIXTURES;
	if (diffusionPath) {
		const fixtures = JSON.parse(await readFile(diffusionPath, 'utf8'));
		assert.equal(fixtures.length, 360);
		assert.equal(new Set(fixtures.map(({ operation }) => operation.settings.kernel)).size, 4);
		assert.equal(new Set(fixtures.map(({ operation }) => operation.settings.feedback)).size, 2);
		assert.equal(
			new Set(fixtures.map(({ operation }) => operation.settings.quantize.matching)).size,
			15
		);
		for (const fixture of fixtures) assert.equal(fixture.operation.operation, 'diffusion');
		fieldFixtures.push(...fixtures);
	}
	const temporary = await mkdtemp(path.join(tmpdir(), 'ditherette-public-conformance-'));
	const processPath = process.env.DITHERETTE_BENCH_PROCESS_FIXTURES;
	const processFixtures = processPath ? JSON.parse(await readFile(processPath, 'utf8')) : [];
	const originalProcessFixtures = createHash('sha256')
		.update(JSON.stringify(processFixtures))
		.digest('hex');
	if (processPath) {
		assert.equal(processFixtures.length, 8);
		for (const fixture of processFixtures) {
			assert.equal(fixture.operation.operation, 'process');
			assert.deepEqual(fixture.production, fixture.staged);
		}
	}
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
		'benchmark-stage-cache.mjs',
		'benchmark-public-browser.mjs',
		'benchmark-oracle-page.mjs',
		'benchmark-transport.mjs'
	])
		await cp(fileURLToPath(new URL(name, import.meta.url)), path.join(root, 'scripts', name));
	await cp(oracle, path.join(root, 'scripts/oracle'), { recursive: true });
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
				const references = [];
				for (const fixture of [...quantizeFixtures, ...fieldFixtures, ...processFixtures]) {
					assert.ok(
						fixture.identity,
						'Generate full-identity conformance fixtures with the current exporter.'
					);
					const reference = await frozenBrowserReference(browser, {
						browser: { assets, runtime: { cross_origin_isolated: true } },
						case: {
							source: fixture.source,
							rgba: fixture.rgba,
							identity: fixture.identity,
							browser: {
								operation: fixture.operation ?? {
									operation: 'quantize',
									settings: fixture.settings
								}
							},
							measurement: { samples: 1 }
						}
					});
					assert.equal(
						browser.contexts().length,
						0,
						'Oracle context closes before any package initialization.'
					);
					assert.deepEqual(reference.case, fixture.identity);
					references.push({
						...fixture,
						resize_attribution: structuredClone(fixture.resize_attribution),
						native_reference: fixture.reference,
						reference: reference.output
					});
				}
				for (const fixture of references.filter((fixture) => fixture.resize_attribution)) {
					for (const key of ['resize_probe', 'post_resize_probe']) {
						const probe = fixture.resize_attribution[key];
						const reference = await frozenBrowserReference(browser, {
							browser: { assets, runtime: { cross_origin_isolated: true } },
							case: {
								source: probe.source,
								rgba: probe.rgba,
								identity: probe.identity,
								browser: { operation: probe.operation },
								measurement: { samples: 1 }
							}
						});
						assert.deepEqual(reference.case, probe.identity);
						probe.native_reference = probe.reference;
						probe.reference = reference.output;
					}
					assert.equal(browser.contexts().length, 0);
				}
				assert.equal(
					createHash('sha256').update(JSON.stringify(processFixtures)).digest('hex'),
					originalProcessFixtures,
					'Each engine leaves native attribution fixtures unchanged.'
				);
				if (process.env.DITHERETTE_BENCH_ORACLE_EVIDENCE)
					await writeFile(
						path.join(
							process.env.DITHERETTE_BENCH_ORACLE_EVIDENCE,
							`${engineName}-references.json`
						),
						JSON.stringify(
							{
								engine: engineName,
								version: browser.version(),
								oracle_manifest: JSON.parse(
									await readFile(path.join(oracle, 'manifest.json'), 'utf8')
								),
								tarball_sha256: createHash('sha256')
									.update(await readFile(tarball))
									.digest('hex'),
								references
							},
							null,
							2
						),
						{ flag: 'wx' }
					);
				const context = await browser.newContext({ serviceWorkers: 'block' });
				await restrictContext(context, server);
				const page = await context.newPage();
				await page.goto(server.url);
				const report = await page.evaluate(
					async ({ assets, quantizeFixtures, fieldFixtures, processFixtures }) => {
						const { prepareOperation, preflightOperation, outputStability, verificationOutput } =
							await import(`/${assets.entries.page}`);
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
						for (const fixture of fieldFixtures) {
							for (const preparation of ['primed-instance', 'fresh-instance']) {
								const request = trial('package', preparation);
								request.case.source = fixture.source;
								request.case.rgba = fixture.rgba;
								request.case.identity.output = fixture.source;
								request.case.browser.operation = fixture.operation;
								const operation = await prepareOperation(request);
								try {
									const indexed = fixture.operation.operation !== 'perturb';
									const pixels = fixture.source.width * fixture.source.height;
									const stability = outputStability(
										indexed ? pixels + 1024 : pixels * 4,
										indexed ? 'indexed8' : 'rgba8'
									);
									equal(
										await preflightOperation(operation, fixture.reference, stability.observe),
										undefined,
										`${fixture.name} ${preparation}`
									);
									const one = await operation.prepare();
									let first;
									try {
										first = one.call();
										stability.observe([first]);
									} finally {
										one.close();
									}
									const retained = verificationOutput(first);
									const two = await operation.prepare();
									try {
										stability.observe([two.call()]);
									} finally {
										two.close();
									}
									equal(
										verificationOutput(first),
										retained,
										'durable field output after later call/disposal'
									);
									equal(
										Array.from(operation.request.source.data),
										fixture.rgba,
										'field source preservation'
									);
									if (!indexed) {
										equal(
											Array.from(first.data).filter((_, i) => i % 4 === 3),
											fixture.rgba.filter((_, i) => i % 4 === 3),
											'perturb alpha preservation'
										);
										continue;
									}
									if (fixture.operation.operation !== 'separable') continue;
									// Both composition steps use actual package adapters, outside every timer.
									const perturbTrial = structuredClone(request);
									perturbTrial.case.browser.operation = {
										operation: 'perturb',
										settings: fixture.operation.settings.perturb
									};
									const perturb = await prepareOperation(perturbTrial);
									let rgba;
									try {
										const prepared = await perturb.prepare();
										try {
											rgba = prepared.call();
										} finally {
											prepared.close();
										}
									} finally {
										perturb.close();
									}
									const quantizeTrial = structuredClone(request);
									quantizeTrial.case.rgba = Array.from(rgba.data);
									quantizeTrial.case.browser.operation = {
										operation: 'quantize',
										settings: fixture.operation.settings.quantize
									};
									const quantize = await prepareOperation(quantizeTrial);
									try {
										equal(
											await preflightOperation(quantize, retained),
											undefined,
											`${fixture.name} actual quantize(perturb) metadata and bytes`
										);
									} finally {
										quantize.close();
									}
								} finally {
									operation.close();
								}
							}
						}
						for (const fixture of [fieldFixtures[0], fieldFixtures[7]]) {
							const request = trial('typescript');
							request.case.browser.operation = fixture.operation;
							let rejected = false;
							try {
								await prepareOperation(request);
							} catch (error) {
								rejected = error.message.includes('No faithful TypeScript field adapter');
							}
							if (!rejected) throw new Error('TypeScript field adapter must be unavailable.');
						}
						const processDiagnostics = [];
						for (const fixture of processFixtures) {
							let resized;
							if (fixture.resize_attribution) {
								const probe = fixture.resize_attribution.resize_probe;
								const request = trial('package', 'primed-instance');
								Object.assign(request.case, {
									source: probe.source,
									rgba: probe.rgba,
									identity: probe.identity
								});
								request.case.browser.operation = probe.operation;
								const operation = await prepareOperation(request);
								try {
									const call = await operation.prepare();
									try {
										resized = verificationOutput(call.call());
									} finally {
										call.close();
									}
								} finally {
									operation.close();
								}
							}
							for (const preparation of ['primed-instance', 'fresh-instance']) {
								const outputs = [];
								for (const backend of ['package-staged', 'package']) {
									const request = trial(backend, preparation);
									request.case.source = fixture.source;
									request.case.rgba = fixture.rgba;
									request.case.identity = fixture.identity;
									request.case.browser.operation = fixture.operation;
									const operation = await prepareOperation(request);
									try {
										const call = await operation.prepare();
										try {
											outputs.push(verificationOutput(call.call()));
										} finally {
											call.close();
										}
									} finally {
										operation.close();
									}
								}
								processDiagnostics.push({
									name: fixture.name,
									preparation,
									identity: fixture.identity,
									frozen: fixture.reference,
									native_frozen: fixture.native_reference,
									resize_attribution: fixture.resize_attribution
										? { ...fixture.resize_attribution, browser_resize: resized }
										: null,
									staged: outputs[0],
									process: outputs[1]
								});
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
							processDiagnostics,
							drift,
							checked: [
								'known-vector',
								'area-bilinear-known-vectors-and-drift',
								'convolution-support-recipes',
								'47-frozen-quantize-fixtures-all-15-modes-primed-and-fresh',
								`${fieldFixtures.length}-frozen-processing-fixtures-primed-and-fresh`,
								`${fieldFixtures.filter((fixture) => fixture.operation.operation === 'separable').length * 2}-actual-quantize-perturb-compositions-including-warning-metadata`,
								...(fieldFixtures.some((fixture) => fixture.name.startsWith('blue-noise'))
									? ['blue-noise-65x33-tile-boundaries']
									: []),
								'identity-copy',
								'fresh-instance',
								'initialization-bytes',
								'initialization-compiled',
								'typescript-drift',
								'noncenter-unavailable'
							]
						};
					},
					{
						assets,
						quantizeFixtures: references.slice(0, quantizeFixtures.length),
						fieldFixtures: references.slice(
							quantizeFixtures.length,
							quantizeFixtures.length + fieldFixtures.length
						),
						processFixtures: references.slice(quantizeFixtures.length + fieldFixtures.length)
					}
				);
				assert.equal(report.isolated, true);
				if (processPath) {
					assert.ok(
						process.env.DITHERETTE_BENCH_ORACLE_EVIDENCE,
						'Retain Process composition and frozen diagnostics before checking equality.'
					);
					await writeFile(
						path.join(
							process.env.DITHERETTE_BENCH_ORACLE_EVIDENCE,
							`${engineName}-process-composition.json`
						),
						JSON.stringify(report.processDiagnostics, null, 2),
						{ flag: 'wx' }
					);
					for (const result of report.processDiagnostics) {
						assert.deepEqual(
							result.process,
							result.staged,
							`${result.name}: Process must equal actual staged production`
						);
						if (result.resize_attribution) {
							assert.equal(
								result.name,
								'area-bayer',
								'Only area has diagnostic measurement approval.'
							);
							const attribution = result.resize_attribution;
							assert.deepEqual(
								attribution.browser_resize,
								attribution.production_resize,
								'Browser resize must match the independently identified post-resize oracle input. Otherwise retain and stop.'
							);
							assert.deepEqual(
								result.process,
								attribution.post_resize_probe.reference,
								'Target-local frozen processing on actual resized bytes must explain every output difference.'
							);
						} else {
							assert.deepEqual(
								result.process,
								result.frozen,
								`${result.name}: unapproved frozen drift`
							);
						}
					}
				}
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

test('Process composition preflight rejects differing actual outputs before timing', async () => {
	let closed = 0;
	const operation = {
		prepare: async () => ({
			call: () => ({ width: 1, height: 1, data: new Uint8Array([1, 2, 3, 255]) }),
			close: () => {
				closed++;
			}
		})
	};
	const output = {
		dimensions: { width: 1, height: 1 },
		pixels: { format: 'rgba8', data: [1, 2, 3, 255] },
		warnings: []
	};
	await requireMatchingComposition(operation, output);
	await assert.rejects(
		requireMatchingComposition(operation, {
			...output,
			pixels: { format: 'rgba8', data: [0, 2, 3, 255] }
		}),
		/Process differs from staged production before timing.*current.*counterpart/
	);
	assert.equal(closed, 2);
});
