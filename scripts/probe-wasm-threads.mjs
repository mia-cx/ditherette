#!/usr/bin/env node
import { createReadStream, existsSync } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, join, normalize, resolve } from 'node:path';
import { chromium } from 'playwright';

const root = resolve(process.cwd(), 'static');
const threadedModule = join(root, 'wasm/ditherette-wasm-threads/ditherette_wasm.js');

if (!existsSync(threadedModule)) {
	throw new Error('Threaded Wasm package is missing. Run `pnpm wasm:build:threads` first.');
}

const server = createServer(async (request, response) => {
	const pathname = new URL(request.url ?? '/', 'http://localhost').pathname;
	const filePath = safeStaticPath(pathname === '/' ? '/thread-probe.html' : pathname);
	setIsolationHeaders(response);

	if (pathname === '/') {
		response.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
		response.end('<!doctype html><title>ditherette wasm thread probe</title>');
		return;
	}

	if (!filePath) {
		response.writeHead(404).end('not found');
		return;
	}

	try {
		const fileStat = await stat(filePath);
		if (!fileStat.isFile()) throw new Error('not a file');
		response.writeHead(200, { 'Content-Type': contentType(filePath) });
		createReadStream(filePath).pipe(response);
	} catch {
		response.writeHead(404).end('not found');
	}
});

const { port } = await listen(server);
const browser = await chromium.launch();
try {
	const page = await browser.newPage();
	await page.goto(`http://127.0.0.1:${port}/`);
	const result = await page.evaluate(async () => {
		const wasm = await import('/wasm/ditherette-wasm-threads/ditherette_wasm.js');
		await wasm.default();
		await wasm.initThreadPool(2);
		const output = wasm.convertColorSpace(
			new Uint8Array([255, 128, 0, 255, 0, 128, 255, 128]),
			2,
			1,
			'rgba8',
			'srgb-f32',
			true
		);
		return {
			crossOriginIsolated,
			length: output.length,
			values: Array.from(output.slice(0, 8))
		};
	});
	console.log(JSON.stringify(result, null, 2));
	if (!result.crossOriginIsolated) throw new Error('browser page is not cross-origin isolated');
	if (result.length !== 8) throw new Error(`unexpected output length ${result.length}`);
} finally {
	await browser.close();
	server.close();
}

function listen(server) {
	return new Promise((resolve, reject) => {
		server.once('error', reject);
		server.listen(0, '127.0.0.1', () => {
			server.off('error', reject);
			resolve(server.address());
		});
	});
}

function safeStaticPath(pathname) {
	const decoded = decodeURIComponent(pathname);
	const candidate = normalize(join(root, decoded));
	return candidate.startsWith(root) ? candidate : undefined;
}

function setIsolationHeaders(response) {
	response.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
	response.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
	response.setHeader('Cross-Origin-Resource-Policy', 'same-origin');
}

function contentType(filePath) {
	switch (extname(filePath)) {
		case '.js':
			return 'text/javascript; charset=utf-8';
		case '.wasm':
			return 'application/wasm';
		case '.json':
			return 'application/json; charset=utf-8';
		default:
			return 'application/octet-stream';
	}
}
