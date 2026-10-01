// Browser half of the GPU parity harness: builds the `gpu` feature, opens Chromium
// with WebGPU, uploads a known image through `copyExternalImageToTexture`, and prints
// the parity JSON from `gpuParity`. `--no-webgpu` launches without WebGPU to show the fallback.
// Usage: node scripts/gpu-parity-browser.mjs [--skip-build] [--no-webgpu]
import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const crate = new URL('../', import.meta.url);
const out = new URL('dist/gpu/', crate);

if (!process.argv.includes('--skip-build')) {
	const args = ['build', '.', '--target', 'web', '--out-dir', 'dist/gpu', '--'];
	const result = spawnSync('wasm-pack', [...args, '--locked', '--features', 'gpu'], {
		cwd: crate,
		stdio: 'inherit'
	});
	if (result.status !== 0) process.exit(result.status ?? 1);
}

// Odd width, every byte in every channel, transparent pixels included.
const page = `<script type="module">
	import init, { gpuParity } from './ditherette_wasm.js';
	await init();
	const [width, height] = [37, 19];
	const rgba = new Uint8ClampedArray(width * height * 4).map((_, i) => (i * 61 + (i >> 2)) & 255);
	const bitmap = await createImageBitmap(new ImageData(rgba, width, height), {
		premultiplyAlpha: 'none',
		colorSpaceConversion: 'none'
	});
	window.report = await gpuParity(bitmap, new Uint8Array(rgba.buffer));
</script>`;

const types = { '.js': 'text/javascript', '.wasm': 'application/wasm' };
const server = createServer(async (request, response) => {
	if (request.url === '/') return response.end(page);
	try {
		const body = await readFile(fileURLToPath(new URL(`.${request.url}`, out)));
		response.writeHead(200, { 'content-type': types[extname(request.url)] ?? 'text/html' });
		response.end(body);
	} catch {
		response.writeHead(404).end();
	}
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));

const webgpu = ['--enable-unsafe-webgpu', '--use-angle=swiftshader', '--enable-features=Vulkan'];
const browser = await chromium.launch({
	args: process.argv.includes('--no-webgpu') ? ['--disable-features=WebGPU'] : webgpu
});
try {
	const tab = await browser.newPage();
	tab.on('console', (message) => console.error(`[page] ${message.text()}`));
	tab.on('pageerror', (error) => console.error(`[page] ${error.message}`));
	await tab.goto(`http://127.0.0.1:${server.address().port}/`);
	await tab.waitForFunction(() => window.report !== undefined, null, { timeout: 60_000 });
	console.log(await tab.evaluate(() => window.report));
} finally {
	await browser.close();
	server.close();
}
