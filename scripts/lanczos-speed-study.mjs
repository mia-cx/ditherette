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
		description: 'Current exact Lanczos3 implementation',
		apply: (source) => source
	},
	{
		id: 'x-byte-offsets',
		description: 'Precompute source byte offsets for horizontal Lanczos taps',
		apply: applyXByteOffsets
	},
	{
		id: 'vertical-row-offset-hoist',
		description: 'Hoist vertical pass row offsets out of the tap loop',
		apply: applyVerticalRowOffsetHoist
	},
	{
		id: 'vertical-six-tap-unroll',
		description: 'Unroll the vertical Lanczos tap accumulation',
		apply: applyVerticalTapUnroll
	},
	{
		id: 'x-byte-offsets-vertical-six-tap-unroll',
		description: 'Combine horizontal byte offsets with vertical tap unrolling',
		apply: (source) => applyVerticalTapUnroll(applyXByteOffsets(source))
	}
];

const args = parseArgs(process.argv.slice(2));
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const outDir = path.resolve(root, args.out ?? `benchmark-results/lanczos-speed-study-${timestamp}`);
const originalWorkingTree = readFileSync(path.join(root, RESIZE_PATH), 'utf8');
const baseline = gitShow(`HEAD:${RESIZE_PATH}`);
let browser;

try {
	await mkdir(outDir, { recursive: true });
	const sources = await benchmarkSources(root, args);
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

function parseArgs(argv) {
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
	console.log(`Usage: pnpm bench:lanczos-speed-study -- [options]

Benchmarks exact-output current Lanczos3 implementation changes across the Lanczos-relevant case matrix.
This does not test other resize modes and does not vary filter semantics.

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
  pnpm bench:lanczos-speed-study
  pnpm bench:lanczos-speed-study -- --iterations 10 --warmups 3 --check-correctness --quiet true
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
	if (!sources.length) throw new Error('No Lanczos speed-study sources available.');
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
	const cases = [];
	for (const source of sources) {
		for (const scale of options.scales) {
			const dimensions = outputDimensions(source.imageData, scale);
			cases.push({
				id: `${source.id}-scale-${scaleLabel(scale)}`,
				source,
				scale,
				width: dimensions.width,
				height: dimensions.height
			});
		}
	}
	return cases;
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
	return resizeImageData(testCase.source.imageData, testCase.width, testCase.height, 'lanczos3');
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
		path.join(variantDir, 'lanczos-speed.json'),
		`${JSON.stringify(result, null, 2)}\n`
	);
	await writeFile(path.join(variantDir, 'lanczos-speed.csv'), variantCsv(result));
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

function applyXByteOffsets(source) {
	return source
		.replace(
			'type ContributionTable = {\n\tindices: Int32Array;\n\tweights: Float64Array;\n\ttotals: Float64Array;\n};',
			'type ContributionTable = {\n\tindices: Int32Array;\n\tbyteOffsets: Int32Array;\n\tweights: Float64Array;\n\ttotals: Float64Array;\n};'
		)
		.replace(
			'const weights = new Float64Array(count * LANCZOS_TAPS);\n\tconst totals = new Float64Array(count);',
			'const byteOffsets = new Int32Array(count * LANCZOS_TAPS);\n\tconst weights = new Float64Array(count * LANCZOS_TAPS);\n\tconst totals = new Float64Array(count);'
		)
		.replace(
			'indices[base + tap] = Math.min(sourceLimit - 1, Math.max(0, sourceIndex));\n\t\t\tweights[base + tap] = weight;',
			'const clampedSourceIndex = Math.min(sourceLimit - 1, Math.max(0, sourceIndex));\n\t\t\tindices[base + tap] = clampedSourceIndex;\n\t\t\tbyteOffsets[base + tap] = clampedSourceIndex * 4;\n\t\t\tweights[base + tap] = weight;'
		)
		.replace(
			'return { indices, weights, totals } satisfies ContributionTable;',
			'return { indices, byteOffsets, weights, totals } satisfies ContributionTable;'
		)
		.replaceAll(
			'sourceRowOffset + xTable.indices[xBase]! * 4',
			'sourceRowOffset + xTable.byteOffsets[xBase]!'
		)
		.replaceAll(
			'sourceRowOffset + xTable.indices[xBase + 1]! * 4',
			'sourceRowOffset + xTable.byteOffsets[xBase + 1]!'
		)
		.replaceAll(
			'sourceRowOffset + xTable.indices[xBase + 2]! * 4',
			'sourceRowOffset + xTable.byteOffsets[xBase + 2]!'
		)
		.replaceAll(
			'sourceRowOffset + xTable.indices[xBase + 3]! * 4',
			'sourceRowOffset + xTable.byteOffsets[xBase + 3]!'
		)
		.replaceAll(
			'sourceRowOffset + xTable.indices[xBase + 4]! * 4',
			'sourceRowOffset + xTable.byteOffsets[xBase + 4]!'
		)
		.replaceAll(
			'sourceRowOffset + xTable.indices[xBase + 5]! * 4',
			'sourceRowOffset + xTable.byteOffsets[xBase + 5]!'
		);
}

function applyVerticalRowOffsetHoist(source) {
	return source
		.replace(
			`let r = 0;
			let g = 0;
			let b = 0;
			for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
				const weight = yTable.weights[yBase + yTap]!;
				const row = rows[yTap];
				if (weight === 0 || !row) continue;
				const rowOffset = targetX * 3;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
			}`,
			`const rowOffset = targetX * 3;
			let r = 0;
			let g = 0;
			let b = 0;
			for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
				const weight = yTable.weights[yBase + yTap]!;
				const row = rows[yTap];
				if (weight === 0 || !row) continue;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
			}`
		)
		.replace(
			`let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;
			for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
				const weight = yTable.weights[yBase + yTap]!;
				const row = rows[yTap];
				if (weight === 0 || !row) continue;
				const rowOffset = targetX * 4;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
				a += row[rowOffset + 3]! * weight;
			}`,
			`const rowOffset = targetX * 4;
			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;
			for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
				const weight = yTable.weights[yBase + yTap]!;
				const row = rows[yTap];
				if (weight === 0 || !row) continue;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
				a += row[rowOffset + 3]! * weight;
			}`
		);
}

function applyVerticalTapUnroll(source) {
	return source
		.replace(
			`let r = 0;
			let g = 0;
			let b = 0;
			for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
				const weight = yTable.weights[yBase + yTap]!;
				const row = rows[yTap];
				if (weight === 0 || !row) continue;
				const rowOffset = targetX * 3;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
			}`,
			`const rowOffset = targetX * 3;
			let r = 0;
			let g = 0;
			let b = 0;

			const row0 = rows[0];
			const weight0 = yTable.weights[yBase]!;
			if (weight0 !== 0 && row0) {
				r += row0[rowOffset]! * weight0;
				g += row0[rowOffset + 1]! * weight0;
				b += row0[rowOffset + 2]! * weight0;
			}

			const row1 = rows[1];
			const weight1 = yTable.weights[yBase + 1]!;
			if (weight1 !== 0 && row1) {
				r += row1[rowOffset]! * weight1;
				g += row1[rowOffset + 1]! * weight1;
				b += row1[rowOffset + 2]! * weight1;
			}

			const row2 = rows[2];
			const weight2 = yTable.weights[yBase + 2]!;
			if (weight2 !== 0 && row2) {
				r += row2[rowOffset]! * weight2;
				g += row2[rowOffset + 1]! * weight2;
				b += row2[rowOffset + 2]! * weight2;
			}

			const row3 = rows[3];
			const weight3 = yTable.weights[yBase + 3]!;
			if (weight3 !== 0 && row3) {
				r += row3[rowOffset]! * weight3;
				g += row3[rowOffset + 1]! * weight3;
				b += row3[rowOffset + 2]! * weight3;
			}

			const row4 = rows[4];
			const weight4 = yTable.weights[yBase + 4]!;
			if (weight4 !== 0 && row4) {
				r += row4[rowOffset]! * weight4;
				g += row4[rowOffset + 1]! * weight4;
				b += row4[rowOffset + 2]! * weight4;
			}

			const row5 = rows[5];
			const weight5 = yTable.weights[yBase + 5]!;
			if (weight5 !== 0 && row5) {
				r += row5[rowOffset]! * weight5;
				g += row5[rowOffset + 1]! * weight5;
				b += row5[rowOffset + 2]! * weight5;
			}`
		)
		.replace(
			`let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;
			for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
				const weight = yTable.weights[yBase + yTap]!;
				const row = rows[yTap];
				if (weight === 0 || !row) continue;
				const rowOffset = targetX * 4;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
				a += row[rowOffset + 3]! * weight;
			}`,
			`const rowOffset = targetX * 4;
			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;

			const row0 = rows[0];
			const weight0 = yTable.weights[yBase]!;
			if (weight0 !== 0 && row0) {
				r += row0[rowOffset]! * weight0;
				g += row0[rowOffset + 1]! * weight0;
				b += row0[rowOffset + 2]! * weight0;
				a += row0[rowOffset + 3]! * weight0;
			}

			const row1 = rows[1];
			const weight1 = yTable.weights[yBase + 1]!;
			if (weight1 !== 0 && row1) {
				r += row1[rowOffset]! * weight1;
				g += row1[rowOffset + 1]! * weight1;
				b += row1[rowOffset + 2]! * weight1;
				a += row1[rowOffset + 3]! * weight1;
			}

			const row2 = rows[2];
			const weight2 = yTable.weights[yBase + 2]!;
			if (weight2 !== 0 && row2) {
				r += row2[rowOffset]! * weight2;
				g += row2[rowOffset + 1]! * weight2;
				b += row2[rowOffset + 2]! * weight2;
				a += row2[rowOffset + 3]! * weight2;
			}

			const row3 = rows[3];
			const weight3 = yTable.weights[yBase + 3]!;
			if (weight3 !== 0 && row3) {
				r += row3[rowOffset]! * weight3;
				g += row3[rowOffset + 1]! * weight3;
				b += row3[rowOffset + 2]! * weight3;
				a += row3[rowOffset + 3]! * weight3;
			}

			const row4 = rows[4];
			const weight4 = yTable.weights[yBase + 4]!;
			if (weight4 !== 0 && row4) {
				r += row4[rowOffset]! * weight4;
				g += row4[rowOffset + 1]! * weight4;
				b += row4[rowOffset + 2]! * weight4;
				a += row4[rowOffset + 3]! * weight4;
			}

			const row5 = rows[5];
			const weight5 = yTable.weights[yBase + 5]!;
			if (weight5 !== 0 && row5) {
				r += row5[rowOffset]! * weight5;
				g += row5[rowOffset + 1]! * weight5;
				b += row5[rowOffset + 2]! * weight5;
				a += row5[rowOffset + 3]! * weight5;
			}`
		);
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
