#!/usr/bin/env node
import { once } from 'node:events';
import { randomUUID } from 'node:crypto';
import { readFile, realpath } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { runBrowserTransport } from './benchmark-transport.mjs';

/** Bind isolated oracle outputs to the emitted transport record without changing measured evidence. */
export function attachOracleReference(result, reference) {
	const { prime_output, ...measuredReference } = reference;
	result.reference = measuredReference;
	if (prime_output) result.prime_reference_output = prime_output;
}

/** Resolve only canonical relative artifact paths. The Rust worker checks full hashes before/after trials. */
export function assetPaths(tree) {
	const paths = new Map();
	for (const file of tree.files) {
		if (
			!file.path ||
			file.path.includes('\\') ||
			path.posix.isAbsolute(file.path) ||
			path.posix.normalize(file.path) !== file.path ||
			file.path.startsWith('../') ||
			file.path === '..'
		)
			throw new Error('Noncanonical asset path.');
		if (paths.has(file.path)) throw new Error('Duplicate asset path.');
		paths.set(file.path, path.resolve(tree.root, file.path));
	}
	return paths;
}

/** The same allowlist covers browser routing and HTTP serving, including lazy module dependencies. */
export function allowedRequest(rawUrl, origin, paths, dataRoutes = new Map(), method = 'GET') {
	const url = new URL(rawUrl);
	if (url.origin !== origin || url.search || url.hash) return false;
	let relative;
	try {
		relative = decodeURIComponent(url.pathname).slice(1);
	} catch {
		return false;
	}
	if (dataRoutes.has(relative)) return dataRoutes.get(relative) === method;
	return method === 'GET' && (relative === '' || paths.has(relative));
}

export async function startAssetServer(assets, isolated, trial) {
	const paths = assetPaths(assets.tree);
	for (const entry of Object.values(assets.entries))
		if (!paths.has(entry)) throw new Error(`Missing asset entry: ${entry}`);
	const failures = [];
	const dataPrefix = '__ditherette_trial__/';
	if ([...paths.keys()].some((name) => name.startsWith(dataPrefix)))
		throw new Error('Asset collides with reserved trial-data routes.');
	const dataRoutes = new Map();
	const token = randomUUID();
	const requestPath = `${dataPrefix}${token}/request`;
	const resultPath = `${dataPrefix}${token}/result`;
	let requestJson = trial === undefined ? undefined : JSON.stringify(trial);
	let result;
	let resultStarted = false;
	// Any trial can discover instability after an exact preflight. Retain both actual outputs.
	const maxResultBytes =
		trial === undefined
			? 0
			: trial.case.identity.output.width * trial.case.identity.output.height * 16 * 2 +
				trial.case.measurement.samples * 32 +
				65_536;
	if (trial !== undefined) {
		if (!Number.isSafeInteger(maxResultBytes) || maxResultBytes < 65_536)
			throw new Error('Invalid result body bound.');
		dataRoutes.set(requestPath, 'GET');
		dataRoutes.set(resultPath, 'POST');
	}
	let origin;
	const instance = createServer(async (request, response) => {
		try {
			const url = new URL(request.url, origin);
			if (
				!allowedRequest(url.href, origin, paths, dataRoutes, request.method) ||
				(request.headers.origin && request.headers.origin !== origin)
			) {
				failures.push(`Rejected asset request ${request.method} ${request.url}`);
				response.writeHead(404).end();
				return;
			}
			if (isolated) {
				response.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
				response.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
			}
			response.setHeader(
				'Content-Security-Policy',
				"default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'; worker-src 'self' blob:"
			);
			response.setHeader('Cache-Control', 'no-store');
			const relative = decodeURIComponent(url.pathname).slice(1);
			if (relative === requestPath && dataRoutes.has(relative)) {
				if (requestJson === undefined) {
					failures.push('Duplicate trial input request.');
					response.writeHead(409).end();
					return;
				}
				response.setHeader('Content-Type', 'application/json');
				response.end(requestJson);
				requestJson = undefined;
				return;
			}
			if (relative === resultPath && dataRoutes.has(relative)) {
				if (resultStarted) {
					failures.push('Duplicate trial result submission.');
					response.writeHead(409).end();
					return;
				}
				resultStarted = true;
				if (request.headers['content-type'] !== 'application/json') {
					failures.push('Trial result requires application/json.');
					response.writeHead(415).end();
					return;
				}
				const length = request.headers['content-length'];
				if (length !== undefined && (!/^\d+$/.test(length) || Number(length) > maxResultBytes)) {
					failures.push('Result body exceeds declared bound.');
					response.writeHead(413).end();
					return;
				}
				const chunks = [];
				let bytes = 0;
				for await (const chunk of request) {
					bytes += chunk.length;
					if (bytes > maxResultBytes) {
						failures.push('Result body exceeds declared bound.');
						response.writeHead(413).end();
						return;
					}
					chunks.push(chunk);
				}
				result = JSON.parse(Buffer.concat(chunks, bytes).toString('utf8'));
				response.writeHead(204).end();
				return;
			}
			const mime = relative.endsWith('.wasm')
				? 'application/wasm'
				: relative.endsWith('.js') || relative.endsWith('.mjs')
					? 'text/javascript'
					: relative.endsWith('.json')
						? 'application/json'
						: 'application/octet-stream';
			response.setHeader('Content-Type', relative ? mime : 'text/html');
			response.end(
				relative
					? await readFile(paths.get(relative))
					: '<!doctype html><meta charset="utf-8"><title>Ditherette benchmark</title>'
			);
		} catch (error) {
			failures.push(String(error));
			response.writeHead(500).end();
		}
	});
	instance.listen(0, '127.0.0.1');
	await once(instance, 'listening');
	origin = `http://127.0.0.1:${instance.address().port}`;
	return {
		instance,
		url: origin,
		paths,
		failures,
		dataRoutes,
		requestUrl: `/${requestPath}`,
		resultUrl: `/${resultPath}`,
		setReference(reference) {
			if (requestJson === undefined || resultStarted)
				throw new Error('Reference arrived after trial execution.');
			requestJson = JSON.stringify({
				...trial,
				reference_output: reference.output,
				prime_reference_output: reference.prime_output
			});
		},
		get result() {
			return result;
		}
	};
}

/** Deny external requests and undeclared dependencies, even when browser caches might otherwise satisfy them. */
export async function restrictContext(context, server) {
	// Server-side exclusion matters: intercepted POST bodies would otherwise enter Playwright RPC again.
	const dataUrls = [...server.dataRoutes.keys()].map((relative) =>
		`${server.url}/${relative}`.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
	);
	const intercepted = dataUrls.length ? new RegExp(`^(?!(?:${dataUrls.join('|')})$)`) : '**/*';
	await context.route(intercepted, async (route) => {
		if (
			allowedRequest(
				route.request().url(),
				server.url,
				server.paths,
				server.dataRoutes,
				route.request().method()
			)
		)
			await route.continue();
		else {
			server.failures.push(`Blocked external or undeclared request: ${route.request().url()}`);
			await route.abort();
		}
	});
}

/** Renderer/process loss rejects the pending control call; it never fabricates timed output. */
export async function rejectOnPageFailure(page, run) {
	let fail;
	const failure = new Promise((_, reject) => {
		fail = reject;
	});
	const crashed = () => fail(new Error('Browser renderer crashed before trial completion.'));
	const closed = () => fail(new Error('Browser page closed before trial completion.'));
	const disconnected = () => fail(new Error('Browser disconnected before trial completion.'));
	const pageError = (error) => fail(error);
	const browser = page.context().browser();
	page.on('crash', crashed);
	page.on('close', closed);
	page.on('pageerror', pageError);
	browser?.on('disconnected', disconnected);
	try {
		return await Promise.race([run(), failure]);
	} finally {
		page.off('crash', crashed);
		page.off('close', closed);
		page.off('pageerror', pageError);
		browser?.off('disconnected', disconnected);
	}
}

/** Bulk JSON travels over bounded loopback HTTP. Playwright receives only URLs and a small acknowledgement. */
export async function exchangeTrial(page, server, pageEntry) {
	const ack = await rejectOnPageFailure(page, () =>
		page.evaluate(
			async ({ requestUrl, resultUrl, pageEntry }) => {
				const response = await fetch(requestUrl);
				if (!response.ok) throw new Error(`Trial input failed: ${response.status}`);
				const request = await response.json();
				const { runTrial } = await import(`/${pageEntry}`);
				const result = await runTrial(request);
				const uploaded = await fetch(resultUrl, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify(result)
				});
				if (!uploaded.ok) throw new Error(`Trial output failed: ${uploaded.status}`);
				return { posted: true };
			},
			{ requestUrl: server.requestUrl, resultUrl: server.resultUrl, pageEntry }
		)
	);
	if (!ack?.posted || server.result === undefined)
		throw new Error('Browser did not publish a complete trial result.');
	return server.result;
}

/** The oracle context and server close before the caller can initialize the actual package. */
export async function frozenBrowserReference(browser, trial) {
	const { assets, runtime } = trial.browser;
	const server = await startAssetServer(assets, runtime.cross_origin_isolated, trial);
	let context;
	try {
		context = await browser.newContext({ serviceWorkers: 'block' });
		await restrictContext(context, server);
		const page = await context.newPage();
		await page.goto(server.url);
		const reference = await exchangeTrial(page, server, 'scripts/benchmark-oracle-page.mjs');
		if (server.failures.length) throw new Error(server.failures.join('\n'));
		return reference;
	} finally {
		try {
			await context?.close();
		} finally {
			server.instance.closeAllConnections();
			await new Promise((resolve) => server.instance.close(resolve));
		}
	}
}

/** Leased CLI transport. Browser/package failures propagate after all owned resources close. */
export async function runPublicBrowser(trial) {
	const { runtime, assets } = trial.browser;
	if (
		(await realpath(process.execPath)) !== (await realpath(runtime.node.path)) ||
		process.version !== runtime.node.version
	)
		throw new Error('Node runtime identity mismatch.');
	const runtimePaths = assetPaths(runtime.playwright.tree);
	const entry = runtimePaths.get(runtime.playwright.entry);
	if (!entry) throw new Error('Playwright entry is absent from its snapshot.');
	const playwright = await import(pathToFileURL(entry));
	const playwrightVersion = JSON.parse(
		await readFile(path.join(path.dirname(entry), 'package.json'), 'utf8')
	).version;
	if (playwrightVersion !== runtime.playwright.version)
		throw new Error('Playwright version mismatch.');
	const engine = playwright[runtime.engine];
	if (!engine) throw new Error('Unknown browser engine.');
	let result;
	await runBrowserTransport({
		startServer: () => startAssetServer(assets, runtime.cross_origin_isolated, trial),
		launchBrowserServer: () =>
			engine.launchServer({
				executablePath: runtime.browser.path,
				args: runtime.launch_args,
				headless: runtime.headless,
				handleSIGINT: false,
				handleSIGTERM: false,
				handleSIGHUP: false
			}),
		run: async ({ server, browserServer }) => {
			const browser = await engine.connect(browserServer.wsEndpoint());
			const version = browser.version();
			if (version !== runtime.browser.version) throw new Error('Browser version mismatch.');
			const reference = await frozenBrowserReference(browser, trial);
			server.setReference(reference);
			const context = await browser.newContext({ serviceWorkers: 'block' });
			try {
				await restrictContext(context, server);
				const page = await context.newPage();
				page.on('console', (message) =>
					process.stderr.write(`[browser ${message.type()}] ${message.text()}\n`)
				);
				page.on('pageerror', (error) => server.failures.push(String(error)));
				await page.goto(server.url);
				result = await exchangeTrial(page, server, assets.entries.page);
				attachOracleReference(result, reference);
				if (server.failures.length) throw new Error(server.failures.join('\n'));
				if (result.observation.cross_origin_isolated !== runtime.cross_origin_isolated)
					throw new Error('Browser isolation mismatch.');
				Object.assign(result.observation, {
					engine: runtime.engine,
					browser_version: version,
					node_version: process.version,
					playwright_version: playwrightVersion
				});
			} finally {
				await context.close();
			}
		}
	});
	return result;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	try {
		if (process.argv.length !== 3)
			throw new Error('Usage: benchmark-public-browser.mjs TRIAL_REQUEST_JSON');
		const trial = JSON.parse(await readFile(process.argv[2], 'utf8'));
		console.log(JSON.stringify(await runPublicBrowser(trial)));
	} catch (error) {
		console.error(error);
		process.exitCode = 1;
	}
}
