#!/usr/bin/env node
import { createServer } from 'node:http';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = parseArgs(process.argv.slice(2));
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const outDir = path.resolve(root, args.out ?? `benchmark-results/wasm-resize-${timestamp}`);
const pkgDir = path.join(root, 'crates/ditherette-wasm/pkg');
const resizeScales = [2, 0.95, 0.75, 0.5, 0.25, 0.125];
const significantNumberFormatter = new Intl.NumberFormat('en-US', {
	maximumSignificantDigits: 4,
	minimumSignificantDigits: 4,
	useGrouping: false
});

await assertFile(
	path.join(pkgDir, 'ditherette_wasm.js'),
	'Run `pnpm wasm:build` before benchmarking.'
);
await assertFile(
	path.join(pkgDir, 'ditherette_wasm_bg.wasm'),
	'Run `pnpm wasm:build` before benchmarking.'
);

const fixtureFile = await benchmarkFixtureFile(args);
const server = await startBenchmarkServer({ pkgDir, fixtureFile });
const browser = await chromium.launch({ headless: true });
let statusLineActive = false;

try {
	const page = await browser.newPage();
	page.on('console', (message) => {
		if (message.type() !== 'debug') return;
		const text = message.text();
		if (!text.startsWith('bench-progress ')) return;
		writeStatusLine(text.slice('bench-progress '.length));
	});
	await page.goto(server.url, { waitUntil: 'load' });

	const result = await page.evaluate(async (config) => globalThis.runResizeBench(config), {
		iterations: args.iterations,
		warmups: args.warmups,
		fixture: {
			name: path.basename(fixtureFile),
			url: `/fixtures/${encodeURIComponent(path.basename(fixtureFile))}`
		},
		scales: resizeScales
	});

	await mkdir(outDir, { recursive: true });
	const jsonPath = path.join(outDir, 'wasm-resize-nearest.json');
	await writeFile(jsonPath, `${JSON.stringify(result, null, 2)}\n`);

	clearStatusLine();
	console.log(formatResultTable(result));
	console.log(`\nWrote ${path.relative(root, jsonPath)}`);
} finally {
	clearStatusLine();
	await browser.close();
	await new Promise((resolve, reject) => {
		server.instance.close((error) => (error ? reject(error) : resolve()));
	});
}

function writeStatusLine(message) {
	const status = `Benchmark ${message}`;
	if (!process.stdout.isTTY) {
		console.error(status);
		return;
	}

	process.stderr.write(`\r${status}\x1b[K`);
	statusLineActive = true;
}

function clearStatusLine() {
	if (!statusLineActive || !process.stderr.isTTY) return;
	process.stderr.write('\r\x1b[K');
	statusLineActive = false;
}

async function benchmarkFixtureFile(options) {
	const fixture = path.resolve(
		root,
		options.image ?? 'benchmark-fixtures/Celeste_box_art_full.png'
	);
	await assertFile(fixture, `Benchmark fixture not found: ${fixture}`);
	return fixture;
}

async function startBenchmarkServer({ pkgDir, fixtureFile }) {
	const server = createServer(async (request, response) => {
		try {
			const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');

			if (requestUrl.pathname === '/') {
				writeBenchmarkHeaders(response, 'text/html; charset=utf-8');
				response.end(benchmarkHtml());
				return;
			}

			if (requestUrl.pathname === '/bench/resize.js') {
				writeBenchmarkHeaders(response, 'text/javascript; charset=utf-8');
				response.end(benchmarkBrowserModule());
				return;
			}

			if (requestUrl.pathname.startsWith('/pkg/')) {
				const fileName = path.basename(requestUrl.pathname);
				await serveFile(response, path.join(pkgDir, fileName));
				return;
			}

			if (requestUrl.pathname.startsWith('/fixtures/')) {
				await serveFile(response, fixtureFile);
				return;
			}

			response.writeHead(404);
			response.end('Not found');
		} catch (error) {
			response.writeHead(500, { 'content-type': 'text/plain; charset=utf-8' });
			response.end(error instanceof Error ? error.stack : String(error));
		}
	});

	await new Promise((resolve, reject) => {
		server.once('error', reject);
		server.listen(0, '127.0.0.1', () => resolve());
	});

	const address = server.address();
	if (!address || typeof address === 'string') throw new Error('Failed to bind benchmark server');

	return { instance: server, url: `http://127.0.0.1:${address.port}/` };
}

async function serveFile(response, filePath) {
	const contents = await readFile(filePath);
	writeBenchmarkHeaders(response, contentType(filePath));
	response.end(contents);
}

function writeBenchmarkHeaders(response, contentType) {
	response.writeHead(200, {
		'content-type': contentType,
		'cross-origin-opener-policy': 'same-origin',
		'cross-origin-embedder-policy': 'require-corp',
		'cross-origin-resource-policy': 'same-origin'
	});
}

function benchmarkHtml() {
	return `<!doctype html>
<html lang="en">
	<head><meta charset="utf-8"><title>Ditherette Wasm Resize Benchmark</title></head>
	<body><script type="module" src="/bench/resize.js"></script></body>
</html>`;
}

function benchmarkBrowserModule() {
	return String.raw`import init, {
	resize_rgba_nearest_into,
	resize_rgba_nearest_reference
} from '/pkg/ditherette_wasm.js';

const RGBA_CHANNEL_COUNT = 4;

const resizeVariants = [
	{ id: 'reference', kind: 'allocating', resize: resize_rgba_nearest_reference },
	{ id: 'baseline', kind: 'reused-output', resizeInto: resize_rgba_nearest_into }
];

globalThis.runResizeBench = async function runResizeBench(config) {
	await init('/pkg/ditherette_wasm_bg.wasm');

	const decodedFixture = await decodeFixture(config.fixture);
	const cases = makeScaleCases(decodedFixture, config.scales);

	const totalRuns = cases.length * resizeVariants.length;
	let completedRuns = 0;
	const results = [];
	reportProgress(completedRuns, totalRuns, 'starting');

	for (const benchmarkCase of cases) {
		for (const variant of resizeVariants) {
			reportProgress(completedRuns, totalRuns, benchmarkCase.id + ' / ' + variant.id);
			results.push(await measureResizeVariant(benchmarkCase, variant, config));
			completedRuns += 1;
			reportProgress(completedRuns, totalRuns, benchmarkCase.id + ' / ' + variant.id);
		}
	}

	assertMatchingVariantChecksums(results);

	return {
		benchmark: 'wasm-resize-nearest',
		crossOriginIsolated: globalThis.crossOriginIsolated,
		fixture: {
			name: config.fixture.name,
			width: decodedFixture.sourceWidth,
			height: decodedFixture.sourceHeight,
			decodeNs: millisecondsToNanoseconds(decodedFixture.decodeMs),
			normalizeNs: millisecondsToNanoseconds(decodedFixture.normalizeMs)
		},
		timestamp: new Date().toISOString(),
		userAgent: navigator.userAgent,
		iterations: config.iterations,
		warmups: config.warmups,
		scales: config.scales,
		variants: resizeVariants.map((variant) => variant.id),
		results
	};
};

async function decodeFixture(fixture) {
	const decodeStarted = performance.now();
	const response = await fetch(fixture.url);
	if (!response.ok) throw new Error('Failed to fetch fixture ' + fixture.url + ': ' + response.status);

	const blob = await response.blob();
	const bitmap = await createImageBitmap(blob);
	const sourceWidth = bitmap.width;
	const sourceHeight = bitmap.height;
	const decodeMs = performance.now() - decodeStarted;

	const normalizeStarted = performance.now();
	const canvas = document.createElement('canvas');
	canvas.width = sourceWidth;
	canvas.height = sourceHeight;

	const context = canvas.getContext('2d', { willReadFrequently: true });
	if (!context) throw new Error('Canvas 2D is unavailable');

	context.drawImage(bitmap, 0, 0);
	const imageData = context.getImageData(0, 0, sourceWidth, sourceHeight);
	const sourceRgba = Uint8Array.from(imageData.data);
	const normalizeMs = performance.now() - normalizeStarted;
	bitmap.close?.();

	return { sourceWidth, sourceHeight, sourceRgba, decodeMs, normalizeMs };
}

function makeScaleCases(decodedFixture, scales) {
	const cases = [];

	for (const scale of scales) {
		const outputWidth = scaledDimension(decodedFixture.sourceWidth, scale);
		const outputHeight = scaledDimension(decodedFixture.sourceHeight, scale);
		const id = scaleLabel(scale);

		cases.push({
			id: id + '-browser-decode',
			lane: 'browser-decode-rgba',
			scale,
			sourceWidth: decodedFixture.sourceWidth,
			sourceHeight: decodedFixture.sourceHeight,
			outputWidth,
			outputHeight,
			sourceRgba: decodedFixture.sourceRgba,
			decodeMs: decodedFixture.decodeMs,
			normalizeMs: decodedFixture.normalizeMs
		});

		cases.push({
			id: id + '-decoded-rgba',
			lane: 'decoded-rgba',
			scale,
			sourceWidth: decodedFixture.sourceWidth,
			sourceHeight: decodedFixture.sourceHeight,
			outputWidth,
			outputHeight,
			sourceRgba: decodedFixture.sourceRgba,
			decodeMs: 0,
			normalizeMs: 0
		});
	}

	return cases;
}

async function measureResizeVariant(benchmarkCase, variant, config) {
	const expectedByteLength = benchmarkCase.outputWidth * benchmarkCase.outputHeight * RGBA_CHANNEL_COUNT;

	if (variant.kind === 'reused-output') {
		return measureIntoVariant(benchmarkCase, variant, config, expectedByteLength);
	}

	return measureAllocatingVariant(benchmarkCase, variant, config, expectedByteLength);
}

async function measureAllocatingVariant(benchmarkCase, variant, config, expectedByteLength) {
	for (let index = 0; index < config.warmups; index += 1) {
		variant.resize(
			benchmarkCase.sourceRgba,
			benchmarkCase.sourceWidth,
			benchmarkCase.sourceHeight,
			benchmarkCase.outputWidth,
			benchmarkCase.outputHeight
		);
	}

	const timingsMs = [];
	const checksums = [];
	let outputByteLength = 0;

	for (let index = 0; index < config.iterations; index += 1) {
		const started = performance.now();
		const outputRgba = variant.resize(
			benchmarkCase.sourceRgba,
			benchmarkCase.sourceWidth,
			benchmarkCase.sourceHeight,
			benchmarkCase.outputWidth,
			benchmarkCase.outputHeight
		);
		const wasmMs = performance.now() - started;

		outputByteLength = outputRgba.byteLength;
		timingsMs.push(wasmMs);
		checksums.push(checksumBytes(outputRgba));
	}

	return resizeResult(benchmarkCase, variant, timingsMs, checksums, outputByteLength, expectedByteLength);
}

async function measureIntoVariant(benchmarkCase, variant, config, expectedByteLength) {
	const outputRgba = new Uint8Array(expectedByteLength);

	for (let index = 0; index < config.warmups; index += 1) {
		variant.resizeInto(
			benchmarkCase.sourceRgba,
			benchmarkCase.sourceWidth,
			benchmarkCase.sourceHeight,
			benchmarkCase.outputWidth,
			benchmarkCase.outputHeight,
			outputRgba
		);
	}

	const timingsMs = [];
	const checksums = [];

	for (let index = 0; index < config.iterations; index += 1) {
		const started = performance.now();
		variant.resizeInto(
			benchmarkCase.sourceRgba,
			benchmarkCase.sourceWidth,
			benchmarkCase.sourceHeight,
			benchmarkCase.outputWidth,
			benchmarkCase.outputHeight,
			outputRgba
		);
		const wasmMs = performance.now() - started;

		timingsMs.push(wasmMs);
		checksums.push(checksumBytes(outputRgba));
	}

	return resizeResult(benchmarkCase, variant, timingsMs, checksums, outputRgba.byteLength, expectedByteLength);
}

function resizeResult(benchmarkCase, variant, timingsMs, checksums, outputByteLength, expectedByteLength) {
	const resultId = benchmarkCase.id + '-' + variant.id;

	if (outputByteLength !== expectedByteLength) {
		throw new Error(resultId + ' produced ' + outputByteLength + ' bytes; expected ' + expectedByteLength);
	}

	if (new Set(checksums).size !== 1) {
		throw new Error(resultId + ' produced unstable checksums: ' + checksums.join(', '));
	}

	return {
		id: benchmarkCase.id,
		variant: variant.id,
		lane: benchmarkCase.lane,
		scale: benchmarkCase.scale,
		source: { width: benchmarkCase.sourceWidth, height: benchmarkCase.sourceHeight },
		output: { width: benchmarkCase.outputWidth, height: benchmarkCase.outputHeight, byteLength: outputByteLength },
		decodeNs: millisecondsToNanoseconds(benchmarkCase.decodeMs),
		normalizeNs: millisecondsToNanoseconds(benchmarkCase.normalizeMs),
		wasmNs: summarizeTimings(timingsMs),
		checksum: checksums[0]
	};
}

function assertMatchingVariantChecksums(results) {
	const baselineChecksums = new Map();

	for (const result of results) {
		const key = result.lane + ':' + result.scale;
		if (result.variant === 'baseline') {
			baselineChecksums.set(key, result.checksum);
		}
	}

	for (const result of results) {
		const key = result.lane + ':' + result.scale;
		const baselineChecksum = baselineChecksums.get(key);
		if (baselineChecksum !== result.checksum) {
			throw new Error(
				result.id + ' ' + result.variant + ' checksum ' + result.checksum + ' did not match baseline ' + baselineChecksum
			);
		}
	}
}

function reportProgress(completedRuns, totalRuns, label) {
	const remainingRuns = totalRuns - completedRuns;
	console.debug('bench-progress ' + completedRuns + '/' + totalRuns + ' complete · ' + remainingRuns + ' remaining · ' + label);
}

function scaledDimension(sourceDimension, scale) {
	return Math.max(1, Math.floor(sourceDimension * scale));
}

function scaleLabel(scale) {
	return Math.round(scale * 1000).toString().padStart(3, '0') + 'x';
}

function summarizeTimings(timingsMs) {
	const sorted = [...timingsMs].sort((left, right) => left - right);
	const total = sorted.reduce((sum, value) => sum + value, 0);

	return {
		mean: millisecondsToNanoseconds(total / sorted.length),
		median: millisecondsToNanoseconds(percentile(sorted, 50)),
		mode: modeNs(timingsMs),
		min: millisecondsToNanoseconds(sorted[0]),
		p1: millisecondsToNanoseconds(percentile(sorted, 1)),
		p2: millisecondsToNanoseconds(percentile(sorted, 2)),
		p5: millisecondsToNanoseconds(percentile(sorted, 5)),
		p25: millisecondsToNanoseconds(percentile(sorted, 25)),
		p50: millisecondsToNanoseconds(percentile(sorted, 50)),
		p75: millisecondsToNanoseconds(percentile(sorted, 75)),
		p95: millisecondsToNanoseconds(percentile(sorted, 95)),
		p98: millisecondsToNanoseconds(percentile(sorted, 98)),
		p99: millisecondsToNanoseconds(percentile(sorted, 99)),
		max: millisecondsToNanoseconds(sorted[sorted.length - 1]),
		samples: timingsMs.map(millisecondsToNanoseconds)
	};
}

function percentile(sortedTimingsMs, percentileValue) {
	if (sortedTimingsMs.length === 1) return sortedTimingsMs[0];

	const rank = (percentileValue / 100) * (sortedTimingsMs.length - 1);
	const lowerIndex = Math.floor(rank);
	const upperIndex = Math.ceil(rank);
	const weight = rank - lowerIndex;

	return sortedTimingsMs[lowerIndex] * (1 - weight) + sortedTimingsMs[upperIndex] * weight;
}

function modeNs(timingsMs) {
	const counts = new Map();

	for (const timing of timingsMs) {
		const nanoseconds = millisecondsToNanoseconds(timing);
		counts.set(nanoseconds, (counts.get(nanoseconds) ?? 0) + 1);
	}

	let mode = millisecondsToNanoseconds(timingsMs[0]);
	let modeCount = 0;

	for (const [timing, count] of counts) {
		if (count > modeCount || (count === modeCount && timing < mode)) {
			mode = timing;
			modeCount = count;
		}
	}

	return mode;
}

function checksumBytes(bytes) {
	let hash = 2166136261;
	for (let index = 0; index < bytes.length; index += 1) {
		hash ^= bytes[index];
		hash = Math.imul(hash, 16777619);
	}
	return hash >>> 0;
}

function millisecondsToNanoseconds(milliseconds) {
	return Math.round(milliseconds * 1_000_000);
}
`;
}
function formatResultTable(result) {
	const baselines = baselineResultsByCase(result.results);
	const rows = result.results.map((entry) => {
		const baseline = baselines.get(entry.id);

		return {
			case: entry.id,
			variant: entry.variant,
			lane: entry.lane,
			scale: `${entry.scale}×`,
			source: `${entry.source.width}×${entry.source.height}px`,
			output: `${entry.output.width}×${entry.output.height}px`,
			decode: formatDuration(entry.decodeNs),
			normalize: formatDuration(entry.normalizeNs),
			mean: formatDurationDelta(entry.wasmNs.mean, baseline?.wasmNs.mean),
			median: formatDurationDelta(entry.wasmNs.median, baseline?.wasmNs.median),
			mode: formatDurationDelta(entry.wasmNs.mode, baseline?.wasmNs.mode),
			min: formatDurationDelta(entry.wasmNs.min, baseline?.wasmNs.min),
			p1: formatDurationDelta(entry.wasmNs.p1, baseline?.wasmNs.p1),
			p2: formatDurationDelta(entry.wasmNs.p2, baseline?.wasmNs.p2),
			p5: formatDurationDelta(entry.wasmNs.p5, baseline?.wasmNs.p5),
			p25: formatDurationDelta(entry.wasmNs.p25, baseline?.wasmNs.p25),
			p50: formatDurationDelta(entry.wasmNs.p50, baseline?.wasmNs.p50),
			p75: formatDurationDelta(entry.wasmNs.p75, baseline?.wasmNs.p75),
			p95: formatDurationDelta(entry.wasmNs.p95, baseline?.wasmNs.p95),
			p98: formatDurationDelta(entry.wasmNs.p98, baseline?.wasmNs.p98),
			p99: formatDurationDelta(entry.wasmNs.p99, baseline?.wasmNs.p99),
			max: formatDurationDelta(entry.wasmNs.max, baseline?.wasmNs.max),
			checksum: entry.checksum
		};
	});

	return [
		`Fixture: ${result.fixture.name} (${result.fixture.width}×${result.fixture.height})`,
		`Browser: ${result.userAgent}`,
		`Cross-origin isolated: ${result.crossOriginIsolated}`,
		`Iterations: ${result.iterations}, warmups: ${result.warmups}`,
		table(rows)
	].join('\n');
}

function baselineResultsByCase(results) {
	return new Map(
		results.filter((result) => result.variant === 'baseline').map((result) => [result.id, result])
	);
}

function formatDurationDelta(nanoseconds, baselineNanoseconds) {
	const duration = formatDuration(nanoseconds);
	if (!baselineNanoseconds) return duration;

	const speedFactor = baselineNanoseconds / nanoseconds;
	return `${duration} (${formatSignificant(speedFactor)}×)`;
}

function formatDuration(nanoseconds) {
	if (nanoseconds === 0) return '0ns';
	if (!Number.isFinite(nanoseconds)) return String(nanoseconds);

	const absolute = Math.abs(nanoseconds);
	if (absolute >= 1_000_000) return `${formatSignificant(nanoseconds / 1_000_000)}ms`;
	if (absolute >= 1_000) return `${formatSignificant(nanoseconds / 1_000)}µs`;
	return `${formatSignificant(nanoseconds)}ns`;
}

function formatSignificant(value) {
	return significantNumberFormatter.format(value);
}

function table(rows) {
	if (!rows.length) return 'No benchmark cases ran.';

	const headers = Object.keys(rows[0]);
	const widths = Object.fromEntries(
		headers.map((header) => [
			header,
			Math.max(header.length, ...rows.map((row) => String(row[header]).length))
		])
	);

	const formatRow = (row) =>
		headers.map((header) => String(row[header]).padEnd(widths[header])).join('  ');
	return [
		formatRow(Object.fromEntries(headers.map((header) => [header, header]))),
		...rows.map(formatRow)
	].join('\n');
}

function parseArgs(rawArgs) {
	const options = {
		image: undefined,
		iterations: 5,
		warmups: 2,
		out: undefined
	};

	for (let index = 0; index < rawArgs.length; index += 1) {
		const arg = rawArgs[index];
		const nextValue = () => {
			index += 1;
			if (index >= rawArgs.length) throw new Error(`Missing value for ${arg}`);
			return rawArgs[index];
		};

		switch (arg) {
			case '--':
				break;
			case '--image':
				options.image = nextValue();
				break;
			case '--iterations':
				options.iterations = positiveInteger(nextValue(), arg);
				break;
			case '--warmups':
				options.warmups = positiveInteger(nextValue(), arg);
				break;
			case '--out':
				options.out = nextValue();
				break;
			case '--help':
				console.log(helpText());
				process.exit(0);
				break;
			default:
				throw new Error(`Unknown argument: ${arg}\n\n${helpText()}`);
		}
	}

	return options;
}

function positiveInteger(value, name) {
	const parsed = Number(value);
	if (!Number.isInteger(parsed) || parsed <= 0)
		throw new Error(`${name} must be a positive integer`);
	return parsed;
}

function helpText() {
	return `Usage: pnpm bench:wasm-resize -- [options]

Options:
  --image FILE    Browser-decode this fixture before benchmarking Wasm resize.
                  Defaults to benchmark-fixtures/Celeste_box_art_full.png.
  --iterations N  Recorded iterations per case. Default: 5.
  --warmups N     Warmup iterations per case. Default: 2.
  --out DIR       Output directory. Default: benchmark-results/wasm-resize-<timestamp>.

Scales:
  0.95x, 0.75x, 0.5x, 0.25x, 0.125x

Lanes:
  browser-decode-rgba  Decode + normalize fixture in browser, then time Wasm resize.
  decoded-rgba         Reuse the decoded RGBA bytes, then time Wasm resize only.

Examples:
  pnpm bench:wasm-resize
  pnpm bench:wasm-resize -- --image benchmark-fixtures/Celeste_box_art_full.png --iterations 10`;
}

async function assertFile(filePath, message) {
	if (!(await fileExists(filePath))) throw new Error(message);
}

async function fileExists(filePath) {
	try {
		const fileStat = await stat(filePath);
		return fileStat.isFile();
	} catch (error) {
		if (error?.code === 'ENOENT') return false;
		throw error;
	}
}

function contentType(filePath) {
	const extension = path.extname(filePath);
	if (extension === '.js') return 'text/javascript; charset=utf-8';
	if (extension === '.wasm') return 'application/wasm';
	if (extension === '.png') return 'image/png';
	if (extension === '.jpg' || extension === '.jpeg') return 'image/jpeg';
	if (extension === '.webp') return 'image/webp';
	return 'application/octet-stream';
}
