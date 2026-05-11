#!/usr/bin/env node
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { Worker } from 'node:worker_threads';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const WORKER_SOURCE = `
const { parentPort, workerData } = require('node:worker_threads');
const source = new Uint8ClampedArray(workerData.sourceBuffer);
const { width, paletteData, colorSpace } = workerData;
parentPort.on('message', ({ startY, endY, outputBuffer, thresholds, size }) => {
  const output = new Uint8Array(outputBuffer);
  const mask = size - 1;
  const shift = size === 2 ? 1 : size === 4 ? 2 : size === 8 ? 3 : 4;
  for (let y = startY; y < endY; y++) {
    const row = y * width;
    const thresholdRow = (y & mask) << shift;
    for (let x = 0; x < width; x++) {
      const index = row + x;
      const offset = index * 4;
      const t = thresholds[thresholdRow + (x & mask)] * 100 * 0.25;
      const r = source[offset], g = source[offset + 1], b = source[offset + 2];
      let v0, v1, v2;
      if (colorSpace === 'weighted-rgb-601') {
        v0 = r * Math.sqrt(0.299); v1 = g * Math.sqrt(0.587); v2 = b * Math.sqrt(0.114);
      } else {
        const rr = srgbByteToLinear(r), gg = srgbByteToLinear(g), bb = srgbByteToLinear(b);
        const l = 0.4122214708 * rr + 0.5363325363 * gg + 0.0514459929 * bb;
        const m = 0.2119034982 * rr + 0.6806995451 * gg + 0.1073969566 * bb;
        const s = 0.0883024619 * rr + 0.2817188376 * gg + 0.6299787005 * bb;
        const lr = Math.cbrt(l), mr = Math.cbrt(m), sr = Math.cbrt(s);
        v0 = 0.2104542553 * lr + 0.793617785 * mr - 0.0040720468 * sr;
        v1 = 1.9779984951 * lr - 2.428592205 * mr + 0.4505937099 * sr;
        v2 = 0.0259040371 * lr + 0.7827717662 * mr - 0.808675766 * sr;
      }
      v0 += t * paletteData.ranges[0]; v1 += t * paletteData.ranges[1]; v2 += t * paletteData.ranges[2];
      let winner = 0, best = Infinity;
      for (let i = 0; i < paletteData.count; i++) {
        const d0 = v0 - paletteData.v0[i], d1 = v1 - paletteData.v1[i], d2 = v2 - paletteData.v2[i];
        const dist = d0*d0 + d1*d1 + d2*d2;
        if (dist < best) { best = dist; winner = i; }
      }
      output[index] = paletteData.indices[winner];
    }
  }
  parentPort.postMessage(0);
});
function srgbByteToLinear(v) { const x = v / 255; return x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4; }
`;

installImageDataPolyfill();
const args = parseArgs(process.argv.slice(2));
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const outDir = path.resolve(
	root,
	args.out ?? `benchmark-results/row-workers-${new Date().toISOString().replace(/[:.]/g, '-')}`
);
const server = await createServer({
	root,
	configFile: path.join(root, 'vite.config.ts'),
	appType: 'custom',
	logLevel: 'silent',
	server: { middlewareMode: true, hmr: false }
});
let browser;
try {
	const [{ WPLACE_PALETTE }, bayerModule] = await Promise.all([
		server.ssrLoadModule('/src/lib/palette/wplace.ts'),
		server.ssrLoadModule('/src/lib/processing/bayer.ts')
	]);
	const imagePath = path.resolve(root, args.image ?? 'benchmark-fixtures/Celeste_box_art_full.png');
	browser = await chromium.launch({ headless: true });
	console.log(`Decoding ${path.relative(root, imagePath)}...`);
	const source = await decodeBenchmarkImage(browser, imagePath);
	const palette = WPLACE_PALETTE.map((color, index) => ({
		index,
		rgb: color.rgb,
		kind: color.kind
	})).filter((color) => color.rgb && color.kind !== 'transparent');
	const sourceBuffer = new SharedArrayBuffer(source.data.byteLength);
	new Uint8ClampedArray(sourceBuffer).set(source.data);
	const cases = [];
	for (const dither of args.dithers ?? ['bayer-8', 'bayer-16']) {
		const size = bayerModule.bayerSizeForAlgorithm(dither);
		if (!size) continue;
		cases.push({ dither, size, thresholds: bayerModule.normalizedBayerMatrix(size) });
	}
	const workers = args.workers ?? [1, 2, 4, Math.min(8, os.cpus().length)];
	const rows = [];
	for (const testCase of cases) {
		for (const colorSpace of args.colorSpaces ?? ['weighted-rgb-601', 'oklab']) {
			for (const workerCount of workers) {
				rows.push(
					await runWorkerCase({
						source,
						sourceBuffer,
						palette,
						testCase,
						colorSpace,
						workerCount,
						iterations: args.iterations ?? 3,
						warmups: args.warmups ?? 1
					})
				);
			}
		}
	}
	await mkdir(outDir, { recursive: true });
	await writeFile(
		path.join(outDir, 'row-workers.json'),
		`${JSON.stringify({ version: 1, runAt: new Date().toISOString(), rows }, null, 2)}\n`
	);
	await writeFile(path.join(outDir, 'row-workers.csv'), `${rowsToCsv(rows)}\n`);
	console.log(formatTable(rows));
	console.log(`\nWrote ${path.relative(root, path.join(outDir, 'row-workers.json'))}`);
} finally {
	await browser?.close();
	await server.close();
}

async function runWorkerCase({
	source,
	sourceBuffer,
	palette,
	testCase,
	colorSpace,
	workerCount,
	iterations,
	warmups
}) {
	const paletteData = buildPaletteData(palette, colorSpace);
	const pool = Array.from(
		{ length: workerCount },
		() =>
			new Worker(WORKER_SOURCE, {
				eval: true,
				workerData: {
					sourceBuffer,
					width: source.width,
					height: source.height,
					paletteData,
					colorSpace
				}
			})
	);
	try {
		for (let i = 0; i < warmups; i++) await runWorkerIteration(pool, source, testCase);
		const runs = [];
		let checksum = '';
		for (let i = 0; i < iterations; i++) {
			const run = await runWorkerIteration(pool, source, testCase);
			runs.push(run.ms);
			checksum = run.checksum;
		}
		const meanMs = runs.reduce((a, b) => a + b, 0) / runs.length;
		const stddevMs = Math.sqrt(runs.reduce((a, b) => a + (b - meanMs) ** 2, 0) / runs.length);
		return {
			dither: testCase.dither,
			colorSpace,
			workerCount,
			pixels: source.width * source.height,
			meanMs,
			stddevMs,
			cv: meanMs ? stddevMs / meanMs : 0,
			checksum
		};
	} finally {
		await Promise.all(pool.map((worker) => worker.terminate()));
	}
}

async function runWorkerIteration(pool, source, testCase) {
	const outputBuffer = new SharedArrayBuffer(source.width * source.height);
	const start = performance.now();
	const rowsPerWorker = Math.ceil(source.height / pool.length);
	await Promise.all(
		pool.map(
			(worker, ordinal) =>
				new Promise((resolve, reject) => {
					const startY = ordinal * rowsPerWorker;
					const endY = Math.min(source.height, startY + rowsPerWorker);
					const onMessage = () => {
						cleanup();
						resolve();
					};
					const onError = (error) => {
						cleanup();
						reject(error);
					};
					const cleanup = () => {
						worker.off('message', onMessage);
						worker.off('error', onError);
					};
					worker.on('message', onMessage);
					worker.on('error', onError);
					worker.postMessage({
						startY,
						endY,
						outputBuffer,
						thresholds: testCase.thresholds,
						size: testCase.size
					});
				})
		)
	);
	const output = new Uint8Array(outputBuffer);
	let checksum = 0x811c9dc5;
	for (let i = 0; i < output.length; i++) {
		checksum ^= output[i] + 1;
		checksum = Math.imul(checksum, 0x01000193) >>> 0;
	}
	return { ms: performance.now() - start, checksum: checksum.toString(16).padStart(8, '0') };
}

function buildPaletteData(palette, colorSpace) {
	const count = palette.length;
	const indices = new Uint8Array(count);
	const v0 = new Float64Array(count);
	const v1 = new Float64Array(count);
	const v2 = new Float64Array(count);
	let min0 = Infinity,
		min1 = Infinity,
		min2 = Infinity,
		max0 = -Infinity,
		max1 = -Infinity,
		max2 = -Infinity;
	for (let i = 0; i < count; i++) {
		const rgb = palette[i].rgb;
		const vec = vectorForRgb(rgb.r, rgb.g, rgb.b, colorSpace);
		indices[i] = palette[i].index;
		v0[i] = vec[0];
		v1[i] = vec[1];
		v2[i] = vec[2];
		min0 = Math.min(min0, vec[0]);
		min1 = Math.min(min1, vec[1]);
		min2 = Math.min(min2, vec[2]);
		max0 = Math.max(max0, vec[0]);
		max1 = Math.max(max1, vec[1]);
		max2 = Math.max(max2, vec[2]);
	}
	return {
		count,
		indices,
		v0,
		v1,
		v2,
		ranges: [
			max0 - min0 || Number.EPSILON,
			max1 - min1 || Number.EPSILON,
			max2 - min2 || Number.EPSILON
		]
	};
}

function vectorForRgb(r, g, b, colorSpace) {
	if (colorSpace === 'weighted-rgb-601')
		return [r * Math.sqrt(0.299), g * Math.sqrt(0.587), b * Math.sqrt(0.114)];
	const rr = srgbByteToLinear(r),
		gg = srgbByteToLinear(g),
		bb = srgbByteToLinear(b);
	const l = 0.4122214708 * rr + 0.5363325363 * gg + 0.0514459929 * bb;
	const m = 0.2119034982 * rr + 0.6806995451 * gg + 0.1073969566 * bb;
	const s = 0.0883024619 * rr + 0.2817188376 * gg + 0.6299787005 * bb;
	const lr = Math.cbrt(l),
		mr = Math.cbrt(m),
		sr = Math.cbrt(s);
	return [
		0.2104542553 * lr + 0.793617785 * mr - 0.0040720468 * sr,
		1.9779984951 * lr - 2.428592205 * mr + 0.4505937099 * sr,
		0.0259040371 * lr + 0.7827717662 * mr - 0.808675766 * sr
	];
}
function srgbByteToLinear(v) {
	const x = v / 255;
	return x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4;
}

async function decodeBenchmarkImage(browser, imagePath) {
	const bytes = await readFile(imagePath);
	const page = await browser.newPage();
	try {
		const decoded = await page.evaluate(
			async (url) => {
				const image = new Image();
				const loaded = new Promise((resolve, reject) => {
					image.onload = resolve;
					image.onerror = reject;
				});
				image.src = url;
				await loaded;
				const canvas = document.createElement('canvas');
				canvas.width = image.naturalWidth;
				canvas.height = image.naturalHeight;
				const context = canvas.getContext('2d', { willReadFrequently: true });
				context.drawImage(image, 0, 0);
				const data = context.getImageData(0, 0, canvas.width, canvas.height);
				window.__rowWorkerImageBytes = data.data;
				return { width: data.width, height: data.height, length: data.data.length };
			},
			`data:image/png;base64,${bytes.toString('base64')}`
		);
		const data = new Uint8ClampedArray(decoded.length);
		const chunkSize = 1 << 20;
		for (let start = 0; start < decoded.length; start += chunkSize) {
			const chunk = await page.evaluate(
				({ start, end }) => Array.from(window.__rowWorkerImageBytes.slice(start, end)),
				{ start, end: Math.min(decoded.length, start + chunkSize) }
			);
			data.set(chunk, start);
		}
		return new ImageData(data, decoded.width, decoded.height);
	} finally {
		await page.close();
	}
}

function rowsToCsv(rows) {
	return [
		'dither,colorSpace,workerCount,pixels,meanMs,stddevMs,cv,checksum',
		...rows.map((r) =>
			[
				r.dither,
				r.colorSpace,
				r.workerCount,
				r.pixels,
				r.meanMs,
				r.stddevMs,
				r.cv,
				r.checksum
			].join(',')
		)
	].join('\n');
}
function formatTable(rows) {
	const table = [
		['dither', 'color', 'workers', 'mean', 'cv', 'checksum'],
		...rows.map((r) => [
			r.dither,
			r.colorSpace,
			String(r.workerCount),
			`${r.meanMs.toFixed(0)}ms`,
			`${(r.cv * 100).toFixed(1)}%`,
			r.checksum
		])
	];
	const widths = table[0].map((_, c) => Math.max(...table.map((r) => r[c].length)));
	return table.map((r) => r.map((v, c) => v.padEnd(widths[c])).join('  ')).join('\n');
}
function parseArgs(argv) {
	const o = {};
	for (let i = 0; i < argv.length; i++) {
		const [k, inline] = argv[i].slice(2).split('=', 2);
		const next = () => inline ?? argv[++i];
		if (k === 'image') o.image = next();
		else if (k === 'out') o.out = next();
		else if (k === 'iterations') o.iterations = Number.parseInt(next(), 10);
		else if (k === 'warmups') o.warmups = Number.parseInt(next(), 10);
		else if (k === 'workers') o.workers = next().split(',').map(Number);
		else if (k === 'dither') o.dithers = next().split(',');
		else if (k === 'color-space') o.colorSpaces = next().split(',');
	}
	return o;
}
function installImageDataPolyfill() {
	if ('ImageData' in globalThis) return;
	globalThis.ImageData = class ImageData {
		constructor(data, width, height) {
			this.data = data;
			this.width = width;
			this.height = height;
		}
	};
}
