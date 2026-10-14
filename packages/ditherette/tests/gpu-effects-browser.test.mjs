import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtemp, readFile, rm, symlink } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import { homedir, tmpdir } from 'node:os';
import { join, extname } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import test from 'node:test';
import { chromium } from 'playwright';

const root = new URL('../../../', import.meta.url);
const mime = {
	'.js': 'text/javascript',
	'.wasm': 'application/wasm',
	'.png': 'image/png',
	'.jpg': 'image/jpeg'
};

async function startServer(t, missingGpu = false) {
	const server = createServer(async (request, response) => {
		try {
			const path = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
			if (path === '/') return response.end('<title>GPU package effects test</title>');
			if (missingGpu && path.startsWith('/packages/ditherette/dist/wasm/gpu/')) {
				response.writeHead(404).end();
				return;
			}
			if (
				!path.startsWith('/packages/ditherette/dist/') &&
				!path.startsWith('/benchmark-fixtures/')
			) {
				response.writeHead(404).end();
				return;
			}
			const url = new URL(`.${path}`, root);
			if (!url.href.startsWith(root.href)) {
				response.writeHead(404).end();
				return;
			}
			response.setHeader('content-type', mime[extname(path)] ?? 'application/octet-stream');
			response.end(await readFile(url));
		} catch {
			response.writeHead(404).end();
		}
	});
	await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
	t.after(
		() =>
			new Promise((resolve) => {
				server.closeAllConnections();
				server.close(resolve);
			})
	);

	return `http://127.0.0.1:${server.address().port}`;
}

async function browser(t, origin, noGpu) {
	const directory = await mkdtemp(join(tmpdir(), 'ditherette-gpu-cdp-'));
	// Chromium's singleton socket pathname has a 108-byte Unix limit. A short
	// alias keeps the actual socket/profile artifacts under the caller's TMPDIR.
	const sockets = join(homedir(), '.cache', `dg-${randomUUID().slice(0, 8)}`);
	await symlink(directory, sockets);
	const process = spawn(
		globalThis.process.env.DITHERETTE_TEST_CHROMIUM_EXECUTABLE ?? '/usr/bin/chromium',
		[
			'--headless=new',
			'--no-sandbox',
			'--disable-dev-shm-usage',
			'--remote-debugging-port=0',
			`--user-data-dir=${directory}`,
			'--no-first-run',
			'--no-default-browser-check',
			...(noGpu
				? ['--disable-features=WebGPU']
				: [
						'--enable-unsafe-webgpu',
						'--ignore-gpu-blocklist',
						'--enable-features=Vulkan',
						'--use-angle=vulkan'
					]),
			'data:text/html,<title>Ditherette GPU issue 381</title><p>GPU package validation</p>'
		],
		{ stdio: ['ignore', 'ignore', 'pipe'], env: { ...globalThis.process.env, TMPDIR: sockets } }
	);
	let stderr = '';
	process.stderr.on('data', (chunk) => {
		stderr = (stderr + chunk).slice(-8000);
	});
	let connection;
	t.after(async () => {
		await connection?.close();
		if (process.exitCode === null && process.signalCode === null) {
			const closed = new Promise((resolve) => process.once('exit', resolve));
			process.kill('SIGTERM');
			await closed;
		}
		await rm(sockets, { force: true });
		await rm(directory, { recursive: true, force: true, maxRetries: 3, retryDelay: 100 });
	});
	let port;
	for (
		let attempt = 0;
		attempt < 120 && process.exitCode === null && process.signalCode === null;
		attempt++
	) {
		const endpoint = stderr.match(/DevTools listening on ws:\/\/127\.0\.0\.1:(\d+)\//);
		if (endpoint) {
			port = endpoint[1];
			break;
		}
		try {
			port = (await readFile(join(directory, 'DevToolsActivePort'), 'utf8')).split('\n')[0];
			break;
		} catch {
			await delay(250);
		}
	}
	assert.ok(
		port,
		`Chromium must expose CDP (exit ${process.exitCode}, signal ${process.signalCode}): ${stderr}`
	);
	connection = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
	const control = await connection.newBrowserCDPSession();
	const system = await control.send('SystemInfo.getInfo');
	const { targetId } = await control.send('Target.createTarget', { url: origin, background: true });
	let tab;
	for (let attempt = 0; attempt < 120; attempt++) {
		tab = connection
			.contexts()[0]
			.pages()
			.find((page) => page.url().startsWith(origin));
		if (tab) break;
		await delay(100);
	}
	assert.ok(tab, `CDP target ${targetId} must attach`);
	const cdp = await tab.context().newCDPSession(tab);
	await cdp.send('Emulation.setFocusEmulationEnabled', { enabled: true });
	const errors = [];
	tab.on('pageerror', (error) => errors.push(error.message));
	return { tab, errors, system: system.gpu.devices };
}

for (const noGpu of [false, true])
	test(
		`real package effects through Chromium CDP (${noGpu ? 'WebGPU disabled' : 'GPU requested'})`,
		{ timeout: 180_000 },
		async (t) => {
			const origin = await startServer(t);
			const { tab, errors, system } = await browser(t, origin, noGpu);
			const result = await tab.evaluate(
				async ({ noGpu }) => {
					const { createDitherette, createGpuEffects } =
						await import('/packages/ditherette/dist/index.js');
					let deviceRequests = 0;
					if (navigator.gpu) {
						const requestAdapter = navigator.gpu.requestAdapter.bind(navigator.gpu);
						navigator.gpu.requestAdapter = async (...args) => {
							const adapter = await requestAdapter(...args);
							if (adapter) {
								const requestDevice = adapter.requestDevice.bind(adapter);
								adapter.requestDevice = (...args) => {
									deviceRequests++;
									return requestDevice(...args);
								};
							}
							return adapter;
						};
					}
					const selected = await navigator.gpu?.requestAdapter({
						powerPreference: 'high-performance'
					});
					const webgpuInfo = selected
						? {
								vendor: selected.info.vendor,
								architecture: selected.info.architecture,
								device: selected.info.device,
								description: selected.info.description,
								isFallbackAdapter: selected.info.isFallbackAdapter ?? selected.isFallbackAdapter
							}
						: null;
					const chains = [
						[
							{
								effect: 'levels',
								enabled: true,
								channel: 'red',
								input: { black: 0.05, white: 0.9 },
								gamma: 1.3,
								output: { black: 0.02, white: 1 }
							}
						],
						[
							{ effect: 'brightness-contrast', enabled: true, brightness: -0.11, contrast: 0.32 },
							{ effect: 'exposure', enabled: true, stops: 0.6 },
							{ effect: 'white-balance', enabled: true, temperature: -0.2, tint: 0.4 }
						],
						[
							{
								effect: 'curves',
								enabled: true,
								curves: [
									{
										kind: 'remap',
										x: { model: 'srgb', channel: 'green' },
										y: { model: 'srgb', channel: 'green' },
										points: [
											[0, 0.1],
											[0.4, 0.8],
											[1, 0.9]
										]
									}
								]
							}
						]
					];
					const cpu = await createDitherette();
					const gpu = await createGpuEffects();
					const reports = [];
					let retained;
					const run = async (source, effects, label) => {
						const request = { version: 1, source, effects };
						const wanted = cpu.applyEffects(request);
						const actual = await gpu.applyEffects(request);
						let maxByteDifference = 0;
						let alphaDifferences = 0;
						for (let i = 0; i < wanted.data.length; i++) {
							maxByteDifference = Math.max(
								maxByteDifference,
								Math.abs(actual.data[i] - wanted.data[i])
							);
							if (i % 4 === 3 && actual.data[i] !== source.data[i]) alphaDifferences++;
						}
						reports.push({
							label,
							pixels: source.width * source.height,
							...gpu.execution,
							maxByteDifference,
							alphaDifferences
						});
						retained ??= { output: actual.data, copy: actual.data.slice() };
					};
					try {
						const width = 37,
							height = 19;
						const data = Uint8Array.from(
							{ length: width * height * 4 },
							(_, i) => (i * 61 + (i >> 2)) & 255
						);
						const source = { width, height, data };
						await run(source, chains[1], 'RGBA ramp packed upload');
						const noOpIdentity =
							(await gpu.applyEffects({ version: 1, source, effects: [] })) === source;
						const disabledIdentity =
							(await gpu.applyEffects({
								version: 1,
								source,
								effects: chains[1].map((step) => ({ ...step, enabled: false }))
							})) === source;
						for (const photo of [
							'Celeste_Insta_selfie.png',
							'Picking_at_thread.jpg',
							'The_Quilt_banner.png'
						]) {
							const decoded = await createImageBitmap(
								await (await fetch(`/benchmark-fixtures/${photo}`)).blob(),
								{ premultiplyAlpha: 'none', colorSpaceConversion: 'none' }
							);
							try {
								// Decode is host-owned. Both backends receive these exact packed
								// reference bytes; GPU output comes only from mapped readback.
								const canvas = new OffscreenCanvas(decoded.width, decoded.height);
								const context = canvas.getContext('2d');
								context.drawImage(decoded, 0, 0);
								const data = new Uint8Array(
									context.getImageData(0, 0, decoded.width, decoded.height).data
								);
								const source = { width: decoded.width, height: decoded.height, data };
								for (const [index, chain] of chains.entries())
									await run(source, chain, `${photo}, chain ${index}`);
							} finally {
								decoded.close();
							}
						}
						const eligibleDeviceRequests = deviceRequests;
						await run(
							source,
							[
								...chains[1],
								{ effect: 'hue-saturation', enabled: true, hue: 30, saturation: 0.2, lightness: 0 }
							],
							'mixed CPU fallback'
						);
						await run(
							source,
							[
								{
									...chains[1][0],
									mask: [
										{
											x: { model: 'srgb', channel: 'red' },
											points: [
												[0, 0],
												[1, 1]
											]
										}
									]
								}
							],
							'masked CPU fallback'
						);
						for (let i = 0; i < 16; i++)
							await run(
								source,
								[
									...chains[1],
									...(i % 2
										? [
												{
													effect: 'hue-saturation',
													enabled: true,
													hue: 30,
													saturation: 0.2,
													lightness: 0
												}
											]
										: [])
								],
								`retired facade ${i}`
							);
						const retiredDeviceRequests = deviceRequests;
						const fresh = await createGpuEffects();
						let recovery;
						try {
							const input = { version: 1, source, effects: chains[1] };
							const wanted = cpu.applyEffects(input);
							const actual = await fresh.applyEffects(input);
							recovery = {
								...fresh.execution,
								exact: actual.data.every((byte, index) => byte === wanted.data[index])
							};
						} finally {
							fresh.dispose();
						}
						return {
							reports,
							noOpIdentity,
							disabledIdentity,
							eligibleDeviceRequests,
							retiredDeviceRequests,
							deviceRequests,
							recovery,
							retained: Array.from(retained.output).every(
								(value, index) => value === retained.copy[index]
							),
							hasGpu: 'gpu' in navigator,
							noGpu,
							webgpuInfo
						};
					} finally {
						gpu.dispose();
						cpu.dispose();
					}
				},
				{ noGpu }
			);
			assert.deepEqual(errors, []);
			assert.ok(result.retained);
			assert.ok(result.noOpIdentity);
			assert.ok(result.disabledIdentity);
			assert.ok(result.recovery.exact);
			assert.equal(result.retiredDeviceRequests, result.eligibleDeviceRequests);
			for (const report of result.reports) {
				assert.equal(report.maxByteDifference, 0, JSON.stringify(report));
				assert.equal(report.alphaDifferences, 0, JSON.stringify(report));
				if (noGpu) assert.equal(report.backend, 'cpu');
			}
			assert.equal(result.reports.at(-1).backend, 'cpu');
			assert.equal(result.reports.at(-2).backend, 'cpu');
			if (result.reports[0].backend === 'gpu') {
				assert.ok(result.reports.slice(0, 10).every((report) => report.backend === 'gpu'));
				assert.equal(result.eligibleDeviceRequests, 1);
				assert.equal(result.deviceRequests, 2);
				assert.equal(result.recovery.backend, 'gpu');
			}
			if (!noGpu && !result.reports.some((report) => report.backend === 'gpu'))
				t.diagnostic(`No browser hardware GPU execution: ${JSON.stringify(result.reports[0])}`);
			t.diagnostic(JSON.stringify({ system, ...result }));
		}
	);

test(
	'optional GPU assets can be absent while the public package operation still works',
	{ timeout: 60_000 },
	async (t) => {
		const origin = await startServer(t, true);
		const { tab, errors } = await browser(t, origin, false);
		const result = await tab.evaluate(async () => {
			const { createDitherette, createGpuEffects } =
				await import('/packages/ditherette/dist/index.js');
			const source = { width: 1, height: 1, data: new Uint8Array([71, 93, 117, 29]) };
			const request = {
				version: 1,
				source,
				effects: [{ effect: 'exposure', enabled: true, stops: 0.5 }]
			};
			const cpu = await createDitherette();
			const gpu = await createGpuEffects();
			try {
				const wanted = cpu.applyEffects(request);
				const actual = await gpu.applyEffects(request);
				return {
					wanted: [...wanted.data],
					actual: [...actual.data],
					execution: gpu.execution,
					capable: 'gpu' in navigator
				};
			} finally {
				gpu.dispose();
				cpu.dispose();
			}
		});
		assert.deepEqual(errors, []);
		assert.deepEqual(result.actual, result.wanted);
		assert.equal(result.execution.backend, 'cpu');
		assert.equal(result.execution.fallback, result.capable ? 'Initialization' : 'NoAdapter');
	}
);
