#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { existsSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

installImageDataPolyfill();

const DEFAULT_SCALES = [0.125, 0.25, 0.5, 0.75];
const DEFAULT_ITERATIONS = 5;
const DEFAULT_WARMUPS = 2;
const DEFAULT_MODES = ['lanczos3', 'lanczos2'];
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const args = parseArgs(process.argv.slice(2));
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const outDir = path.resolve(
	root,
	args.out ?? `benchmark-results/lanczos-radius-study-${timestamp}`
);
let browser;

try {
	await mkdir(outDir, { recursive: true });
	const correctness = args.checkCorrectness ? runCorrectnessCheck(outDir) : { skipped: true };
	const sources = await benchmarkSources(root, args);
	const cases = speedCases(sources, args);
	const startedAt = new Date().toISOString();
	const server = await createServer({
		root,
		configFile: path.join(root, 'vite.config.ts'),
		appType: 'custom',
		logLevel: 'silent',
		server: { middlewareMode: true, hmr: false }
	});
	try {
		const resizeModule = await server.ssrLoadModule(
			`/src/lib/processing/resize.ts?radius-study=${Date.now()}`
		);
		const runs = [];
		for (const mode of args.modes) {
			console.log(`\n=== ${mode} ===`);
			const result = runMode(mode, resizeModule.resizeImageData, cases, args);
			runs.push(result);
			await writeModeArtifacts(outDir, mode, result);
			writeStudySummary(outDir, startedAt, args, correctness, runs);
			console.log(formatModeSummary(result));
		}
		writeStudySummary(outDir, startedAt, args, correctness, runs);
	} finally {
		await server.close();
	}

	console.log(`\nWrote ${path.relative(root, path.join(outDir, 'study-summary.json'))}`);
	console.log(`Wrote ${path.relative(root, path.join(outDir, 'study-summary.csv'))}`);
} finally {
	await browser?.close();
}

function parseArgs(argv) {
	const options = {
		iterations: DEFAULT_ITERATIONS,
		warmups: DEFAULT_WARMUPS,
		scales: DEFAULT_SCALES,
		modes: DEFAULT_MODES,
		images: [],
		fixtureDirs: [],
		maxFixtures: Infinity,
		includeSynthetic: true,
		checkCorrectness: false,
		quiet: false
	};

	for (let index = 0; index < argv.length; index++) {
		const arg = argv[index];
		if (!arg.startsWith('--')) throw new Error(`Unexpected argument: ${arg}`);
		const name = arg.slice(2);
		const value = () => {
			const next = argv[++index];
			if (!next) throw new Error(`Missing value for ${arg}`);
			return next;
		};
		switch (name) {
			case 'out':
				options.out = value();
				break;
			case 'iterations':
				options.iterations = Math.max(1, Math.floor(Number(value())));
				break;
			case 'warmups':
				options.warmups = Math.max(0, Math.floor(Number(value())));
				break;
			case 'scales':
				options.scales = value()
					.split(',')
					.map((item) => Number(item.trim()))
					.filter((item) => Number.isFinite(item) && item > 0);
				break;
			case 'modes':
				options.modes = value()
					.split(',')
					.map((item) => item.trim())
					.filter(Boolean);
				break;
			case 'image':
				options.images.push(value());
				break;
			case 'fixtures-dir':
				options.fixtureDirs.push(value());
				break;
			case 'max-fixtures':
				options.maxFixtures = Math.max(1, Math.floor(Number(value())));
				break;
			case 'synthetic':
				options.includeSynthetic = optionalBoolean(argv, index, true);
				if (argv[index + 1] && !argv[index + 1].startsWith('--')) index++;
				break;
			case 'check-correctness':
				options.checkCorrectness = true;
				break;
			case 'quiet':
				options.quiet = optionalBoolean(argv, index, true);
				if (argv[index + 1] && !argv[index + 1].startsWith('--')) index++;
				break;
			case 'help':
			case 'h':
				printHelp();
				process.exit(0);
				break;
			default:
				throw new Error(`Unknown option: ${arg}`);
		}
	}
	return options;
}

function optionalBoolean(argv, index, defaultValue) {
	const next = argv[index + 1];
	if (!next || next.startsWith('--')) return defaultValue;
	return next !== 'false';
}

function printHelp() {
	console.log(`Usage: pnpm bench:lanczos-radius-study -- [options]

Compares Lanczos radius modes, usually lanczos3 vs lanczos2, on the same resize-only matrix.
This intentionally compares output-changing filter choices, not exact implementation variants.

Options:
  --iterations N          Recorded iterations per case (default: ${DEFAULT_ITERATIONS})
  --warmups N             Warmup iterations per case (default: ${DEFAULT_WARMUPS})
  --scales a,b            Output-size scales to test (default: ${DEFAULT_SCALES.join(',')})
  --modes a,b             Resize modes to compare (default: ${DEFAULT_MODES.join(',')})
  --image FILE            Include an image file
  --fixtures-dir DIR      Decode all fixtures from a directory (default: benchmark-fixtures/)
  --max-fixtures N        Limit decoded real fixtures
  --synthetic true|false  Include deterministic opaque/alpha synthetic sources (default: true)
  --check-correctness     Run resize.spec.ts before benchmarking
  --quiet true|false      Hide per-case progress (default: false)
  --out DIR               Artifact directory

Examples:
  pnpm bench:lanczos-radius-study -- --iterations 10 --warmups 3 --quiet true --check-correctness
`);
}

async function benchmarkSources(rootDir, options) {
	const sources = [];
	if (options.includeSynthetic) {
		sources.push(syntheticSource('synthetic-opaque-2mp', 1920, 1080, false));
		sources.push(syntheticSource('synthetic-alpha-2mp', 1920, 1080, true));
	}

	const imagePaths = await benchmarkImagePaths(rootDir, options);
	if (imagePaths.length) {
		browser = await chromium.launch({ headless: true });
		for (const imagePath of imagePaths.slice(0, options.maxFixtures)) {
			console.log(`Decoding ${path.relative(rootDir, imagePath)}...`);
			sources.push(await decodeBenchmarkImage(browser, rootDir, imagePath));
		}
	}
	if (!sources.length) throw new Error('No Lanczos radius-study sources available.');
	return sources;
}

async function benchmarkImagePaths(rootDir, options) {
	const explicit = options.images.map((imagePath) => path.resolve(rootDir, imagePath));
	const fixtureDirs = options.fixtureDirs.length
		? options.fixtureDirs
		: [existsSync(path.join(rootDir, 'benchmark-fixtures')) ? 'benchmark-fixtures' : undefined];
	const fixtureImages = (
		await Promise.all(
			fixtureDirs
				.filter(Boolean)
				.map((fixtureDir) => imageFilesInDirectory(path.resolve(rootDir, fixtureDir)))
		)
	).flat();
	return [...new Set([...explicit, ...fixtureImages])];
}

async function imageFilesInDirectory(directory) {
	const entries = await readdir(directory, { withFileTypes: true });
	const files = await Promise.all(
		entries.map(async (entry) => {
			const entryPath = path.join(directory, entry.name);
			if (entry.isDirectory()) return imageFilesInDirectory(entryPath);
			return isSupportedImagePath(entryPath) ? [entryPath] : [];
		})
	);
	return files.flat().sort();
}

async function decodeBenchmarkImage(browserInstance, rootDir, imagePath) {
	const start = performance.now();
	const bytes = await readFile(imagePath);
	const dataUrl = `data:${mimeForPath(imagePath)};base64,${bytes.toString('base64')}`;
	const page = await browserInstance.newPage();
	try {
		const decoded = await page.evaluate(async (url) => {
			const image = new Image();
			image.decoding = 'async';
			image.src = url;
			await image.decode();
			const canvas = document.createElement('canvas');
			canvas.width = image.naturalWidth;
			canvas.height = image.naturalHeight;
			const context = canvas.getContext('2d', { willReadFrequently: true });
			if (!context) throw new Error('Unable to create a 2D canvas context.');
			context.drawImage(image, 0, 0);
			const imageData = context.getImageData(0, 0, canvas.width, canvas.height);
			return { width: imageData.width, height: imageData.height, data: imageData.data };
		}, dataUrl);
		const data =
			decoded.data instanceof Uint8ClampedArray
				? decoded.data
				: new Uint8ClampedArray(decoded.data);
		return {
			id: safeId(path.basename(imagePath, path.extname(imagePath))),
			label: path.relative(rootDir, imagePath),
			kind: 'image',
			path: path.relative(rootDir, imagePath),
			decodeMs: performance.now() - start,
			imageData: new ImageData(data, decoded.width, decoded.height)
		};
	} finally {
		await page.close();
	}
}

function syntheticSource(id, width, height, withAlpha) {
	const data = new Uint8ClampedArray(width * height * 4);
	for (let y = 0; y < height; y++) {
		for (let x = 0; x < width; x++) {
			const offset = (y * width + x) * 4;
			data[offset] = (x * 47 + y * 11) & 255;
			data[offset + 1] = (x * 7 + y * 53) & 255;
			data[offset + 2] = (x * y * 19 + 31) & 255;
			data[offset + 3] = withAlpha && (x + y) % 7 === 0 ? 96 : 255;
		}
	}
	return { id, label: id, kind: 'synthetic', imageData: new ImageData(data, width, height) };
}

function speedCases(sources, options) {
	return sources.flatMap((source) =>
		options.scales.map((scale) => {
			const dimensions = outputDimensions(source.imageData, scale);
			return {
				id: `${source.id}-scale-${scaleLabel(scale)}`,
				source,
				scale,
				width: dimensions.width,
				height: dimensions.height
			};
		})
	);
}

function outputDimensions(source, scale) {
	return {
		width: Math.max(1, Math.round(source.width * scale)),
		height: Math.max(1, Math.round(source.height * scale))
	};
}

function runMode(mode, resizeImageData, cases, options) {
	const results = [];
	let caseIndex = 0;
	for (const testCase of cases) {
		caseIndex++;
		if (!options.quiet) {
			console.log(
				`[${caseIndex}/${cases.length}] ${testCase.id} ${testCase.width}×${testCase.height}`
			);
		}
		for (let warmup = 0; warmup < options.warmups; warmup++)
			runResize(resizeImageData, mode, testCase);
		const runs = [];
		for (let iteration = 0; iteration < options.iterations; iteration++) {
			const start = performance.now();
			runResize(resizeImageData, mode, testCase);
			runs.push(performance.now() - start);
		}
		results.push({
			case: {
				id: testCase.id,
				scale: testCase.scale,
				width: testCase.width,
				height: testCase.height
			},
			source: {
				id: testCase.source.id,
				label: testCase.source.label,
				kind: testCase.source.kind,
				width: testCase.source.imageData.width,
				height: testCase.source.imageData.height
			},
			iterations: options.iterations,
			runs,
			stats: stats(runs)
		});
	}
	return {
		version: 1,
		mode,
		runAt: new Date().toISOString(),
		iterations: options.iterations,
		warmups: options.warmups,
		results,
		summary: summarizeResults(results)
	};
}

function runResize(resizeImageData, mode, testCase) {
	return resizeImageData(testCase.source.imageData, testCase.width, testCase.height, mode);
}

function stats(values) {
	const sorted = [...values].sort((a, b) => a - b);
	return {
		minMs: sorted[0] ?? 0,
		meanMs: sum(sorted, (value) => value) / sorted.length,
		medianMs: percentile(sorted, 0.5),
		p95Ms: percentile(sorted, 0.95),
		maxMs: sorted.at(-1) ?? 0
	};
}

function percentile(sorted, p) {
	if (!sorted.length) return 0;
	const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * p) - 1);
	return sorted[index] ?? 0;
}

function summarizeResults(results) {
	const byScale = {};
	const bySource = {};
	for (const result of results) {
		addSummary(byScale, scaleLabel(result.case.scale), result.stats.meanMs);
		addSummary(bySource, result.source.id, result.stats.meanMs);
	}
	return {
		resultCount: results.length,
		resizeMs: sum(results, (result) => result.stats.meanMs),
		byScale,
		bySource
	};
}

function addSummary(bucket, key, meanMs) {
	bucket[key] ??= { count: 0, resizeMs: 0 };
	bucket[key].count++;
	bucket[key].resizeMs += meanMs;
}

async function writeModeArtifacts(directory, mode, result) {
	const modeDir = path.join(directory, mode);
	await mkdir(modeDir, { recursive: true });
	await writeFile(
		path.join(modeDir, 'lanczos-radius.json'),
		`${JSON.stringify(result, null, 2)}\n`
	);
	await writeFile(path.join(modeDir, 'lanczos-radius.csv'), modeCsv(result));
}

function writeStudySummary(directory, startedAt, options, correctness, runs) {
	const serializableRuns = runs.map((run) => ({
		mode: run.mode,
		summary: run.summary
	}));
	writeFileSync(
		path.join(directory, 'study-summary.json'),
		`${JSON.stringify(
			{
				startedAt,
				iterations: options.iterations,
				warmups: options.warmups,
				scales: options.scales,
				modes: options.modes,
				correctness: correctness.skipped
					? { skipped: true }
					: { passed: correctness.passed, status: correctness.status },
				environment: {
					node: process.version,
					platform: `${process.platform} ${process.arch}`,
					cpus: os.cpus().map((cpu) => cpu.model),
					totalMemoryBytes: os.totalmem()
				},
				runs: serializableRuns
			},
			null,
			2
		)}\n`
	);
	writeFileSync(
		path.join(directory, 'study-summary.csv'),
		studyCsv(serializableRuns, options, correctness)
	);
}

function studyCsv(runs, options, correctness) {
	const scaleLabels = options.scales.map(scaleLabel);
	const baseline = runs[0]?.summary.resizeMs;
	const rows = [
		[
			'mode',
			'correctness',
			'results',
			'resizeMs',
			'relativeToFirst',
			...scaleLabels.map((scale) => `scale${scale}ResizeMs`)
		]
	];
	for (const run of runs) {
		rows.push([
			run.mode,
			correctness.skipped ? 'skipped' : correctness.passed ? 'passed' : 'failed',
			run.summary.resultCount,
			run.summary.resizeMs,
			baseline ? run.summary.resizeMs / baseline : 1,
			...scaleLabels.map((scale) => run.summary.byScale[scale]?.resizeMs ?? 0)
		]);
	}
	return `${rows.map((row) => row.map(csvCell).join(',')).join('\n')}\n`;
}

function modeCsv(result) {
	const rows = [
		[
			'case',
			'source',
			'sourceKind',
			'scale',
			'width',
			'height',
			'minMs',
			'meanMs',
			'medianMs',
			'p95Ms',
			'maxMs'
		]
	];
	for (const row of result.results) {
		rows.push([
			row.case.id,
			row.source.id,
			row.source.kind,
			row.case.scale,
			row.case.width,
			row.case.height,
			row.stats.minMs,
			row.stats.meanMs,
			row.stats.medianMs,
			row.stats.p95Ms,
			row.stats.maxMs
		]);
	}
	return `${rows.map((row) => row.map(csvCell).join(',')).join('\n')}\n`;
}

function formatModeSummary(result) {
	return `${result.mode}: ${result.summary.resultCount} cases ${formatMs(result.summary.resizeMs)}`;
}

function formatMs(value) {
	return `${value.toFixed(value >= 100 ? 0 : 1)}ms`;
}

function runCorrectnessCheck(directory) {
	const result = spawnSync(
		'pnpm',
		['test:unit', '--', '--run', 'src/lib/processing/resize.spec.ts'],
		{
			cwd: root,
			encoding: 'utf8'
		}
	);
	const correctness = {
		passed: result.status === 0,
		status: result.status,
		stdout: result.stdout,
		stderr: result.stderr
	};
	writeFileSync(
		path.join(directory, 'correctness.log'),
		`${result.stdout ?? ''}\n${result.stderr ?? ''}`
	);
	if (!correctness.passed) throw new Error('Resize correctness failed. See correctness.log.');
	return correctness;
}

function sum(values, getter) {
	return values.reduce((total, value) => total + getter(value), 0);
}

function csvCell(value) {
	const text = String(value);
	return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

function scaleLabel(scale) {
	return String(scale).replace('0.', '').replace('.', '_');
}

function safeId(value) {
	return value.replaceAll(/[^a-z0-9._-]+/gi, '-').replaceAll(/^-|-$/g, '');
}

function isSupportedImagePath(filePath) {
	return ['.png', '.jpg', '.jpeg', '.webp', '.gif'].includes(path.extname(filePath).toLowerCase());
}

function mimeForPath(filePath) {
	switch (path.extname(filePath).toLowerCase()) {
		case '.jpg':
		case '.jpeg':
			return 'image/jpeg';
		case '.webp':
			return 'image/webp';
		case '.gif':
			return 'image/gif';
		case '.png':
			return 'image/png';
		default:
			throw new Error(`Unsupported benchmark image type: ${filePath}`);
	}
}

function installImageDataPolyfill() {
	if (typeof globalThis.ImageData !== 'undefined') return;
	globalThis.ImageData = class ImageDataPolyfill {
		constructor(dataOrWidth, widthOrHeight, height) {
			if (typeof dataOrWidth === 'number') {
				this.width = dataOrWidth;
				this.height = widthOrHeight;
				this.data = new Uint8ClampedArray(this.width * this.height * 4);
				this.colorSpace = 'srgb';
				return;
			}
			this.data = dataOrWidth;
			this.width = widthOrHeight;
			this.height = height ?? dataOrWidth.length / 4 / widthOrHeight;
			this.colorSpace = 'srgb';
		}
	};
}
