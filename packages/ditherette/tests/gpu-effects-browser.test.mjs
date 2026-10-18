import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtemp, readFile, rm, symlink } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import { homedir, tmpdir } from 'node:os';
import { join, extname } from 'node:path';
import { pathToFileURL } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import test from 'node:test';
import { chromium } from 'playwright';

const root = new URL('../../../', import.meta.url);
const installed = process.env.DITHERETTE_TEST_GPU_PACKAGE
	? pathToFileURL(`${process.env.DITHERETTE_TEST_GPU_PACKAGE}/`)
	: undefined;
const mime = {
	'.js': 'text/javascript',
	'.mjs': 'text/javascript',
	'.wasm': 'application/wasm',
	'.png': 'image/png',
	'.jpg': 'image/jpeg'
};

async function startServer(t, missingGpu = false, isolated = false) {
	const server = createServer(async (request, response) => {
		if (isolated) {
			response.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
			response.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
		}
		try {
			const path = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
			if (path === '/') return response.end('<title>GPU package effects test</title>');
			if (path === '/__private_gpu_fixture.rgba' && process.env.DITHERETTE_GPU_RGBA) {
				response.setHeader('content-type', 'application/octet-stream');
				return response.end(await readFile(process.env.DITHERETTE_GPU_RGBA));
			}
			if (missingGpu && path.startsWith('/packages/ditherette/dist/wasm/gpu/')) {
				response.writeHead(404).end();
				return;
			}
			if (
				!path.startsWith('/packages/ditherette/dist/') &&
				path !== '/packages/ditherette/tests/gpu-dither-worker.mjs' &&
				!path.startsWith('/benchmark-fixtures/')
			) {
				response.writeHead(404).end();
				return;
			}
			const packaged = installed && path.startsWith('/packages/ditherette/dist/');
			const directory = packaged ? installed : root;
			const url = new URL(
				packaged ? path.slice('/packages/ditherette/'.length) : `.${path}`,
				directory
			);
			if (!url.href.startsWith(directory.href)) {
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

for (const mode of ['gpu', 'disabled', 'missing', 'yliluoma', 'diffusion'])
	test(
		`real indexed dither worker through Chromium CDP (${mode})`,
		{ timeout: 180_000 },
		async (t) => {
			const origin = await startServer(t, mode === 'missing');
			const { tab, errors, system } = await browser(t, origin, mode === 'disabled');
			const result = await tab.evaluate(async (mode) => {
				const worker = new Worker('/packages/ditherette/tests/gpu-dither-worker.mjs', {
					type: 'module'
				});
				try {
					return await new Promise((resolve, reject) => {
						worker.onmessage = (event) => {
							if (event.currentTarget === worker) resolve(event.data);
						};
						worker.onerror = (event) => reject(Error(event.message));
						worker.postMessage({
							fallback: !['gpu', 'yliluoma', 'diffusion'].includes(mode),
							yliluoma: mode === 'yliluoma',
							gpuDiffusion: mode === 'diffusion'
						});
					});
				} finally {
					worker.terminate();
				}
			}, mode);
			assert.equal(result.error, undefined, result.error);
			assert.deepEqual(errors, []);
			if (mode === 'gpu') {
				assert.equal(result.cases, 363);
				assert.equal(result.backends.gpu, 315);
				assert.equal(result.backends.hybrid, 48);
				assert.equal(result.webgpu.isFallbackAdapter, false);
				assert.equal(result.cancelled, true);
				assert.equal(result.callbackRecovered, true);
				assert.equal(result.analyses, 1);
				assert.equal(result.reuses, 197);
			} else if (mode === 'yliluoma') {
				assert.equal(result.cases, 121);
				assert.equal(result.backends.gpu, 121);
				assert.equal(result.webgpu.isFallbackAdapter, false);
			} else if (mode === 'diffusion') {
				assert.equal(result.cases, 48);
				assert.equal(result.backends.gpu, 48);
				assert.equal(result.webgpu.isFallbackAdapter, false);
			} else assert.equal(result.backends.cpu, 3);
			t.diagnostic(JSON.stringify({ mode, ...result, system }));
		}
	);

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
				async ({ noGpu, full }) => {
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
					const run = async (source, effects, label, context, tolerance = 0) => {
						const request = { version: 1, source, effects, context };
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
							alphaDifferences,
							tolerance
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
						if (full && !noGpu) {
							const data = new Uint8Array(
								await (await fetch('/__private_gpu_fixture.rgba')).arrayBuffer()
							);
							if (data.length !== 2600 * 4168 * 4)
								throw Error('private fixture must remain full resolution');
							await run(
								{ width: 2600, height: 4168, data },
								[
									...chains[1],
									{
										effect: 'hue-saturation',
										enabled: true,
										hue: 37,
										saturation: 0.17,
										lightness: -0.12
									}
								],
								'full 1x continuous effects',
								undefined,
								1
							);
						}
						await run(
							source,
							[
								...chains[1],
								{ effect: 'hue-saturation', enabled: true, hue: 30, saturation: 0.2, lightness: 0 }
							],
							'mixed continuous GPU effects',
							undefined,
							1
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
							'masked continuous GPU effects',
							undefined,
							1
						);
						for (const [model, channel] of [
							['srgb', 'red'],
							['linear-rgb', 'green'],
							['hsl', 'saturation'],
							['hsv', 'value'],
							['oklab', 'a'],
							['oklch', 'chroma'],
							['cielab', 'lightness'],
							['cielch', 'hue'],
							['ycbcr', 'cb']
						]) {
							await run(
								source,
								[
									{
										effect: 'palette-fit',
										enabled: true,
										look: 'vivid',
										space: 'oklab',
										strength: 1.5,
										curves: [
											{
												kind: 'adjust',
												x: { model: 'hsl', channel: 'hue' },
												x2: { model: 'srgb', channel: 'green' },
												y: { model, channel },
												grid: {
													columns: [0.1, 0.4, 0.8],
													rows: [0, 0.3, 1],
													values: [
														[0.2, 0.4, 0.6],
														[0.7, 0.4, 0.2],
														[0.6, 0.6, 0.8]
													]
												}
											}
										],
										mask: [
											{
												x: { model: 'srgb', channel: 'blue' },
												points: [
													[0, 0.1],
													[1, 1]
												]
											}
										]
									}
								],
								`explicit palette curves ${model}`,
								undefined,
								1
							);
						}
						await run(
							source,
							[
								{
									effect: 'recolour',
									enabled: true,
									strength: 0.7,
									recipe: {
										space: 'oklab',
										tone: [
											[0, 0],
											[0.4, 0.45],
											[1, 0.95]
										],
										chroma: 0.85,
										shift: [0.01, -0.02],
										groups: [{ hue: 90, width: 80, turn: 23, chroma: 1.1 }]
									}
								}
							],
							'explicit recolour recipe',
							{ space: 'oklab' },
							1
						);
						const eligibleReports = reports.length;
						const context = {
							palette: [
								{ kind: 'color', rgb: [0, 0, 0] },
								{ kind: 'color', rgb: [255, 255, 255] }
							]
						};
						await run(
							source,
							[
								{
									effect: 'palette-fit',
									enabled: true,
									space: 'oklab',
									look: 'natural',
									strength: 1,
									curves: null
								}
							],
							'automatic analysis CPU fallback',
							context
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
							eligibleReports,
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
				{ noGpu, full: Boolean(process.env.DITHERETTE_GPU_RGBA) }
			);
			assert.deepEqual(errors, []);
			assert.ok(result.retained);
			assert.ok(result.noOpIdentity);
			assert.ok(result.disabledIdentity);
			assert.ok(result.recovery.exact);
			assert.equal(result.retiredDeviceRequests, result.eligibleDeviceRequests);
			for (const report of result.reports) {
				assert.ok(report.maxByteDifference <= report.tolerance, JSON.stringify(report));
				assert.equal(report.alphaDifferences, 0, JSON.stringify(report));
				if (noGpu) assert.equal(report.backend, 'cpu');
			}
			assert.equal(result.reports.at(-1).backend, 'cpu');
			assert.equal(result.reports.at(-2).backend, 'cpu');
			if (result.reports[0].backend === 'gpu') {
				assert.ok(
					result.reports
						.slice(0, result.eligibleReports)
						.every((report) => report.backend === 'gpu')
				);
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

test(
	'all GPU resize filters preserve tiled output through the public browser worker',
	{ timeout: 60000 },
	async (t) => {
		const origin = await startServer(t);
		const { tab, errors } = await browser(t, origin, false);
		const reports = await tab.evaluate(async () => {
			const reports = [];
			for (const photo of [
				undefined,
				'Celeste_Insta_selfie.png',
				'Picking_at_thread.jpg',
				'The_Quilt_banner.png'
			]) {
				let source;
				if (photo) {
					const bitmap = await createImageBitmap(
						await (await fetch(`/benchmark-fixtures/${photo}`)).blob(),
						{ premultiplyAlpha: 'none', colorSpaceConversion: 'none' }
					);
					const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
					const context = canvas.getContext('2d');
					context.drawImage(bitmap, 0, 0);
					source = {
						width: bitmap.width,
						height: bitmap.height,
						data: new Uint8Array(context.getImageData(0, 0, bitmap.width, bitmap.height).data)
					};
					bitmap.close();
				}
				const worker = new Worker('/packages/ditherette/tests/gpu-dither-worker.mjs', {
					type: 'module'
				});
				try {
					const result = await new Promise((resolve, reject) => {
						worker.onmessage = (event) => {
							if (event.currentTarget === worker) resolve(event.data);
						};
						worker.onerror = (event) => reject(Error(event.message));
						worker.postMessage({ resize: true, source }, source ? [source.data.buffer] : []);
					});
					reports.push({ photo: photo ?? 'RGBA fixture', ...result });
				} finally {
					worker.terminate();
				}
			}
			return reports;
		});
		assert.deepEqual(errors, []);
		for (const result of reports) {
			assert.equal(result.error, undefined, `${result.photo}: ${result.error}`);
			assert.equal(result.calls, 60);
			assert.ok(result.maximumByteDifference <= 2);
			assert.equal(result.callbackRecovered, true);
		}
		t.diagnostic(JSON.stringify(reports));
	}
);

test(
	'explicit CPU tiling works in real scalar and threaded browser workers',
	{ timeout: 60000 },
	async (t) => {
		const origin = await startServer(t, false, true);
		const { tab, errors } = await browser(t, origin, false);
		const result = await tab.evaluate(async () => {
			const worker = new Worker('/packages/ditherette/tests/gpu-dither-worker.mjs', {
				type: 'module'
			});
			try {
				return await new Promise((resolve, reject) => {
					worker.onmessage = (event) => {
						if (event.currentTarget === worker) resolve(event.data);
					};
					worker.onerror = (event) => reject(Error(event.message));
					worker.postMessage({ tiling: true });
				});
			} finally {
				worker.terminate();
			}
		});
		assert.deepEqual(errors, []);
		assert.equal(result.error, undefined, result.error);
		assert.equal(result.calls, 18);
	}
);

test(
	`full 1x indexed Celeste through the real GPU worker${process.env.DITHERETTE_GPU_YLILUOMA ? ' (Yliluoma)' : ''}`,
	{ timeout: 360_000, skip: !process.env.DITHERETTE_GPU_RGBA },
	async (t) => {
		const origin = await startServer(t);
		const { tab, errors, system } = await browser(t, origin, false);
		const text = await readFile(new URL('src/lib/palette/wplace.ts', root), 'utf8');
		const palette = [...text.matchAll(/\['(?!#)[^']+', '(#[0-9A-F]{6})'\]/g)].map(([, hex]) => ({
			kind: 'color',
			rgb: [1, 3, 5].map((offset) => parseInt(hex.slice(offset, offset + 2), 16))
		}));
		palette.push({ kind: 'transparent' });
		assert.equal(palette.length, 64);
		const result = await tab.evaluate(
			async ({ palette, mixtures }) => {
				const data = new Uint8Array(
					await (await fetch('/__private_gpu_fixture.rgba')).arrayBuffer()
				);
				const worker = new Worker('/packages/ditherette/tests/gpu-dither-worker.mjs', {
					type: 'module'
				});
				try {
					return await new Promise((resolve, reject) => {
						worker.onmessage = (event) => {
							if (event.currentTarget === worker) resolve(event.data);
						};
						worker.onerror = (event) => reject(Error(event.message));
						worker.postMessage(
							{
								full: mixtures ? 'yliluoma' : true,
								source: { width: 2600, height: 4168, data },
								palette
							},
							[data.buffer]
						);
					});
				} finally {
					worker.terminate();
				}
			},
			{ palette, mixtures: Boolean(process.env.DITHERETTE_GPU_YLILUOMA) }
		);
		assert.equal(result.error, undefined, result.error);
		assert.deepEqual(errors, []);
		assert.equal(result.cases, process.env.DITHERETTE_GPU_YLILUOMA ? 4 : 32);
		assert.deepEqual(
			result.backends,
			process.env.DITHERETTE_GPU_YLILUOMA ? { gpu: 4 } : { gpu: 16, hybrid: 16 }
		);
		assert.equal(result.analyses, process.env.DITHERETTE_GPU_YLILUOMA ? 1 : 8);
		assert.equal(result.reuses, process.env.DITHERETTE_GPU_YLILUOMA ? 1 : 24);
		t.diagnostic(JSON.stringify({ ...result, system }));
	}
);

test(
	'indexed benchmark photos use GPU matching, fields and Adaptive analysis',
	{ timeout: 180_000 },
	async (t) => {
		const origin = await startServer(t);
		const { tab, errors, system } = await browser(t, origin, false);
		const reports = await tab.evaluate(async () => {
			const reports = [];
			for (const photo of [
				'Celeste_Insta_selfie.png',
				'Picking_at_thread.jpg',
				'The_Quilt_banner.png'
			]) {
				const bitmap = await createImageBitmap(
					await (await fetch(`/benchmark-fixtures/${photo}`)).blob(),
					{
						premultiplyAlpha: 'none',
						colorSpaceConversion: 'none'
					}
				);
				const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
				const context = canvas.getContext('2d');
				context.drawImage(bitmap, 0, 0);
				const source = {
					width: bitmap.width,
					height: bitmap.height,
					data: new Uint8Array(context.getImageData(0, 0, bitmap.width, bitmap.height).data)
				};
				bitmap.close();
				const worker = new Worker('/packages/ditherette/tests/gpu-dither-worker.mjs', {
					type: 'module'
				});
				try {
					const result = await new Promise((resolve, reject) => {
						worker.onmessage = (event) => {
							if (event.currentTarget === worker) resolve(event.data);
						};
						worker.onerror = (event) => reject(Error(event.message));
						worker.postMessage({ photo: true, source }, [source.data.buffer]);
					});
					reports.push({ photo, ...result });
				} finally {
					worker.terminate();
				}
			}
			return reports;
		});
		assert.deepEqual(errors, []);
		for (const report of reports) {
			assert.equal(report.error, undefined, report.error);
			assert.equal(report.cases, 3);
			assert.deepEqual(report.backends, { gpu: 2, hybrid: 1 });
			assert.equal(report.analyses, 1);
			assert.equal(report.reuses, 1);
		}
		t.diagnostic(JSON.stringify({ reports, system }));
	}
);
