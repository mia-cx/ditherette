import assert from 'node:assert/strict';
import { cp, mkdir, mkdtemp, readFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { chromium, firefox } from 'playwright';
import { exchangeTrial, restrictContext, startAssetServer } from './benchmark-public-browser.mjs';

const bundlePath = process.env.DITHERETTE_BENCH_ROW_BUNDLE;
const source = { width: 11, height: 9 };
const smaller = { width: 7, height: 5 };
const rgba = Array.from(
	{ length: source.width * source.height * 4 },
	(_, index) => (index * 73 + Math.floor(index / 7) * 19) % 256
);
const quantize = {
	palette: [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] },
		{ kind: 'color', rgb: [255, 0, 0] },
		{ kind: 'transparent' }
	],
	alpha: { mode: 'preserve', threshold: 128 },
	matching: 'srgb-euclidean'
};
const perturb = {
	field: { algorithm: 'random', seed: 0xffffffff },
	space: 'srgb',
	strength: 0.5,
	placement: { mode: 'everywhere' }
};
const fixtures = [
	{
		operation: { operation: 'resize-area' },
		output: smaller,
		policy: 'resize',
		stages: ['resize']
	},
	{
		operation: { operation: 'resize-lanczos3', anchor: 'bottom-right', support: 'scale-aware' },
		output: smaller,
		policy: 'resize',
		stages: ['resize']
	},
	{
		operation: { operation: 'perturb', settings: perturb },
		output: source,
		policy: 'indexed',
		stages: ['perturb']
	},
	{
		operation: { operation: 'quantize', settings: quantize },
		output: source,
		policy: 'indexed',
		stages: ['quantize']
	},
	{
		operation: { operation: 'separable', settings: { quantize, perturb } },
		output: source,
		policy: 'indexed',
		stages: ['perturb', 'quantize']
	},
	{
		operation: {
			operation: 'yliluoma',
			settings: { quantize, size: '4', placement: { mode: 'everywhere' } }
		},
		output: source,
		policy: 'mixing',
		stages: ['dither-and-quantize']
	},
	{
		operation: {
			operation: 'process',
			settings: {
				palette: quantize.palette,
				recipe: {
					version: 1,
					output: { ...smaller, resize: { algorithm: 'bilinear', anchor: 'center' } },
					alpha: quantize.alpha,
					match: quantize.matching,
					dither: { family: 'separable', perturb }
				}
			}
		},
		output: smaller,
		policy: 'indexed',
		stages: ['resize', 'perturb', 'quantize']
	}
];

function trialFor(assets, fixture, role) {
	return {
		role,
		pair: 0,
		browser: { assets },
		case: {
			name: `untimed-row-host-${fixture.operation.operation}`,
			source,
			rgba,
			identity: { input: Array(32).fill(1), settings: Array(32).fill(2), output: fixture.output },
			measurement: {
				scope: 'complete-call',
				mode: 'single-call',
				application_cache: 'not-applicable',
				samples: 1
			},
			browser: {
				execution: 'host-worker',
				preparation: 'primed-instance',
				cache: 'none',
				accepted: 'package',
				candidate: 'package',
				threads: { accepted: 'required', candidate: 'required' },
				row_policy: {
					stage: fixture.policy,
					accepted: { height: 0, active_workers: 2 },
					candidate: { height: 2, active_workers: 2 }
				},
				operation: fixture.operation
			}
		}
	};
}

test(
	'installed row candidates preserve complete public calls in real Chromium and Firefox hosts, without timings',
	{
		skip:
			!bundlePath &&
			'Set DITHERETTE_BENCH_ROW_BUNDLE to a prepared --bench-subjects bundle-source.json.',
		timeout: 180_000
	},
	async (t) => {
		const bundle = JSON.parse(await readFile(bundlePath, 'utf8'));
		const provenance = JSON.parse(await readFile(bundle.provenance, 'utf8'));
		assert.equal(provenance.build_mode, 'bench-subjects');
		const wasmEntry = 'package/dist/wasm/threads/ditherette_wasm_bg.wasm';
		// Inspect the real raw ABI; do not substitute guessed glue names.
		const module = await WebAssembly.compile(
			await readFile(path.join(bundle.package, 'dist/wasm/threads/ditherette_wasm_bg.wasm'))
		);
		const exports = WebAssembly.Module.exports(module);
		for (const name of ['privateExecutionPolicy', 'privateThreadCount'])
			assert.ok(
				exports.some((entry) => entry.name === name && entry.kind === 'function'),
				`Missing raw export ${name}`
			);
		const root = await mkdtemp(path.join(tmpdir(), 'ditherette-row-host-'));
		t.after(() => rm(root, { recursive: true, force: true }));
		await cp(bundle.package, path.join(root, 'package'), { recursive: true, dereference: true });
		await mkdir(path.join(root, 'scripts'));
		for (const name of [
			'benchmark-host-worker',
			'benchmark-public-page',
			'benchmark-public-timing',
			'benchmark-stage-cache',
			'benchmark-progress',
			'benchmark-row-policy'
		])
			await cp(path.join(bundle.scripts, `${name}.mjs`), path.join(root, `scripts/${name}.mjs`));
		await cp(
			new URL('./benchmark-row-host-fixture.mjs', import.meta.url),
			path.join(root, 'scripts/benchmark-row-host-fixture.mjs')
		);
		const files = [];
		async function walk(relative = '') {
			for (const entry of await readdir(path.join(root, relative), { withFileTypes: true })) {
				const name = path.posix.join(relative, entry.name);
				if (entry.isDirectory()) await walk(name);
				else files.push({ path: name });
			}
		}
		await walk();
		const assets = {
			tree: { root, files },
			entries: { package: 'package/dist/index.js', wasm: wasmEntry }
		};
		for (const [engineName, engine] of Object.entries({ chromium, firefox })) {
			await t.test(engineName, async () => {
				const browser = await engine.launch({ headless: true });
				try {
					for (const fixture of fixtures) {
						const results = [];
						for (const role of ['accepted', 'candidate']) {
							const trial = trialFor(assets, fixture, role);
							const server = await startAssetServer(assets, true, trial);
							let context;
							try {
								context = await browser.newContext({ serviceWorkers: 'block' });
								await restrictContext(context, server);
								const page = await context.newPage();
								await page.goto(server.url);
								const result = await exchangeTrial(
									page,
									server,
									'scripts/benchmark-row-host-fixture.mjs',
									'host-worker'
								);
								assert.deepEqual(server.failures, []);
								assert.equal(result.execution, 'host-worker');
								assert.equal(result.isolated, true);
								assert.equal(result.instantiation_restored, true);
								assert.equal(result.row_policy.stage, fixture.policy);
								assert.deepEqual(result.row_policy.parameters, trial.case.browser.row_policy[role]);
								assert.ok(result.row_policy.pool_size >= 2 && result.row_policy.pool_size <= 8);
								assert.deepEqual(result.failure, { code: 'callback', path: 'onProgress' });
								for (const events of [
									result.first_events,
									result.failed_events,
									result.recovery_events
								]) {
									assert.equal(events[0].stage, 'prepare');
									assert.equal(events.at(-1).stage, 'complete');
									for (const stage of fixture.stages)
										assert.ok(
											events.some((event) => event.stage === stage),
											`${fixture.operation.operation} missed ${stage}`
										);
								}
								assert.deepEqual(result.recovered, result.repeated);
								assert.deepEqual(result.original_after_calls, result.output);
								assert.equal(Object.hasOwn(result, 'sample_ns'), false);
								results.push(result);
							} finally {
								await context?.close();
								server.instance.closeAllConnections();
								await new Promise((resolve) => server.instance.close(resolve));
							}
						}
						assert.deepEqual(results[1].output, results[0].output, fixture.operation.operation);
						assert.deepEqual(
							results[1].recovered,
							results[0].recovered,
							`${fixture.operation.operation} recovery`
						);
					}
				} finally {
					await browser.close();
				}
			});
		}
	}
);
