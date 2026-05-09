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
const DEFAULT_VARIANTS = [
	'baseline',
	'scale-aware-lanczos3',
	'lanczos2',
	'hamming',
	'mks2013',
	'box'
];
const DEFAULT_SCALES = [0.125, 0.25, 0.5, 0.75];
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = parseArgs(process.argv.slice(2));
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const outDir = path.resolve(
	root,
	args.out ?? `benchmark-results/lanczos-quality-study-${timestamp}`
);
const originalWorkingTree = readFileSync(path.join(root, RESIZE_PATH), 'utf8');

let browser;
let page;

try {
	const imagePaths = await qualityImagePaths(root, args);
	if (!imagePaths.length) throw new Error('No quality-study images found.');
	browser = await chromium.launch({ headless: true });
	page = await browser.newPage();
	const sources = [];
	for (const imagePath of imagePaths.slice(0, args.maxFixtures)) {
		console.log(`Decoding ${path.relative(root, imagePath)}...`);
		sources.push(await decodeImage(page, root, imagePath));
	}

	await mkdir(outDir, { recursive: true });
	const baselineOutputs = new Map();
	const summaries = [];

	for (const variant of args.variants) {
		console.log(`\n=== ${variant} quality snapshots ===`);
		writeFileSync(path.join(root, RESIZE_PATH), sourceForVariant(variant));
		const server = await createServer({
			root,
			configFile: path.join(root, 'vite.config.ts'),
			appType: 'custom',
			logLevel: 'silent',
			server: { middlewareMode: true, hmr: false }
		});
		try {
			const resizeModule = await server.ssrLoadModule(
				`/src/lib/processing/resize.ts?variant=${encodeURIComponent(variant)}`
			);
			for (const source of sources) {
				for (const scale of args.scales) {
					const output = resizeModule.resizeImageData(
						source.imageData,
						Math.max(1, Math.round(source.imageData.width * scale)),
						Math.max(1, Math.round(source.imageData.height * scale)),
						'stretch',
						'lanczos3'
					);
					const key = `${source.id}/${scaleLabel(scale)}`;
					if (variant === 'baseline') baselineOutputs.set(key, output);
					const png = await encodePng(page, output);
					const snapshotPath = path.join(
						outDir,
						'snapshots',
						source.id,
						scaleLabel(scale),
						`${variant}.png`
					);
					await mkdir(path.dirname(snapshotPath), { recursive: true });
					await writeFile(snapshotPath, png);

					let diff;
					if (variant !== 'baseline') {
						const baseline = baselineOutputs.get(key);
						if (!baseline) throw new Error(`Missing baseline output for ${key}`);
						diff = diffImages(baseline, output, png.length);
						const diffPath = path.join(
							outDir,
							'diffs',
							source.id,
							scaleLabel(scale),
							`${variant}-vs-baseline.json`
						);
						await mkdir(path.dirname(diffPath), { recursive: true });
						await writeFile(diffPath, `${JSON.stringify(diff, null, 2)}\n`);
					}

					summaries.push({
						variant,
						source: source.id,
						scale,
						width: output.width,
						height: output.height,
						pngBytes: png.length,
						diff
					});
					console.log(
						`${source.id} ${scale}× ${variant} ${output.width}×${output.height}${
							diff
								? ` changed=${diff.changedPixelPercent.toFixed(2)}% mae=${diff.meanAbsoluteChannelError.toFixed(2)}`
								: ''
						}`
					);
				}
			}
		} finally {
			await server.close();
		}
	}

	const artifact = {
		version: 1,
		runAt: new Date().toISOString(),
		variants: args.variants,
		scales: args.scales,
		sources: sources.map(({ imageData, ...source }) => ({
			...source,
			width: imageData.width,
			height: imageData.height
		})),
		environment: {
			node: process.version,
			platform: `${process.platform} ${process.arch}`,
			cpus: os.cpus().map((cpu) => cpu.model),
			totalMemoryBytes: os.totalmem()
		},
		results: summaries
	};
	await writeFile(
		path.join(outDir, 'quality-summary.json'),
		`${JSON.stringify(artifact, null, 2)}\n`
	);
	await writeFile(path.join(outDir, 'quality-summary.csv'), qualityCsv(summaries));
	console.log(`\nWrote ${path.relative(root, path.join(outDir, 'quality-summary.json'))}`);
	console.log(`Wrote ${path.relative(root, path.join(outDir, 'quality-summary.csv'))}`);
} finally {
	writeFileSync(path.join(root, RESIZE_PATH), originalWorkingTree);
	await page?.close();
	await browser?.close();
}

function parseArgs(argv) {
	const options = {
		variants: DEFAULT_VARIANTS,
		scales: DEFAULT_SCALES,
		images: [],
		fixtureDirs: [],
		maxFixtures: 4
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
			case 'variant':
			case 'variants':
				options.variants = value()
					.split(',')
					.map((item) => item.trim())
					.filter(Boolean);
				break;
			case 'scale':
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
				options.maxFixtures = Math.max(1, Number(value()));
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

function printHelp() {
	console.log(`Usage: pnpm bench:lanczos-quality-study -- [options]

Writes PNG snapshots and JSON/CSV diff metrics for output-changing Lanczos/filter variants.

Options:
  --variants a,b          Variant IDs (default: ${DEFAULT_VARIANTS.join(',')})
  --scales a,b            Scales (default: ${DEFAULT_SCALES.join(',')})
  --image FILE            Include an image file
  --fixtures-dir DIR      Read fixtures from a directory (default: benchmark-fixtures/)
  --max-fixtures N        Limit fixture count (default: 4)
  --out DIR               Artifact directory

Examples:
  pnpm bench:lanczos-quality-study
  pnpm bench:lanczos-quality-study -- --variants baseline,hamming,box --scales 0.25,0.5,0.75
`);
}

function sourceForVariant(variant) {
	const result = spawnSync(
		process.execPath,
		['scripts/lanczos-deferred-study.mjs', '--emit-source', variant],
		{ cwd: root, encoding: 'utf8' }
	);
	if (result.status !== 0) throw new Error(result.stderr || `Unable to emit source for ${variant}`);
	return result.stdout;
}

async function qualityImagePaths(rootDir, options) {
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

async function decodeImage(page, rootDir, imagePath) {
	const bytes = await readFile(imagePath);
	const dataUrl = `data:${mimeForPath(imagePath)};base64,${bytes.toString('base64')}`;
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
		return { width: imageData.width, height: imageData.height, data: [...imageData.data] };
	}, dataUrl);
	return {
		id: safePathSegment(path.basename(imagePath, path.extname(imagePath))),
		label: path.relative(rootDir, imagePath),
		path: path.relative(rootDir, imagePath),
		imageData: new ImageData(new Uint8ClampedArray(decoded.data), decoded.width, decoded.height)
	};
}

async function encodePng(page, imageData) {
	const bytes = await page.evaluate(
		async ({ width, height, data }) => {
			const canvas = document.createElement('canvas');
			canvas.width = width;
			canvas.height = height;
			const context = canvas.getContext('2d');
			if (!context) throw new Error('Unable to create a 2D canvas context.');
			context.putImageData(new ImageData(new Uint8ClampedArray(data), width, height), 0, 0);
			const blob = await new Promise((resolve) => canvas.toBlob(resolve, 'image/png'));
			if (!blob) throw new Error('Unable to encode PNG.');
			return [...new Uint8Array(await blob.arrayBuffer())];
		},
		{ width: imageData.width, height: imageData.height, data: [...imageData.data] }
	);
	return Buffer.from(bytes);
}

function diffImages(baseline, candidate, pngBytes) {
	if (baseline.width !== candidate.width || baseline.height !== candidate.height) {
		throw new Error('Cannot diff images with different dimensions.');
	}
	let changedPixels = 0;
	let changedChannels = 0;
	let alphaOnlyPixels = 0;
	let maxChannelDelta = 0;
	let absoluteChannelError = 0;
	for (let offset = 0; offset < baseline.data.length; offset += 4) {
		let pixelChanged = false;
		let rgbChanged = false;
		let alphaChanged = false;
		for (let channel = 0; channel < 4; channel++) {
			const delta = Math.abs(baseline.data[offset + channel] - candidate.data[offset + channel]);
			absoluteChannelError += delta;
			if (delta > 0) {
				changedChannels++;
				pixelChanged = true;
				if (channel === 3) alphaChanged = true;
				else rgbChanged = true;
				maxChannelDelta = Math.max(maxChannelDelta, delta);
			}
		}
		if (pixelChanged) changedPixels++;
		if (alphaChanged && !rgbChanged) alphaOnlyPixels++;
	}
	const pixels = baseline.width * baseline.height;
	return {
		width: baseline.width,
		height: baseline.height,
		pixels,
		changedPixels,
		changedPixelPercent: (changedPixels / pixels) * 100,
		changedChannels,
		alphaOnlyPixels,
		maxChannelDelta,
		meanAbsoluteChannelError: absoluteChannelError / baseline.data.length,
		pngBytes
	};
}

function qualityCsv(results) {
	const rows = [
		[
			'variant',
			'source',
			'scale',
			'width',
			'height',
			'pngBytes',
			'changedPixels',
			'changedPixelPercent',
			'changedChannels',
			'alphaOnlyPixels',
			'maxChannelDelta',
			'meanAbsoluteChannelError'
		]
	];
	for (const result of results) {
		rows.push([
			result.variant,
			result.source,
			result.scale,
			result.width,
			result.height,
			result.pngBytes,
			result.diff?.changedPixels ?? 0,
			result.diff?.changedPixelPercent ?? 0,
			result.diff?.changedChannels ?? 0,
			result.diff?.alphaOnlyPixels ?? 0,
			result.diff?.maxChannelDelta ?? 0,
			result.diff?.meanAbsoluteChannelError ?? 0
		]);
	}
	return `${rows.map((row) => row.map(csvCell).join(',')).join('\n')}\n`;
}

function csvCell(value) {
	const text = String(value);
	return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
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
			throw new Error(`Unsupported image type: ${filePath}`);
	}
}

function scaleLabel(scale) {
	return String(scale).replace('0.', '').replace('.', '_');
}

function safePathSegment(value) {
	return value.replaceAll(/[^a-z0-9._-]+/gi, '-').replaceAll(/^-|-$/g, '');
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
