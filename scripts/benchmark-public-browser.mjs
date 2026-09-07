#!/usr/bin/env node
import { once } from 'node:events';
import { readFile, realpath } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { runBrowserTransport } from './benchmark-transport.mjs';

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
export function allowedRequest(rawUrl, origin, paths) {
	const url = new URL(rawUrl);
	if (url.origin !== origin || url.search || url.hash) return false;
	let relative;
	try {
		relative = decodeURIComponent(url.pathname).slice(1);
	} catch {
		return false;
	}
	return relative === '' || paths.has(relative);
}

export async function startAssetServer(assets, isolated) {
	const paths = assetPaths(assets.tree);
	for (const entry of Object.values(assets.entries))
		if (!paths.has(entry)) throw new Error(`Missing asset entry: ${entry}`);
	const failures = [];
	let origin;
	const instance = createServer(async (request, response) => {
		try {
			const url = new URL(request.url, origin);
			if (request.method !== 'GET' || !allowedRequest(url.href, origin, paths)) {
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
				"default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'"
			);
			response.setHeader('Cache-Control', 'no-store');
			const relative = decodeURIComponent(url.pathname).slice(1);
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
	return { instance, url: origin, paths, failures };
}

/** Deny external requests and undeclared dependencies, even when browser caches might otherwise satisfy them. */
export async function restrictContext(context, server) {
	await context.route('**/*', async (route) => {
		if (allowedRequest(route.request().url(), server.url, server.paths)) await route.continue();
		else {
			server.failures.push(`Blocked external or undeclared request: ${route.request().url()}`);
			await route.abort();
		}
	});
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
		startServer: () => startAssetServer(assets, runtime.cross_origin_isolated),
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
			const context = await browser.newContext({ serviceWorkers: 'block' });
			try {
				await restrictContext(context, server);
				const page = await context.newPage();
				page.on('console', (message) =>
					process.stderr.write(`[browser ${message.type()}] ${message.text()}\n`)
				);
				page.on('pageerror', (error) => server.failures.push(String(error)));
				await page.goto(server.url);
				result = await page.evaluate(async (request) => {
					const { runTrial } = await import(`/${request.browser.assets.entries.page}`);
					return runTrial(request);
				}, trial);
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
