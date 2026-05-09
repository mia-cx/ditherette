#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

installImageDataPolyfill();

const RESIZE_PATH = 'src/lib/processing/resize.ts';
const DEFAULT_SCALES = [0.125, 0.25, 0.5, 0.75];
const DEFAULT_ITERATIONS = 5;
const DEFAULT_WARMUPS = 2;
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const VARIANTS = [
	{
		id: 'baseline',
		description: 'Committed bilinear path through generic resize loop',
		apply: (source) => source
	},
	{
		id: 'direct-loop',
		description: 'Mode-specific bilinear loop that writes output directly',
		apply: (source) => addBilinearBranch(source, DIRECT_BILINEAR_HELPERS, 'resizeBilinearDirect')
	},
	{
		id: 'direct-loop-global-opaque',
		description: 'Direct bilinear loop with whole-source opaque fast path',
		apply: (source) =>
			addBilinearBranch(source, GLOBAL_OPAQUE_BILINEAR_HELPERS, 'resizeBilinearGlobalOpaque')
	},
	{
		id: 'axis-tables-global-opaque',
		description: 'Precompute bilinear axis tables plus whole-source opaque fast path',
		apply: (source) =>
			addBilinearBranch(source, AXIS_TABLE_BILINEAR_HELPERS, 'resizeBilinearAxisTables')
	}
];

async function main() {
	const args = parseArgs(process.argv.slice(2));
	const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
	const outDir = path.resolve(
		root,
		args.out ?? `benchmark-results/bilinear-speed-study-${timestamp}`
	);
	const originalWorkingTree = readFileSync(path.join(root, RESIZE_PATH), 'utf8');
	const baseline = gitShow(`HEAD:${RESIZE_PATH}`);
	let browser;

	try {
		await mkdir(outDir, { recursive: true });
		const sources = await benchmarkSources(root, args, (launchedBrowser) => {
			browser = launchedBrowser;
		});
		const cases = speedCases(sources, args);
		const startedAt = new Date().toISOString();
		const runs = [];

		for (const variant of VARIANTS) {
			console.log(`\n=== ${variant.id}: ${variant.description} ===`);
			writeFileSync(path.join(root, RESIZE_PATH), variant.apply(baseline));
			const correctness = args.checkCorrectness
				? runCorrectnessCheck(outDir, variant.id)
				: { skipped: true };

			const server = await createServer({
				root,
				configFile: path.join(root, 'vite.config.ts'),
				appType: 'custom',
				logLevel: 'silent',
				server: { middlewareMode: true, hmr: false }
			});
			try {
				const resizeModule = await server.ssrLoadModule(
					`/src/lib/processing/resize.ts?variant=${variant.id}-${Date.now()}`
				);
				const result = runVariant(variant, resizeModule.resizeImageData, cases, args);
				runs.push({ ...variant, apply: undefined, correctness, result });
				await writeVariantArtifacts(outDir, variant.id, result);
				writeStudySummary(outDir, startedAt, args, runs);
				console.log(formatVariantSummary(result));
			} finally {
				await server.close();
			}
		}

		writeStudySummary(outDir, startedAt, args, runs);
		console.log(`\nWrote ${path.relative(root, path.join(outDir, 'study-summary.json'))}`);
		console.log(`Wrote ${path.relative(root, path.join(outDir, 'study-summary.csv'))}`);
	} finally {
		writeFileSync(path.join(root, RESIZE_PATH), originalWorkingTree);
		await browser?.close();
	}
}

function parseArgs(argv) {
	argv = argv.filter((arg) => arg !== '--');
	const options = {
		iterations: DEFAULT_ITERATIONS,
		warmups: DEFAULT_WARMUPS,
		scales: DEFAULT_SCALES,
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
	console.log(`Usage: pnpm bench:bilinear-speed-study -- [options]

Benchmarks exact-output bilinear implementation variants across the resize case matrix.
This does not test other resize filters.

Options:
  --iterations N          Recorded iterations per case (default: ${DEFAULT_ITERATIONS})
  --warmups N             Warmup iterations per case (default: ${DEFAULT_WARMUPS})
  --scales a,b            Output-size scales to test (default: ${DEFAULT_SCALES.join(',')})
  --image FILE            Include an image file
  --fixtures-dir DIR      Decode all fixtures from a directory (default: benchmark-fixtures/)
  --max-fixtures N        Limit decoded real fixtures
  --synthetic true|false  Include deterministic opaque/alpha synthetic sources (default: true)
  --check-correctness     Run exact resize.spec.ts for each variant
  --quiet true|false      Hide per-case progress (default: false)
  --out DIR               Artifact directory

Examples:
  pnpm bench:bilinear-speed-study -- --iterations 10 --warmups 3 --quiet true --check-correctness
`);
}

async function benchmarkSources(rootDir, options, onBrowserLaunch = () => {}) {
	const sources = [];
	if (options.includeSynthetic) {
		sources.push(syntheticSource('synthetic-opaque-2mp', 1920, 1080, false));
		sources.push(syntheticSource('synthetic-alpha-2mp', 1920, 1080, true));
	}

	const imagePaths = await benchmarkImagePaths(rootDir, options);
	if (imagePaths.length) {
		const browser = await chromium.launch({ headless: true });
		onBrowserLaunch(browser);
		for (const imagePath of imagePaths.slice(0, options.maxFixtures)) {
			console.log(`Decoding ${path.relative(rootDir, imagePath)}...`);
			sources.push(await decodeBenchmarkImage(browser, rootDir, imagePath));
		}
	}
	if (!sources.length) throw new Error('No bilinear speed-study sources available.');
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

function runVariant(variant, resizeImageData, cases, options) {
	const results = [];
	let caseIndex = 0;
	for (const testCase of cases) {
		caseIndex++;
		if (!options.quiet) {
			console.log(
				`[${caseIndex}/${cases.length}] ${testCase.id} ${testCase.width}×${testCase.height}`
			);
		}
		for (let warmup = 0; warmup < options.warmups; warmup++) runResize(resizeImageData, testCase);
		const runs = [];
		for (let iteration = 0; iteration < options.iterations; iteration++) {
			const start = performance.now();
			runResize(resizeImageData, testCase);
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
		variant: variant.id,
		description: variant.description,
		runAt: new Date().toISOString(),
		iterations: options.iterations,
		warmups: options.warmups,
		results,
		summary: summarizeResults(results)
	};
}

function runResize(resizeImageData, testCase) {
	return resizeImageData(testCase.source.imageData, testCase.width, testCase.height, 'bilinear');
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

async function writeVariantArtifacts(directory, variantId, result) {
	const variantDir = path.join(directory, variantId);
	await mkdir(variantDir, { recursive: true });
	await writeFile(
		path.join(variantDir, 'bilinear-speed.json'),
		`${JSON.stringify(result, null, 2)}\n`
	);
	await writeFile(path.join(variantDir, 'bilinear-speed.csv'), variantCsv(result));
}

function writeStudySummary(directory, startedAt, options, runs) {
	const serializableRuns = runs.map(({ id, description, outDir, correctness, result }) => ({
		id,
		description,
		outDir,
		correctness: correctness?.skipped
			? { skipped: true }
			: { passed: correctness?.passed, status: correctness?.status },
		summary: result.summary
	}));
	writeFileSync(
		path.join(directory, 'study-summary.json'),
		`${JSON.stringify(
			{
				startedAt,
				iterations: options.iterations,
				warmups: options.warmups,
				scales: options.scales,
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
	writeFileSync(path.join(directory, 'study-summary.csv'), studyCsv(serializableRuns, options));
}

function studyCsv(runs, options) {
	const scaleLabels = options.scales.map(scaleLabel);
	const rows = [
		[
			'variant',
			'correctness',
			'results',
			'resizeMs',
			...scaleLabels.map((scale) => `scale${scale}ResizeMs`)
		]
	];
	for (const run of runs) {
		rows.push([
			run.id,
			run.correctness.skipped ? 'skipped' : run.correctness.passed ? 'passed' : 'failed',
			run.summary.resultCount,
			run.summary.resizeMs,
			...scaleLabels.map((scale) => run.summary.byScale[scale]?.resizeMs ?? 0)
		]);
	}
	return `${rows.map((row) => row.map(csvCell).join(',')).join('\n')}\n`;
}

function variantCsv(result) {
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

function formatVariantSummary(result) {
	return `${result.variant}: ${result.summary.resultCount} cases ${formatMs(result.summary.resizeMs)}`;
}

function formatMs(value) {
	return `${value.toFixed(value >= 100 ? 0 : 1)}ms`;
}

function gitShow(ref) {
	const result = spawnSync('git', ['show', ref], { cwd: root, encoding: 'utf8' });
	if (result.status !== 0) throw new Error(result.stderr || `git show ${ref} failed`);
	return result.stdout;
}

function runCorrectnessCheck(directory, variantId) {
	const result = spawnSync(
		'pnpm',
		['test:unit', '--', '--run', 'src/lib/processing/resize.spec.ts'],
		{ cwd: root, encoding: 'utf8' }
	);
	const correctness = {
		passed: result.status === 0,
		status: result.status,
		stdout: result.stdout,
		stderr: result.stderr
	};
	writeFileSync(
		path.join(directory, `${variantId}-correctness.log`),
		`${result.stdout ?? ''}\n${result.stderr ?? ''}`
	);
	if (!correctness.passed) {
		console.warn(`${variantId} correctness failed; continuing so perf can still be measured.`);
	}
	return correctness;
}

function addBilinearBranch(source, helpers, helperName) {
	const branch = `\n\tif (mode === 'bilinear') {\n\t\t${helperName}(source, output, sourceRect);\n\t\treturn output;\n\t}\n`;
	return insertBefore(
		insertBefore(source, '\n\tfor (let y = targetTop;', branch),
		'\nfunction sample(',
		`\n${helpers}\n`
	);
}

function insertBefore(source, marker, insertion) {
	if (!source.includes(marker)) throw new Error(`Could not find marker: ${marker}`);
	return source.replace(marker, `${insertion}${marker}`);
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

const DIRECT_BILINEAR_HELPERS = `function resizeBilinearDirect(source: ImageData, output: ImageData, sourceRect: Rect) {
	const outputData = output.data;
	const data = source.data;
	const scaleX = sourceRect.width / output.width;
	const scaleY = sourceRect.height / output.height;
	for (let y = 0; y < output.height; y++) {
		const sourceY = sourceRect.y + (y + 0.5) * scaleY - 0.5;
		const y0 = Math.floor(sourceY);
		const y1 = y0 + 1;
		const clampedY0 = Math.min(source.height - 1, Math.max(0, y0));
		const clampedY1 = Math.min(source.height - 1, Math.max(0, y1));
		const ty = sourceY - y0;
		for (let x = 0; x < output.width; x++) {
			const sourceX = sourceRect.x + (x + 0.5) * scaleX - 0.5;
			const x0 = Math.floor(sourceX);
			const x1 = x0 + 1;
			const clampedX0 = Math.min(source.width - 1, Math.max(0, x0));
			const clampedX1 = Math.min(source.width - 1, Math.max(0, x1));
			const tx = sourceX - x0;
			const weight00 = (1 - tx) * (1 - ty);
			const weight10 = tx * (1 - ty);
			const weight01 = (1 - tx) * ty;
			const weight11 = tx * ty;
			const offset00 = (clampedY0 * source.width + clampedX0) * 4;
			const offset10 = (clampedY0 * source.width + clampedX1) * 4;
			const offset01 = (clampedY1 * source.width + clampedX0) * 4;
			const offset11 = (clampedY1 * source.width + clampedX1) * 4;
			const total = weight00 + weight10 + weight01 + weight11;
			const targetOffset = (y * output.width + x) * 4;

			if (
				data[offset00 + 3] === 255 &&
				data[offset10 + 3] === 255 &&
				data[offset01 + 3] === 255 &&
				data[offset11 + 3] === 255
			) {
				outputData[targetOffset] = clampByte(
					(data[offset00]! * weight00 +
						data[offset10]! * weight10 +
						data[offset01]! * weight01 +
						data[offset11]! * weight11) /
						total
				);
				outputData[targetOffset + 1] = clampByte(
					(data[offset00 + 1]! * weight00 +
						data[offset10 + 1]! * weight10 +
						data[offset01 + 1]! * weight01 +
						data[offset11 + 1]! * weight11) /
						total
				);
				outputData[targetOffset + 2] = clampByte(
					(data[offset00 + 2]! * weight00 +
						data[offset10 + 2]! * weight10 +
						data[offset01 + 2]! * weight01 +
						data[offset11 + 2]! * weight11) /
						total
				);
				outputData[targetOffset + 3] = 255;
				continue;
			}

			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;
			let alpha = data[offset00 + 3]! / 255;
			r += data[offset00]! * alpha * weight00;
			g += data[offset00 + 1]! * alpha * weight00;
			b += data[offset00 + 2]! * alpha * weight00;
			a += data[offset00 + 3]! * weight00;

			alpha = data[offset10 + 3]! / 255;
			r += data[offset10]! * alpha * weight10;
			g += data[offset10 + 1]! * alpha * weight10;
			b += data[offset10 + 2]! * alpha * weight10;
			a += data[offset10 + 3]! * weight10;

			alpha = data[offset01 + 3]! / 255;
			r += data[offset01]! * alpha * weight01;
			g += data[offset01 + 1]! * alpha * weight01;
			b += data[offset01 + 2]! * alpha * weight01;
			a += data[offset01 + 3]! * weight01;

			alpha = data[offset11 + 3]! / 255;
			r += data[offset11]! * alpha * weight11;
			g += data[offset11 + 1]! * alpha * weight11;
			b += data[offset11 + 2]! * alpha * weight11;
			a += data[offset11 + 3]! * weight11;

			if (total === 0) {
				outputData[targetOffset] = 0;
				outputData[targetOffset + 1] = 0;
				outputData[targetOffset + 2] = 0;
				outputData[targetOffset + 3] = 0;
				continue;
			}
			const [outR, outG, outB, outA] = unpremultiplySample(
				r / total,
				g / total,
				b / total,
				a / total
			);
			outputData[targetOffset] = outR;
			outputData[targetOffset + 1] = outG;
			outputData[targetOffset + 2] = outB;
			outputData[targetOffset + 3] = outA;
		}
	}
}
`;

const GLOBAL_OPAQUE_BILINEAR_HELPERS = `${DIRECT_BILINEAR_HELPERS}
function resizeBilinearGlobalOpaque(source: ImageData, output: ImageData, sourceRect: Rect) {
	if (hasTransparentPixels(source.data)) {
		resizeBilinearDirect(source, output, sourceRect);
		return;
	}
	resizeOpaqueBilinearDirect(source, output, sourceRect);
}

function resizeOpaqueBilinearDirect(source: ImageData, output: ImageData, sourceRect: Rect) {
	const outputData = output.data;
	const data = source.data;
	const scaleX = sourceRect.width / output.width;
	const scaleY = sourceRect.height / output.height;
	for (let y = 0; y < output.height; y++) {
		const sourceY = sourceRect.y + (y + 0.5) * scaleY - 0.5;
		const y0 = Math.floor(sourceY);
		const y1 = y0 + 1;
		const clampedY0 = Math.min(source.height - 1, Math.max(0, y0));
		const clampedY1 = Math.min(source.height - 1, Math.max(0, y1));
		const ty = sourceY - y0;
		const weightY0 = 1 - ty;
		for (let x = 0; x < output.width; x++) {
			const sourceX = sourceRect.x + (x + 0.5) * scaleX - 0.5;
			const x0 = Math.floor(sourceX);
			const x1 = x0 + 1;
			const clampedX0 = Math.min(source.width - 1, Math.max(0, x0));
			const clampedX1 = Math.min(source.width - 1, Math.max(0, x1));
			const tx = sourceX - x0;
			const weight00 = (1 - tx) * weightY0;
			const weight10 = tx * weightY0;
			const weight01 = (1 - tx) * ty;
			const weight11 = tx * ty;
			const offset00 = (clampedY0 * source.width + clampedX0) * 4;
			const offset10 = (clampedY0 * source.width + clampedX1) * 4;
			const offset01 = (clampedY1 * source.width + clampedX0) * 4;
			const offset11 = (clampedY1 * source.width + clampedX1) * 4;
			const total = weight00 + weight10 + weight01 + weight11;
			const targetOffset = (y * output.width + x) * 4;
			outputData[targetOffset] = clampByte(
				(data[offset00]! * weight00 +
					data[offset10]! * weight10 +
					data[offset01]! * weight01 +
					data[offset11]! * weight11) /
					total
			);
			outputData[targetOffset + 1] = clampByte(
				(data[offset00 + 1]! * weight00 +
					data[offset10 + 1]! * weight10 +
					data[offset01 + 1]! * weight01 +
					data[offset11 + 1]! * weight11) /
					total
			);
			outputData[targetOffset + 2] = clampByte(
				(data[offset00 + 2]! * weight00 +
					data[offset10 + 2]! * weight10 +
					data[offset01 + 2]! * weight01 +
					data[offset11 + 2]! * weight11) /
					total
			);
			outputData[targetOffset + 3] = 255;
		}
	}
}
`;

const AXIS_TABLE_BILINEAR_HELPERS = `${GLOBAL_OPAQUE_BILINEAR_HELPERS}
type BilinearAxisTable = {
	first: Int32Array;
	second: Int32Array;
	firstWeight: Float64Array;
	secondWeight: Float64Array;
};

function resizeBilinearAxisTables(source: ImageData, output: ImageData, sourceRect: Rect) {
	if (hasTransparentPixels(source.data)) {
		resizeBilinearAxisTablesWithAlpha(source, output, sourceRect);
		return;
	}
	resizeOpaqueBilinearAxisTables(source, output, sourceRect);
}

function bilinearAxisTable(count: number, sourceStart: number, scale: number, sourceLimit: number) {
	const first = new Int32Array(count);
	const second = new Int32Array(count);
	const firstWeight = new Float64Array(count);
	const secondWeight = new Float64Array(count);
	for (let target = 0; target < count; target++) {
		const sourcePosition = sourceStart + (target + 0.5) * scale - 0.5;
		const source0 = Math.floor(sourcePosition);
		const source1 = source0 + 1;
		const t = sourcePosition - source0;
		first[target] = Math.min(sourceLimit - 1, Math.max(0, source0)) * 4;
		second[target] = Math.min(sourceLimit - 1, Math.max(0, source1)) * 4;
		firstWeight[target] = 1 - t;
		secondWeight[target] = t;
	}
	return { first, second, firstWeight, secondWeight } satisfies BilinearAxisTable;
}

function resizeOpaqueBilinearAxisTables(source: ImageData, output: ImageData, sourceRect: Rect) {
	const outputData = output.data;
	const data = source.data;
	const scaleX = sourceRect.width / output.width;
	const scaleY = sourceRect.height / output.height;
	const xTable = bilinearAxisTable(output.width, sourceRect.x, scaleX, source.width);
	const yTable = bilinearAxisTable(output.height, sourceRect.y, scaleY, source.height);
	for (let y = 0; y < output.height; y++) {
		const row0 = (yTable.first[y]! / 4) * source.width * 4;
		const row1 = (yTable.second[y]! / 4) * source.width * 4;
		const wy0 = yTable.firstWeight[y]!;
		const wy1 = yTable.secondWeight[y]!;
		for (let x = 0; x < output.width; x++) {
			const wx0 = xTable.firstWeight[x]!;
			const wx1 = xTable.secondWeight[x]!;
			const weight00 = wx0 * wy0;
			const weight10 = wx1 * wy0;
			const weight01 = wx0 * wy1;
			const weight11 = wx1 * wy1;
			const offset00 = row0 + xTable.first[x]!;
			const offset10 = row0 + xTable.second[x]!;
			const offset01 = row1 + xTable.first[x]!;
			const offset11 = row1 + xTable.second[x]!;
			const total = weight00 + weight10 + weight01 + weight11;
			const targetOffset = (y * output.width + x) * 4;
			outputData[targetOffset] = clampByte(
				(data[offset00]! * weight00 +
					data[offset10]! * weight10 +
					data[offset01]! * weight01 +
					data[offset11]! * weight11) /
					total
			);
			outputData[targetOffset + 1] = clampByte(
				(data[offset00 + 1]! * weight00 +
					data[offset10 + 1]! * weight10 +
					data[offset01 + 1]! * weight01 +
					data[offset11 + 1]! * weight11) /
					total
			);
			outputData[targetOffset + 2] = clampByte(
				(data[offset00 + 2]! * weight00 +
					data[offset10 + 2]! * weight10 +
					data[offset01 + 2]! * weight01 +
					data[offset11 + 2]! * weight11) /
					total
			);
			outputData[targetOffset + 3] = 255;
		}
	}
}

function resizeBilinearAxisTablesWithAlpha(source: ImageData, output: ImageData, sourceRect: Rect) {
	const outputData = output.data;
	const data = source.data;
	const scaleX = sourceRect.width / output.width;
	const scaleY = sourceRect.height / output.height;
	const xTable = bilinearAxisTable(output.width, sourceRect.x, scaleX, source.width);
	const yTable = bilinearAxisTable(output.height, sourceRect.y, scaleY, source.height);
	for (let y = 0; y < output.height; y++) {
		const row0 = (yTable.first[y]! / 4) * source.width * 4;
		const row1 = (yTable.second[y]! / 4) * source.width * 4;
		const wy0 = yTable.firstWeight[y]!;
		const wy1 = yTable.secondWeight[y]!;
		for (let x = 0; x < output.width; x++) {
			const wx0 = xTable.firstWeight[x]!;
			const wx1 = xTable.secondWeight[x]!;
			const weight00 = wx0 * wy0;
			const weight10 = wx1 * wy0;
			const weight01 = wx0 * wy1;
			const weight11 = wx1 * wy1;
			const offset00 = row0 + xTable.first[x]!;
			const offset10 = row0 + xTable.second[x]!;
			const offset01 = row1 + xTable.first[x]!;
			const offset11 = row1 + xTable.second[x]!;
			const total = weight00 + weight10 + weight01 + weight11;
			const targetOffset = (y * output.width + x) * 4;

			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;
			let alpha = data[offset00 + 3]! / 255;
			r += data[offset00]! * alpha * weight00;
			g += data[offset00 + 1]! * alpha * weight00;
			b += data[offset00 + 2]! * alpha * weight00;
			a += data[offset00 + 3]! * weight00;

			alpha = data[offset10 + 3]! / 255;
			r += data[offset10]! * alpha * weight10;
			g += data[offset10 + 1]! * alpha * weight10;
			b += data[offset10 + 2]! * alpha * weight10;
			a += data[offset10 + 3]! * weight10;

			alpha = data[offset01 + 3]! / 255;
			r += data[offset01]! * alpha * weight01;
			g += data[offset01 + 1]! * alpha * weight01;
			b += data[offset01 + 2]! * alpha * weight01;
			a += data[offset01 + 3]! * weight01;

			alpha = data[offset11 + 3]! / 255;
			r += data[offset11]! * alpha * weight11;
			g += data[offset11 + 1]! * alpha * weight11;
			b += data[offset11 + 2]! * alpha * weight11;
			a += data[offset11 + 3]! * weight11;

			if (total === 0) {
				outputData[targetOffset] = 0;
				outputData[targetOffset + 1] = 0;
				outputData[targetOffset + 2] = 0;
				outputData[targetOffset + 3] = 0;
				continue;
			}
			const [outR, outG, outB, outA] = unpremultiplySample(
				r / total,
				g / total,
				b / total,
				a / total
			);
			outputData[targetOffset] = outR;
			outputData[targetOffset + 1] = outG;
			outputData[targetOffset + 2] = outB;
			outputData[targetOffset + 3] = outA;
		}
	}
}
`;

main();
