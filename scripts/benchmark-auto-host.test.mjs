import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { chromium, firefox } from 'playwright';
import { automaticCases, automaticTrial } from './benchmark-auto-host-cases.mjs';
import { runTrial } from './benchmark-auto-host-fixture.mjs';
import { events, reset } from './benchmark-stage-cache-fixture.mjs';
import { stageHostAssets } from './benchmark-host-assets.mjs';
import { equalOutput } from './benchmark-public-page.mjs';
import { exchangeTrial, restrictContext, startAssetServer } from './benchmark-public-browser.mjs';

const bundlePath = process.env.DITHERETTE_BENCH_AUTO_BUNDLE;
const wasmEntry = (variant) => `package/dist/wasm/${variant}/ditherette_wasm_bg.wasm`;

test('automatic trial roles use ordinary initialization and no developer override', () => {
	for (const role of ['accepted', 'candidate']) {
		const trial = automaticTrial({}, automaticCases.at(-1), role);
		assert.equal(trial.case.browser.threads[role], role === 'accepted' ? 'disabled' : 'required');
		assert.equal(Object.hasOwn(trial.case.browser, 'row_policy'), false);
		assert.equal(trial.case.browser[role], 'package');
		assert.equal(trial.case.rgba.length, 65 * 49 * 4);
		assert.deepEqual(trial.case.rgba.slice(0, 8), [0, 0, 0, 255, 17, 43, 11, 255]);
	}
});

test('untimed automatic fixture uses compiled ordinary calls and actual staged composition', async (t) => {
	for (const [name, value] of Object.entries({
		location: { href: 'file:///' },
		fetch: async () => new Response(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])),
		DedicatedWorkerGlobalScope: Object,
		crossOriginIsolated: true,
		Worker: class {}
	})) {
		const previous = Object.getOwnPropertyDescriptor(globalThis, name);
		Object.defineProperty(globalThis, name, { configurable: true, writable: true, value });
		t.after(() => {
			if (previous) Object.defineProperty(globalThis, name, previous);
			else delete globalThis[name];
		});
	}
	const fixture = {
		...automaticCases.find((item) => item.composition),
		source: { width: 1, height: 1 },
		output: { width: 1, height: 1 }
	};
	const assets = {
		entries: {
			package: fileURLToPath(new URL('./benchmark-stage-cache-fixture.mjs', import.meta.url)).slice(
				1
			),
			wasm: 'empty.wasm'
		}
	};
	for (const role of ['accepted', 'candidate']) {
		reset();
		const result = await runTrial(automaticTrial(assets, fixture, role));
		assert.ok(equalOutput(result.output, result.composition));
		assert.equal(result.instantiation_restored, true);
		assert.equal(Object.hasOwn(result, 'sample_ns'), false);
		assert.deepEqual(
			events
				.filter((event) => ['ditherAndQuantize', 'perturb', 'quantize'].includes(event.type))
				.map((event) => event.type),
			['ditherAndQuantize', 'perturb', 'quantize']
		);
		assert.deepEqual(
			events.filter((event) => event.type === 'initialize').map((event) => event.wasm),
			['bytes', 'compiled', 'bytes', 'compiled', 'bytes', 'compiled']
		);
		assert.ok(
			events
				.filter((event) => event.type === 'initialize')
				.every((event) => event.threads === (role === 'accepted' ? 'disabled' : 'required'))
		);
		assert.deepEqual(
			events.filter((event) => event.type === 'create').map((event) => event.id),
			events.filter((event) => event.type === 'dispose').map((event) => event.id)
		);
	}
});

test(
	'ordinary installed automatic policies match scalar public calls in Chromium and Firefox hosts',
	{
		skip:
			!bundlePath && 'Set DITHERETTE_BENCH_AUTO_BUNDLE to an ordinary public bundle-source.json.',
		timeout: 600_000
	},
	async (t) => {
		const bundle = JSON.parse(await readFile(bundlePath, 'utf8'));
		const provenance = JSON.parse(await readFile(bundle.provenance, 'utf8'));
		assert.equal(provenance.build_mode, 'public');
		// Both real package variants must exclude the developer override ABI.
		for (const variant of ['scalar', 'threads']) {
			const module = await WebAssembly.compile(
				await readFile(path.join(bundle.package, `dist/wasm/${variant}/ditherette_wasm_bg.wasm`))
			);
			assert.equal(
				WebAssembly.Module.exports(module).some((entry) => entry.name === 'privateExecutionPolicy'),
				false
			);
		}
		const root = await mkdtemp(path.join(tmpdir(), 'ditherette-auto-host-'));
		t.after(() => rm(root, { recursive: true, force: true }));
		const assets = await stageHostAssets(
			bundle,
			root,
			'benchmark-auto-host-fixture.mjs',
			wasmEntry('scalar')
		);
		for (const [engineName, engine] of Object.entries({ chromium, firefox })) {
			await t.test(engineName, async (engineTest) => {
				const browser = await engine.launch({
					headless: true,
					executablePath:
						process.env[`DITHERETTE_BENCH_AUTO_${engineName.toUpperCase()}_EXECUTABLE`]
				});
				try {
					for (const fixture of automaticCases) {
						await engineTest.test(fixture.name, async () => {
							const results = [];
							for (const role of ['accepted', 'candidate']) {
								const roleAssets = {
									...assets,
									entries: {
										...assets.entries,
										wasm: wasmEntry(role === 'accepted' ? 'scalar' : 'threads')
									}
								};
								const trial = automaticTrial(roleAssets, fixture, role);
								const server = await startAssetServer(roleAssets, true, trial);
								let context;
								try {
									context = await browser.newContext({ serviceWorkers: 'block' });
									await restrictContext(context, server);
									const page = await context.newPage();
									await page.goto(server.url);
									const result = await exchangeTrial(
										page,
										server,
										'scripts/benchmark-auto-host-fixture.mjs',
										'host-worker'
									);
									assert.deepEqual(server.failures, []);
									assert.equal(result.execution, 'host-worker');
									assert.equal(result.isolated, true);
									assert.equal(result.instantiation_restored, true);
									assert.equal(
										result.created,
										result.terminated,
										'every initialized pool is disposed'
									);
									if (role === 'accepted') assert.equal(result.created, 0);
									else
										assert.ok(
											result.created >= 4,
											'required initialization must create at least two workers per instance'
										);
									assert.equal(result.events[0].stage, 'prepare');
									assert.equal(result.events.at(-1).stage, 'complete');
									for (const stage of fixture.stages)
										assert.ok(
											result.events.some((event) => event.stage === stage),
											`missing ${stage}`
										);
									assert.deepEqual(result.output.dimensions, fixture.output);
									if (fixture.composition)
										assert.ok(
											equalOutput(result.output, result.composition),
											'actual quantize(perturb) composition'
										);
									assert.equal(Object.hasOwn(result, 'sample_ns'), false);
									results.push(result.output);
								} finally {
									await context?.close();
									server.instance.closeAllConnections();
									await new Promise((resolve) => server.instance.close(resolve));
								}
							}
							// A bounded error avoids dumping megabytes of pixel arrays into the collector.
							assert.ok(
								equalOutput(results[0], results[1]),
								`${fixture.name}: automatic/scalar bytes or metadata differ`
							);
						});
					}
				} finally {
					await browser.close();
				}
			});
		}
	}
);
