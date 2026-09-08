import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { workerLifetimePrelude, workerLifetimeEpilogue } from './thread-worker-observer.mjs';

export const threadTestAssets = Object.fromEntries(
	['thread-worker-observer.mjs', 'thread-lifetime-worker.mjs'].map((name) => [
		`__tests__/${name}`,
		fileURLToPath(new URL(name, import.meta.url))
	])
);

const observedServers = new WeakSet();

/** Serve temporary instrumented copies because Firefox does not intercept nested worker requests. */
export async function prepareThreadServer(server, t) {
	if (observedServers.has(server)) return;
	observedServers.add(server);
	const directory = await mkdtemp(join(tmpdir(), 'ditherette-worker-observer-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	server.instance.prependListener('request', (_request, response) => {
		const setHeader = response.setHeader.bind(response);
		response.setHeader = (name, value) =>
			setHeader(
				name,
				name.toLowerCase() === 'content-security-policy'
					? "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self' blob:; connect-src 'self'"
					: value
			);
	});
	for (const path of ['dist/thread-worker.js', '__tests__/thread-lifetime-worker.mjs']) {
		if (!server.paths.has(path)) continue;
		const original = await readFile(server.paths.get(path), 'utf8');
		const instrumented = join(directory, path.replaceAll('/', '-'));
		await writeFile(instrumented, workerLifetimePrelude + original + workerLifetimeEpilogue);
		server.paths.set(path, instrumented);
	}
}

export async function heldWorkers(page, prefix = 'ditherette-test-') {
	return page.evaluate(async (prefix) => {
		const { held } = await navigator.locks.query();
		return held
			.map(({ name }) => name)
			.filter((name) => name.startsWith(prefix))
			.sort();
	}, prefix);
}

export async function waitForWorkers(page, count, prefix = 'ditherette-test-') {
	return page.evaluate(
		async ({ count, prefix }) => {
			const deadline = performance.now() + 10_000;
			for (;;) {
				const { held } = await navigator.locks.query();
				const names = held
					.map(({ name }) => name)
					.filter((name) => name.startsWith(prefix))
					.sort();
				if (names.length === count) return names;
				if (performance.now() >= deadline)
					throw new Error(`Expected ${count} held worker locks, found ${JSON.stringify(names)}`);
				await new Promise(requestAnimationFrame);
			}
		},
		{ count, prefix }
	);
}

export async function lifetimeObserverChecks({ page, server, t }) {
	await prepareThreadServer(server, t);
	await page.goto(server.url);
	await page.evaluate(() => {
		globalThis.lifetimeWorker = new Worker('/__tests__/thread-lifetime-worker.mjs', {
			type: 'module',
			name: 'ditherette-test-observer'
		});
		lifetimeWorker.onmessage = () => {
			globalThis.lifetimeReady = true;
		};
		lifetimeWorker.onerror = (event) => {
			globalThis.lifetimeError = event.message;
		};
		lifetimeWorker.postMessage({ child: true });
	});
	await page.waitForFunction(
		() => globalThis.lifetimeReady || globalThis.lifetimeError,
		undefined,
		{ timeout: 10_000 }
	);
	assert.equal(await page.evaluate(() => globalThis.lifetimeError), undefined);
	await waitForWorkers(page, 2);
	assert.deepEqual(await heldWorkers(page), [
		'ditherette-test-observer',
		'ditherette-test-observer-child'
	]);
	await page.evaluate(() => lifetimeWorker.terminate());
	await waitForWorkers(page, 0);
	return { actualWorkers: 2, releasedAfterHostTermination: 2 };
}
