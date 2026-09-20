import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, readdir, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { browserEngines, browserLaunchOptions } from './browser-engines.mjs';
import { startAssetServer, restrictContext } from '../../../scripts/benchmark-public-browser.mjs';

/** Install an explicit tarball and run one caller-visible fixture in each browser engine. */
export async function installedBrowserChecks(t, check, expected, options = {}) {
	const directory = await mkdtemp(join(tmpdir(), 'ditherette-stage-ownership-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const tarball = process.env.DITHERETTE_TEST_TARBALL
		? resolve(process.env.DITHERETTE_TEST_TARBALL)
		: join(directory, 'ditherette.tgz');
	if (!process.env.DITHERETTE_TEST_TARBALL)
		execFileSync('pnpm', ['pack', '--out', tarball], {
			cwd: fileURLToPath(new URL('../', import.meta.url)),
			stdio: 'pipe'
		});
	const digest = createHash('sha256')
		.update(await readFile(tarball))
		.digest('hex');
	if (process.env.DITHERETTE_TEST_TARBALL_SHA256)
		assert.equal(digest, process.env.DITHERETTE_TEST_TARBALL_SHA256);
	t.diagnostic(`Installed tarball SHA-256 ${digest}; behavioral checks do not prove cache hits.`);
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
	execFileSync(
		'pnpm',
		['install', '--offline', '--ignore-scripts', '--ignore-workspace', '--lockfile=false'],
		{
			cwd: consumer,
			stdio: 'pipe'
		}
	);
	const packagePath = await realpath(join(consumer, 'node_modules/ditherette'));
	const files = (await readdir(packagePath, { recursive: true, withFileTypes: true }))
		.filter((entry) => entry.isFile())
		.map((entry) => ({ path: join(entry.parentPath, entry.name).slice(packagePath.length + 1) }));
	const server = await startAssetServer(
		{ tree: { root: packagePath, files }, entries: {} },
		options.isolated ?? false
	);
	for (const [route, file] of Object.entries(options.assets ?? {})) server.paths.set(route, file);
	t.after(
		() =>
			new Promise((resolve) => {
				server.instance.closeAllConnections();
				server.instance.close(resolve);
			})
	);
	const vectors = JSON.parse(await readFile(new URL('./fixtures/fields.json', import.meta.url)));
	for (const [name, engine] of browserEngines()) {
		await t.test(name, async () => {
			const browser = await engine.launch(browserLaunchOptions(name));
			try {
				const context = await browser.newContext();
				await restrictContext(context, server);
				const page = await context.newPage();
				const input = {
					moduleUrl: `${server.url}/dist/index.js`,
					wasmUrl: `${server.url}/dist/wasm/scalar/ditherette_wasm_bg.wasm`,
					vectors
				};
				assert.deepEqual(
					options.driver
						? await options.driver({ page, context, server, input, t })
						: await (async () => {
								await page.goto(server.url);
								return page.evaluate(check, input);
							})(),
					expected
				);
				t.diagnostic(`${name} ${browser.version()}: installed fixture passes`);
			} finally {
				await browser.close();
			}
		});
	}
	assert.deepEqual(server.failures, []);
}
