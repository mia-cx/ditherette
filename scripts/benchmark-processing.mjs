#!/usr/bin/env node
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

installImageDataPolyfill();

const args = parseArgs(process.argv.slice(2));
const scriptPath = fileURLToPath(import.meta.url);
const root = path.resolve(path.dirname(scriptPath), '..');

if (process.env.DITHERETTE_BENCH_CHILD !== '1') {
	process.exit(await runBenchmarkChild(scriptPath, root));
}

const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const outDir = path.resolve(root, args.out ?? `benchmark-results/processing-${timestamp}`);

const server = await createServer({
	root,
	configFile: path.join(root, 'vite.config.ts'),
	appType: 'custom',
	logLevel: 'silent',
	server: { middlewareMode: true, hmr: false }
});

let browser;

try {
	const benchmark = await server.ssrLoadModule('/src/lib/benchmark/processing-benchmark.ts');
	const imagePaths = await benchmarkImagePaths(root, args);
	let sources;
	if (imagePaths.length) {
		browser = await chromium.launch({ headless: true });
		sources = [];
		for (const imagePath of imagePaths) {
			console.log(`Decoding ${path.relative(root, imagePath)}...`);
			sources.push(await decodeBenchmarkImage(browser, root, imagePath));
		}
	}
	const matrixDimensions = [...new Set(args.matrixDimensions)];
	const result = benchmark.runProcessingBenchmarks({
		profile: args.profile,
		iterations: args.iterations,
		warmups: args.warmups,
		caseIds: args.cases,
		includePng: args.includePng,
		sources,
		matrixDimensions,
		stopAfterStage: args.stopAfterStage ?? stopStageForDimensions(matrixDimensions),
		onProgress: args.quiet ? undefined : logProgress
	});
	const artifact = {
		...result,
		environment: {
			node: process.version,
			platform: `${process.platform} ${process.arch}`,
			cpus: os.cpus().map((cpu) => cpu.model),
			totalMemoryBytes: os.totalmem()
		}
	};
	await mkdir(outDir, { recursive: true });
	const jsonPath = path.join(outDir, 'processing-benchmark.json');
	const csvPath = path.join(outDir, 'processing-benchmark.csv');
	await writeFile(jsonPath, `${JSON.stringify(artifact, null, 2)}\n`);
	await writeFile(csvPath, benchmark.benchmarkResultsToCsv(result));
	console.log(benchmark.formatBenchmarkTable(result));
	console.log(`\nWrote ${path.relative(root, jsonPath)}`);
	console.log(`Wrote ${path.relative(root, csvPath)}`);
} finally {
	await closeResources();
}

async function runBenchmarkChild(scriptPath, root) {
	const child = spawn(process.execPath, [scriptPath, ...process.argv.slice(2)], {
		cwd: root,
		detached: process.platform !== 'win32',
		env: { ...process.env, DITHERETTE_BENCH_CHILD: '1' },
		stdio: 'inherit'
	});
	let childExited = false;
	let stopping = false;
	let forceKillTimer;

	const killChild = (signal) => {
		if (childExited) return;
		killProcessTree(child.pid, signal);
	};

	const stop = (signal) => {
		if (stopping) {
			console.error(`\nReceived ${signal} again; killing benchmark process group now.`);
			killChild('SIGKILL');
			return;
		}
		stopping = true;
		console.error(`\nReceived ${signal}; killing benchmark process tree...`);
		killChild('SIGKILL');
		forceKillTimer = setTimeout(() => process.exit(signal === 'SIGINT' ? 130 : 143), 50);
	};

	process.once('SIGINT', () => stop('SIGINT'));
	process.once('SIGTERM', () => stop('SIGTERM'));
	process.once('exit', () => {
		if (!childExited) killChild('SIGKILL');
	});

	return await new Promise((resolve) => {
		child.once('exit', (code, signal) => {
			childExited = true;
			if (forceKillTimer) clearTimeout(forceKillTimer);
			if (code !== null) {
				resolve(code);
				return;
			}
			resolve(signal === 'SIGINT' ? 130 : signal === 'SIGTERM' ? 143 : 1);
		});
		child.once('error', (error) => {
			childExited = true;
			if (forceKillTimer) clearTimeout(forceKillTimer);
			console.error(error instanceof Error ? error.message : error);
			resolve(1);
		});
	});
}

function killProcessTree(rootPid, signal) {
	if (!rootPid) return;
	const descendants = processTree(rootPid);
	for (const processInfo of descendants) killPid(processInfo.pid, signal);
	const processGroups = new Set(descendants.map((processInfo) => processInfo.pgid).filter(Boolean));
	for (const pgid of processGroups) killPid(-pgid, signal);
	killPid(rootPid, signal);
}

function killPid(pid, signal) {
	try {
		process.kill(pid, signal);
	} catch (error) {
		if (error?.code !== 'ESRCH' && error?.code !== 'EPERM') {
			console.error(error instanceof Error ? error.message : error);
		}
	}
}

function processTree(rootPid) {
	if (process.platform === 'win32') return [{ pid: rootPid, pgid: rootPid }];
	let rows;
	try {
		rows = execFileSync('ps', ['-axo', 'pid=,ppid=,pgid='], { encoding: 'utf8' });
	} catch {
		return [{ pid: rootPid, pgid: rootPid }];
	}
	const childrenByParent = new Map();
	const processes = new Map();
	for (const row of rows.trim().split('\n')) {
		const [pidText, ppidText, pgidText] = row.trim().split(/\s+/);
		const pid = Number(pidText);
		const ppid = Number(ppidText);
		const pgid = Number(pgidText);
		if (!pid || !ppid || !pgid) continue;
		processes.set(pid, { pid, ppid, pgid });
		const children = childrenByParent.get(ppid) ?? [];
		children.push(pid);
		childrenByParent.set(ppid, children);
	}
	const result = [];
	const queue = [rootPid];
	const seen = new Set();
	while (queue.length) {
		const pid = queue.shift();
		if (!pid || seen.has(pid)) continue;
		seen.add(pid);
		const processInfo = processes.get(pid) ?? { pid, ppid: 0, pgid: pid };
		result.push(processInfo);
		queue.push(...(childrenByParent.get(pid) ?? []));
	}
	return result.sort((left, right) => right.pid - left.pid);
}

async function closeResources() {
	await browser?.close();
	browser = undefined;
	await server.close();
}

async function benchmarkImagePaths(root, args) {
	const explicit = args.images.map((imagePath) => path.resolve(root, imagePath));
	if (args.synthetic || (explicit.length && !args.fixtureDirs.length)) return explicit;
	const fixtureDirs = args.fixtureDirs.length
		? args.fixtureDirs
		: [
				(await directoryExists(path.join(root, 'benchmark-fixtures')))
					? 'benchmark-fixtures'
					: undefined
			];
	const fixtureImages = (
		await Promise.all(
			fixtureDirs
				.filter(Boolean)
				.map((fixtureDir) => imageFilesInDirectory(path.resolve(root, fixtureDir)))
		)
	).flat();
	return [...new Set([...explicit, ...fixtureImages])];
}

async function directoryExists(directory) {
	try {
		const entries = await readdir(directory);
		return entries.length >= 0;
	} catch {
		return false;
	}
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

async function decodeBenchmarkImage(browser, root, imagePath) {
	const start = performance.now();
	const bytes = await readFile(imagePath);
	const dataUrl = `data:${mimeForPath(imagePath)};base64,${bytes.toString('base64')}`;
	const page = await browser.newPage();
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
			return {
				width: imageData.width,
				height: imageData.height,
				data: imageData.data
			};
		}, dataUrl);
		return {
			id: path.basename(imagePath, path.extname(imagePath)),
			label: path.relative(root, imagePath),
			kind: 'image',
			path: path.relative(root, imagePath),
			decodeMs: performance.now() - start,
			imageData: new ImageData(new Uint8ClampedArray(decoded.data), decoded.width, decoded.height)
		};
	} finally {
		await page.close();
	}
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

function logProgress(event) {
	switch (event.type) {
		case 'case-start':
			console.log(
				`\n[${event.caseIndex}/${event.totalCases}] ${event.sourceId} › ${event.caseId} (${formatPixels(event.outputPixels)})`
			);
			break;
		case 'iteration-end':
			console.log(formatIterationProgress(event));
			break;
		case 'case-end':
			console.log(formatCaseSummary(event));
			break;
	}
}

function formatIterationProgress(event) {
	const convertMs = quantizeConvertMs(event.quantizeTimings);
	const loopMs = quantizeLoopMs(event.quantizeTimings);
	const pieces = [
		`  ${iterationLabel(event)}`,
		`total ${formatMs(event.durationMs)}`,
		`${event.resizeCacheHit ? 'resize hit' : 'resize'} ${formatMs(event.stages.resize)}`,
		`quantize ${formatMs(loopMs)}`
	];
	if (convertMs > 0 || loopMs > 0) {
		pieces.push(`q convert ${formatMs(convertMs)}`);
		pieces.push(`q loop ${formatMs(loopMs)}`);
	}
	const matchCount = quantizeMatchCount(event.quantizeCounts);
	const memoHits = quantizeMemoHits(event.quantizeCounts);
	if (matchCount > 0 || memoHits > 0) {
		pieces.push(`matches ${formatCount(matchCount)}`);
		pieces.push(`memo hits ${formatCount(memoHits)}`);
	}
	return pieces.join(' · ');
}

function formatCaseSummary(event) {
	const result = event.result;
	const resizeHits = result.runs.filter((run) => run.resizeCacheHit).length;
	const details = [
		`  mean total ${formatMs(result.stages.total.meanMs)}`,
		`resize ${formatMs(result.stages.resize.meanMs)} (${resizeHits}/${result.runs.length} hits)`,
		`quantize ${formatMs(quantizeLoopMean(result))}`,
		`q convert ${formatMs(quantizeConvertMean(result))}`,
		`q image ${formatMs(quantizeMean(result, 'color space convert composited image') + quantizeMean(result, 'color space convert source image'))}`,
		`q palette ${formatMs(quantizeMean(result, 'color space convert palette'))}`,
		`q loop ${formatMs(quantizeLoopMean(result))}`
	];
	const matchCount =
		quantizeCounterMean(result, 'nearest rgb') + quantizeCounterMean(result, 'nearest vector');
	const memoHits =
		quantizeCounterMean(result, 'rgb memo hit') + quantizeCounterMean(result, 'vector memo hit');
	if (matchCount > 0 || memoHits > 0) {
		details.push(`matches/run ${formatCount(matchCount)}`);
		details.push(`memo hits/run ${formatCount(memoHits)}`);
	}
	return `${details.join(' · ')}\n  hotspots ${result.hotspot} / ${result.quantizeHotspot ?? '—'} · case wall ${formatMs(event.durationMs)}`;
}

function iterationLabel(event) {
	return event.warmup ? `warmup ${event.iteration}` : `run ${event.iteration}`;
}

function quantizeConvertMs(timings) {
	return (
		(timings['color space convert palette'] ?? 0) +
		(timings['color space convert composited image'] ?? 0) +
		(timings['color space convert source image'] ?? 0)
	);
}

function quantizeLoopMs(timings) {
	return (
		(timings['quantize direct dither+match loop'] ?? 0) +
		(timings['quantize vector diffusion dither+match loop'] ?? 0) +
		(timings['quantize rgb diffusion dither+match loop'] ?? 0)
	);
}

function quantizeMatchCount(counts) {
	return (counts['nearest rgb'] ?? 0) + (counts['nearest vector'] ?? 0);
}

function quantizeMemoHits(counts) {
	return (counts['rgb memo hit'] ?? 0) + (counts['vector memo hit'] ?? 0);
}

function quantizeConvertMean(result) {
	return (
		quantizeMean(result, 'color space convert palette') +
		quantizeMean(result, 'color space convert composited image') +
		quantizeMean(result, 'color space convert source image')
	);
}

function quantizeLoopMean(result) {
	return (
		quantizeMean(result, 'quantize direct dither+match loop') +
		quantizeMean(result, 'quantize vector diffusion dither+match loop') +
		quantizeMean(result, 'quantize rgb diffusion dither+match loop')
	);
}

function quantizeMean(result, name) {
	return result.quantizeSubstages[name]?.meanMs ?? 0;
}

function quantizeCounterMean(result, name) {
	return result.quantizeCounters[name]?.meanMs ?? 0;
}

function formatMs(value) {
	return `${value.toFixed(value >= 100 ? 0 : 1)}ms`;
}

function formatCount(value) {
	if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
	if (value >= 1_000) return `${Math.round(value / 1_000)}K`;
	return String(Math.round(value));
}

function formatPixels(value) {
	if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}MP`;
	if (value >= 1_000) return `${Math.round(value / 1_000)}K`;
	return String(value);
}

function stopStageForDimensions(dimensions) {
	if (dimensions.includes('dither')) return 'quantize';
	if (dimensions.includes('colorSpace')) return 'colorSpaceConvert';
	if (dimensions.includes('scale') || dimensions.includes('resize')) return 'resize';
	return undefined;
}

function parseArgs(argv) {
	const options = { images: [], fixtureDirs: [], matrixDimensions: [] };
	for (let index = 0; index < argv.length; index++) {
		const arg = argv[index];
		if (arg === '--') continue;
		if (!arg.startsWith('--')) throw new Error(`Unexpected positional argument: ${arg}`);
		const [key, inlineValue] = arg.slice(2).split('=', 2);
		const nextValue = () => inlineValue ?? argv[++index];
		const optionalBoolean = () => {
			if (inlineValue !== undefined) return parseBoolean(inlineValue);
			if (argv[index + 1] && !argv[index + 1].startsWith('--')) return parseBoolean(argv[++index]);
			return true;
		};
		switch (key) {
			case 'profile':
				options.profile = nextValue();
				break;
			case 'iterations':
				options.iterations = Number.parseInt(nextValue(), 10);
				break;
			case 'warmups':
				options.warmups = Number.parseInt(nextValue(), 10);
				break;
			case 'cases':
				options.cases = nextValue()
					.split(',')
					.map((item) => item.trim())
					.filter(Boolean);
				break;
			case 'image':
				options.images.push(nextValue());
				break;
			case 'fixtures-dir':
				options.fixtureDirs.push(nextValue());
				break;
			case 'scale':
				if (optionalBoolean()) options.matrixDimensions.push('scale');
				break;
			case 'resize':
				if (optionalBoolean()) options.matrixDimensions.push('resize');
				break;
			case 'dither':
				if (optionalBoolean()) options.matrixDimensions.push('dither');
				break;
			case 'color-space':
			case 'colorspace':
				if (optionalBoolean()) options.matrixDimensions.push('colorSpace');
				break;
			case 'preview':
				if (optionalBoolean()) options.stopAfterStage = 'previewRender';
				break;
			case 'png':
				if (optionalBoolean()) options.stopAfterStage = 'pngEncode';
				break;
			case 'synthetic':
				options.synthetic = optionalBoolean();
				break;
			case 'include-png':
				options.includePng = optionalBoolean();
				break;
			case 'quiet':
				options.quiet = optionalBoolean();
				break;
			case 'out':
				options.out = nextValue();
				break;
			case 'help':
				printHelp();
				process.exit(0);
				break;
			default:
				throw new Error(`Unknown option: --${key}`);
		}
	}
	return options;
}

function parseBoolean(value) {
	if (value === 'true') return true;
	if (value === 'false') return false;
	throw new Error(`Expected true or false, got ${value}`);
}

function printHelp() {
	console.log(
		`Usage: pnpm bench:processing -- [options]\n\nOptions:\n  --profile smoke|baseline|large|exhaustive\n                                  Fixture profile to run (default: smoke). exhaustive runs\n                                  0.125×/0.25×/0.5×/0.75× × every resize mode × every color space × every dither mode.\n  --iterations N                  Recorded iterations per case (default: 3)\n  --warmups N                     Warmup iterations per case (default: 1)\n  --cases id,id                   Comma-separated case IDs\n  --scale                         Vary 0.125×, 0.25×, 0.5×, 0.75×; stops after resize unless later flags are set\n  --resize                        Vary nearest, bilinear, lanczos3, area; stops after resize unless later flags are set\n  --dither                        Vary every dither mode; stops after quantize\n  --color-space                   Vary every color space; stops after color conversion unless --dither is set\n  --preview                       Stop after preview render\n  --png                           Stop after PNG encode\n  --image FILE                    Decode and benchmark a real image; repeatable\n  --fixtures-dir DIR              Decode all images in a fixture directory\n                                  Defaults to benchmark-fixtures/ when it exists\n  --synthetic true|false          Ignore benchmark-fixtures/ and use synthetic sources unless --image is set\n  --include-png true|false        Override per-case PNG encode stage\n  --quiet true|false              Disable per-case/stage progress logs\n  --out DIR                       Artifact directory (default: benchmark-results/processing-<timestamp>)\n\nExamples:\n  pnpm bench:processing -- --image ~/Pictures/photo.jpg --profile exhaustive\n  pnpm bench:processing -- --scale --resize\n  pnpm bench:processing -- --dither --color-space\n  mkdir -p benchmark-fixtures && cp ~/Pictures/*.png benchmark-fixtures/\n  pnpm bench:processing -- --fixtures-dir benchmark-fixtures --profile smoke\n`
	);
}

function installImageDataPolyfill() {
	if ('ImageData' in globalThis) return;
	globalThis.ImageData = class ImageData {
		constructor(dataOrWidth, width, height) {
			if (typeof dataOrWidth === 'number') {
				this.width = dataOrWidth;
				this.height = width;
				this.data = new Uint8ClampedArray(this.width * this.height * 4);
				return;
			}
			this.data = dataOrWidth;
			this.width = width;
			this.height = height;
		}
	};
}
