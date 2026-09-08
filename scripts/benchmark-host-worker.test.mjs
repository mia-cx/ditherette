import assert from 'node:assert/strict';
import { cp, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { chromium } from 'playwright';
import { exchangeTrial, restrictContext, startAssetServer } from './benchmark-public-browser.mjs';

const reference = {
	dimensions: { width: 1, height: 1 },
	pixels: { format: 'rgba8', data: [11, 23, 47, 127] },
	warnings: []
};

async function hostFixture(fake, check) {
	const root = await mkdtemp(path.join(tmpdir(), 'ditherette-host-protocol-'));
	let browser;
	try {
		await mkdir(path.join(root, 'scripts'));
		for (const name of [
			'benchmark-host-worker',
			'benchmark-host-fixture',
			'benchmark-public-page',
			'benchmark-public-timing',
			'benchmark-stage-cache',
			'benchmark-progress',
			'benchmark-row-policy',
			'benchmark-stage-cache-fixture'
		])
			await cp(new URL(`./${name}.mjs`, import.meta.url), path.join(root, `scripts/${name}.mjs`));
		if (fake)
			await writeFile(path.join(root, 'empty.wasm'), new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]));
		else {
			const source = JSON.parse(
				await readFile(process.env.DITHERETTE_BENCH_STARTUP_BUNDLE, 'utf8')
			);
			await cp(source.package, path.join(root, 'package'), { recursive: true, dereference: true });
		}
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
			entries: {
				package: fake ? 'scripts/benchmark-stage-cache-fixture.mjs' : 'package/dist/index.js',
				wasm: fake ? 'empty.wasm' : 'package/dist/wasm/threads/ditherette_wasm_bg.wasm'
			}
		};
		browser = await chromium.launch({ headless: true });
		for (const preparation of ['initialization-bytes', 'initialization-compiled']) {
			const trial = {
				fake,
				role: 'candidate',
				pair: 0,
				browser: { assets },
				reference_output: reference,
				case: {
					name: 'host-startup-fixture',
					source: reference.dimensions,
					rgba: reference.pixels.data,
					identity: {
						input: Array(32).fill(1),
						settings: Array(32).fill(2),
						output: reference.dimensions
					},
					measurement: {
						scope: 'initialization',
						mode: 'single-call',
						application_cache: 'not-applicable',
						warmup_ms: 1,
						samples: 5,
						measurement_ms: 10
					},
					browser: {
						execution: 'host-worker',
						preparation,
						accepted: 'package',
						candidate: 'package',
						threads: { accepted: 'required', candidate: 'required' },
						cache: 'none',
						operation: { operation: 'resize-nearest', anchor: 'center' }
					}
				}
			};
			await check(async (configure = () => {}) => {
				configure(trial);
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
						'scripts/benchmark-host-fixture.mjs',
						'host-worker'
					);
					assert.deepEqual(server.failures, []);
					return result;
				} finally {
					await context?.close();
					server.instance.closeAllConnections();
					await new Promise((resolve) => server.instance.close(resolve));
				}
			});
		}
	} finally {
		await browser?.close();
		await rm(root, { recursive: true, force: true });
	}
}

test('host exchange reuses the actual collector with a fake clock and propagates setup failure', async () => {
	await hostFixture(true, async (run) => {
		const result = await run();
		assert.deepEqual(result.output, reference);
		assert.equal(result.observation.execution, 'host-worker');
		assert.deepEqual(result.sample_ns, Array(5).fill(1e6));
		const events = result.fixture_events;
		assert.deepEqual(
			events.filter((e) => e.type === 'create').map((e) => e.id),
			events.filter((e) => e.type === 'dispose').map((e) => e.id)
		);
		assert.ok(events.filter((e) => e.type === 'initialize').every((e) => e.threads === 'required'));
		await assert.rejects(
			run((trial) => {
				trial.fail = true;
			}),
			/injected required startup failure/
		);
	});
});

test(
	'installed package repeatedly initializes required pools inside the real host without timers',
	{
		skip: !process.env.DITHERETTE_BENCH_STARTUP_BUNDLE,
		timeout: 60_000
	},
	async () => {
		await hostFixture(false, async (run) => {
			const result = await run();
			assert.deepEqual(result.outputs, Array(3).fill(reference));
			assert.ok(result.created >= 4);
			assert.equal(result.created, result.terminated);
		});
	}
);
