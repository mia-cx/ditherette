// Untimed heap diagnostic. RPC/JSON modes use no browser; HTTP echoes data through a parent-owned browser.
// None of these modes invokes image processing or its timing loops.
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const playwrightRequire = createRequire(require.resolve('playwright/package.json'));
const core = path.dirname(playwrightRequire.resolve('playwright-core/package.json'));
const { serializeArgument } = require(path.join(core, 'lib/client/jsHandle.js'));
const request = JSON.parse(await readFile(process.argv[3], 'utf8'));
if (process.argv[2] === 'rpc') {
	const serialized = serializeArgument(request);
	console.log(
		JSON.stringify({
			mode: 'rpc',
			bytes: JSON.stringify(serialized).length,
			rss: process.resourceUsage().maxRSS
		})
	);
} else if (process.argv[2] === 'json') {
	console.log(
		JSON.stringify({
			mode: 'json',
			bytes: JSON.stringify(request).length,
			rss: process.resourceUsage().maxRSS
		})
	);
} else if (process.argv[2] === 'http') {
	const { chromium } = await import('playwright');
	const { exchangeTrial, restrictContext, startAssetServer } =
		await import('./benchmark-public-browser.mjs');
	const temporary = await mkdtemp(path.join(tmpdir(), 'ditherette-ipc-'));
	let server;
	let browser;
	try {
		await writeFile(
			path.join(temporary, 'echo.mjs'),
			'export async function runTrial(request) { return { output: request.reference_output, source: request.case.rgba, marker: "untimed-ipc" }; }'
		);
		// Both directions carry the full source-size payload. This echo never invokes image processing.
		const transferred = {
			...request,
			case: {
				...request.case,
				identity: {
					...request.case.identity,
					output: { width: request.case.source.width, height: request.case.source.height * 2 }
				}
			}
		};
		const assets = {
			tree: { root: temporary, files: [{ path: 'echo.mjs' }] },
			entries: { page: 'echo.mjs' }
		};
		server = await startAssetServer(assets, true, transferred);
		browser = await chromium.connect(process.argv[4]);
		let maxRpcBytes = 0;
		const connection = browser._connection;
		const outgoing = connection.onmessage;
		const incoming = connection.dispatch.bind(connection);
		connection.onmessage = (message) => {
			maxRpcBytes = Math.max(maxRpcBytes, JSON.stringify(message).length);
			return outgoing(message);
		};
		connection.dispatch = (message) => {
			maxRpcBytes = Math.max(maxRpcBytes, JSON.stringify(message).length);
			return incoming(message);
		};
		const context = await browser.newContext({ serviceWorkers: 'block' });
		await restrictContext(context, server);
		const page = await context.newPage();
		await page.goto(server.url);
		const result = await exchangeTrial(page, server, 'echo.mjs');
		if (
			result.marker !== 'untimed-ipc' ||
			result.source.length !== request.case.rgba.length ||
			!result.source.every((byte, index) => byte === request.case.rgba[index])
		)
			throw new Error('IPC source bytes changed.');
		if (
			result.output.pixels.data.length !== request.reference_output.pixels.data.length ||
			!result.output.pixels.data.every(
				(byte, index) => byte === request.reference_output.pixels.data[index]
			)
		)
			throw new Error('IPC output bytes changed.');
		if (maxRpcBytes >= 65_536)
			throw new Error(`Bulk data entered Playwright RPC: ${maxRpcBytes} bytes`);
		if (server.failures.length) throw new Error(server.failures.join('\n'));
		const resultBytes = Buffer.byteLength(JSON.stringify(result));
		console.log(
			JSON.stringify({
				mode: 'http',
				inputBytes: JSON.stringify(transferred).length,
				sourceChannels: result.source.length,
				outputChannels: result.output.pixels.data.length,
				resultBytes,
				maxRpcBytes,
				rss: process.resourceUsage().maxRSS
			})
		);
		await context.close();
	} finally {
		await browser?.close();
		if (server) {
			server.instance.closeAllConnections();
			await new Promise((resolve) => server.instance.close(resolve));
		}
		await rm(temporary, { recursive: true, force: true });
	}
} else throw new Error('Expected rpc, json, or http diagnostic mode.');
