import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { workerLifetimePrelude, workerLifetimeEpilogue } from './thread-worker-observer.mjs';
import { scalarSelectionChecks } from './threads-browser-fixture.mjs';

export const threadTestAssets = Object.fromEntries(
	[
		'thread-worker-observer.mjs',
		'thread-lifetime-worker.mjs',
		'thread-processing-host.mjs',
		'threads-browser-fixture.mjs',
		'thread-atomic-wait-worker.mjs'
	].map((name) => [`__tests__/${name}`, fileURLToPath(new URL(name, import.meta.url))])
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
		'__tests__/thread-processing-host.mjs',
		'__tests__/thread-atomic-wait-worker.mjs'
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
	await startCheckHost(page);
	const threadedInput = { ...input, wasmUrl: input.wasmUrl.replace('/scalar/', '/threads/') };
	const { firstWorkers, totalWorkers } = await hostCheck(page, 'initializePair', threadedInput);
	try {
		await waitForWorkers(page, totalWorkers);
		const result = await hostCheck(page, 'exercisePair');
		await waitForWorkers(page, totalWorkers - firstWorkers);
		await hostCheck(page, 'disposeSecond');
		await waitForWorkers(page, 0);
		const custom = await hostCheck(page, 'customInputs', threadedInput);
		await waitForWorkers(page, 0);
		return { ...result, ...custom, independentMemories: 2, cleanup: true };
	} finally {
		await hostCheck(page, 'cleanupPair');
		await page.evaluate(() => checkHost.terminate());
	}
}

async function startCheckHost(page) {
	await page.evaluate(() => {
		globalThis.checkHost = new Worker('/__tests__/thread-processing-host.mjs', { type: 'module', name: 'fixture-host' });
		let id = 0;
		globalThis.hostCheck = (command) => new Promise((resolve, reject) => {
			const request = ++id;
			const onMessage = ({ data }) => {
				if (data.id !== request) return;
				cleanup();
				if (data.kind === 'error') reject(new Error(`${data.message} (code=${data.code}, path=${data.path})`));
				else resolve(data.result);
			};
			const onError = (event) => { cleanup(); reject(new Error(event.message)); };
			const timeout = setTimeout(() => { cleanup(); reject(new Error('Thread fixture command timed out.')); }, 20_000);
			const cleanup = () => {
				clearTimeout(timeout);
				checkHost.removeEventListener('message', onMessage);
				checkHost.removeEventListener('error', onError);
			};
			checkHost.addEventListener('message', onMessage);
			checkHost.addEventListener('error', onError);
			checkHost.postMessage({ ...command, kind: 'check', id: request });
		});
	});
}

function hostCheck(page, operation, input) {
	return page.evaluate((command) => globalThis.hostCheck(command), { operation, input });
}

export async function threadedFailureDriver({ page, server, input, t, context }) {
	await prepareThreadServer(server, t, context);
	for (const threads of ['preferred', 'required']) {
		await page.goto(server.url);
		await startCheckHost(page);
		await hostCheck(page, 'initializePartial', { ...input, threads });
		try {
			const names = await waitForWorkers(page, 2);
			assert.equal(await hostCheck(page, 'partialOutcome'), undefined);
			await page.evaluate(
				(name) => {
					const gate = new BroadcastChannel(name);
					gate.postMessage('fail-now');
					gate.close();
				},
				names.find((name) => name.includes('-fail-'))
			);
			const result = await hostCheck(page, 'completePartial');
			assert.deepEqual(
				result.outcome,
				threads === 'preferred'
					? { kind: 'scalar' }
					: { kind: 'error', structured: true, code: 'initialization', path: 'threads' }
			);
			await waitForWorkers(page, 0);
			assert.deepEqual(
				result.scalarFetches,
				threads === 'preferred' ? [[]] : [],
				'Partial workers are gone before scalar Wasm fetching, and required never falls back.'
			);
			if (threads === 'preferred') {
				assert.deepEqual(
					result.output,
					[19, 83, 127, 255]
				);
			}
		} finally {
			await hostCheck(page, 'cleanupPartial');
			await page.evaluate(() => checkHost.terminate());
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

/** Opt-in engine diagnostic, independent of the package and Rayon. */
export async function atomicWaitDriver({ page, server, t, context }) {
	await prepareThreadServer(server, t, context);
	await page.goto(server.url);
	await page.evaluate(() => {
		globalThis.atomicWorker = new Worker('/__tests__/thread-atomic-wait-worker.mjs', {
			type: 'module',
			name: 'ditherette-test-atomic-wait'
		});
		atomicWorker.onmessage = () => {
			globalThis.atomicReady = true;
		};
		atomicWorker.onerror = (event) => {
			globalThis.atomicError = event.message;
		};
		atomicWorker.postMessage('start');
	});
	try {
		await page.waitForFunction(() => globalThis.atomicReady || globalThis.atomicError, undefined, {
			timeout: 10_000
		});
		assert.equal(await page.evaluate(() => globalThis.atomicError), undefined);
		await waitForWorkers(page, 1);
		await page.evaluate(() => atomicWorker.terminate());
		await waitForWorkers(page, 0);
		return { wasmWaitWorkerTerminated: true };
	} finally {
		await page.evaluate(() => atomicWorker.terminate());
	}
}
