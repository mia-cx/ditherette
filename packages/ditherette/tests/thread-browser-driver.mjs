import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { workerLifetimePrelude, workerLifetimeEpilogue } from './thread-worker-observer.mjs';
import {
	scalarSelectionChecks,
	initializeThreadedPair,
	exerciseThreadedPair,
	customThreadedInputs
} from './threads-browser-fixture.mjs';

export const threadTestAssets = Object.fromEntries(
	['thread-worker-observer.mjs', 'thread-lifetime-worker.mjs', 'thread-processing-host.mjs'].map(
		(name) => [`__tests__/${name}`, fileURLToPath(new URL(name, import.meta.url))]
	)
);

const observedServers = new WeakSet();

/** Serve temporary instrumented copies because Firefox does not intercept nested worker requests. */
export async function prepareThreadServer(server, t, context) {
	await context.route(`blob:${server.url}/*`, (route) => route.continue());
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
	for (const path of [
		'dist/thread-worker.js',
		'__tests__/thread-lifetime-worker.mjs',
		'__tests__/thread-processing-host.mjs'
	]) {
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

export async function lifetimeObserverChecks({ page, server, t, context }) {
	await prepareThreadServer(server, t, context);
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

export async function scalarSelectionDriver({ page, server, input }) {
	const requests = [];
	const record = (request) => requests.push(request.url);
	server.instance.on('request', record);
	try {
		await page.goto(server.url);
		await page.evaluate(async ({ moduleUrl }) => {
			globalThis.workerCount = 0;
			const OriginalWorker = Worker;
			globalThis.Worker = class extends OriginalWorker {
				constructor(...args) {
					super(...args);
					globalThis.workerCount++;
				}
			};
			await import(moduleUrl);
		}, input);
		assert.equal(
			requests.some((url) => url.endsWith('.wasm')),
			false,
			'root import is inert'
		);
		const result = await page.evaluate(scalarSelectionChecks, input);
		assert.equal(await page.evaluate(() => globalThis.workerCount), 0);
		assert.equal(
			requests.some((url) =>
				/\/threads(?:\/|\.js)|\/thread-worker\.js|\/worker-pool\.js/.test(url)
			),
			false,
			'scalar selection must not import or fetch threaded assets'
		);
		return result;
	} finally {
		server.instance.off('request', record);
	}
}

export async function threadedOwnershipDriver({ page, server, input, t, context }) {
	await prepareThreadServer(server, t, context);
	await page.goto(server.url);
	const threadedInput = { ...input, wasmUrl: input.wasmUrl.replace('/scalar/', '/threads/') };
	const { firstWorkers, totalWorkers } = await page.evaluate(initializeThreadedPair, threadedInput);
	try {
		await waitForWorkers(page, totalWorkers);
		const result = await page.evaluate(exerciseThreadedPair);
		await waitForWorkers(page, totalWorkers - firstWorkers);
		await page.evaluate(() => threadPair.instances[1].dispose());
		await waitForWorkers(page, 0);
		const custom = await page.evaluate(customThreadedInputs, threadedInput);
		await waitForWorkers(page, 0);
		return { ...result, ...custom, independentMemories: 2, cleanup: true };
	} finally {
		await page.evaluate(() => {
			for (const instance of threadPair.instances) instance.dispose();
			for (const worker of threadPair.observed.workers) worker.terminate();
			threadPair.observed.restore();
		});
	}
}

export async function threadedFailureDriver({ page, server, input, t, context }) {
	await prepareThreadServer(server, t, context);
	for (const threads of ['preferred', 'required']) {
		await page.goto(server.url);
		await page.evaluate(
			async ({ moduleUrl, threads }) => {
				Object.defineProperty(navigator, 'hardwareConcurrency', { value: 4 });
				const { observeWorkers } = await import('/__tests__/thread-worker-observer.mjs');
				globalThis.partialWorkers = observeWorkers('partial', { failAt: 2 });
				const { createDitherette, DitheretteError } = await import(moduleUrl);
				globalThis.partialOutcome = undefined;
				globalThis.partialCompletion = createDitherette({ threads }).then(
					(instance) => {
						globalThis.fallback = instance;
						globalThis.partialOutcome = { kind: 'scalar' };
					},
					(error) => {
						globalThis.partialOutcome = {
							kind: 'error',
							structured: error instanceof DitheretteError,
							code: error.code,
							path: error.path
						};
					}
				);
			},
			{ ...input, threads }
		);
		try {
			const names = await waitForWorkers(page, 2);
			assert.equal(await page.evaluate(() => globalThis.partialOutcome), undefined);
			await page.evaluate(
				(name) => {
					const gate = new BroadcastChannel(name);
					gate.postMessage('fail-now');
					gate.close();
				},
				names.find((name) => name.includes('-fail-'))
			);
			await page.waitForFunction(() => globalThis.partialOutcome, undefined, { timeout: 15_000 });
			assert.deepEqual(
				await page.evaluate(() => globalThis.partialOutcome),
				threads === 'preferred'
					? { kind: 'scalar' }
					: { kind: 'error', structured: true, code: 'initialization', path: 'threads' }
			);
			await waitForWorkers(page, 0);
			if (threads === 'preferred') {
				assert.deepEqual(
					await page.evaluate(() => [
						...fallback.resize({
							version: 1,
							source: { width: 1, height: 1, data: new Uint8Array([19, 83, 127, 255]) },
							output: { width: 1, height: 1, resize: { algorithm: 'nearest', anchor: 'center' } }
						}).data
					]),
					[19, 83, 127, 255]
				);
			}
		} finally {
			await page.evaluate(() => {
				globalThis.fallback?.dispose();
				for (const worker of partialWorkers.workers) worker.terminate();
				partialWorkers.restore();
			});
		}
	}
	return { partialWorkersObserved: 4, preferredFallback: true, requiredInitializationError: true };
}

export async function processingHostDriver({ page, server, input, t, context }) {
	await prepareThreadServer(server, t, context);
	for (const starting of [false, true]) {
		await page.goto(server.url);
		await page.evaluate(
			({ moduleUrl, starting }) => {
				globalThis.hostEvents = [];
				globalThis.processingHost = new Worker('/__tests__/thread-processing-host.mjs', {
					type: 'module',
					name: 'ditherette-test-host'
				});
				processingHost.onmessage = ({ data }) => hostEvents.push(data);
				processingHost.onerror = (event) =>
					hostEvents.push({ kind: 'error', message: event.message });
				processingHost.postMessage({ kind: 'initialize', moduleUrl, starting });
			},
			{ ...input, starting }
		);
		try {
			if (starting) await waitForWorkers(page, 3);
			else {
				await page.waitForFunction(() => hostEvents.length > 0, undefined, { timeout: 15_000 });
				const ready = await page.evaluate(() => hostEvents[0]);
				assert.equal(ready.kind, 'ready', JSON.stringify(ready));
				assert.ok(ready.workers > 0);
				await waitForWorkers(page, ready.workers + 1);
				await page.evaluate(() =>
					processingHost.postMessage({ kind: 'process', gate: new SharedArrayBuffer(4) })
				);
				await page.waitForFunction(() => hostEvents.length > 1, undefined, { timeout: 10_000 });
				assert.equal(await page.evaluate(() => hostEvents[1].kind), 'processing');
			}
			await page.evaluate(() => processingHost.terminate());
			await waitForWorkers(page, 0);
			assert.equal(
				await page.evaluate(() => hostEvents.some(({ kind }) => kind === 'unexpected-result')),
				false
			);
		} finally {
			await page.evaluate(() => processingHost.terminate());
		}
	}
	return { hostTerminationDuringCall: true, hostTerminationDuringStartup: true };
}
