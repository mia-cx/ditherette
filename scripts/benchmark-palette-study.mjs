#!/usr/bin/env node
import { mkdir, readFile, writeFile } from 'node:fs/promises';
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
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const outDir = path.resolve(root, args.out ?? `benchmark-results/palette-study-${timestamp}`);

const server = await createServer({
	root,
	configFile: path.join(root, 'vite.config.ts'),
	appType: 'custom',
	logLevel: 'silent',
	server: { middlewareMode: true, hmr: false }
});
let browser;

try {
	const study = await server.ssrLoadModule('/src/lib/benchmark/palette-study.ts');
	const imagePath = path.resolve(root, args.image ?? 'benchmark-fixtures/Celeste_box_art_full.png');
	browser = await chromium.launch({ headless: true });
	console.log(`Decoding ${path.relative(root, imagePath)}...`);
	const source = await decodeBenchmarkImage(browser, root, imagePath);
	const result = study.runPaletteStudy({
		sources: [source],
		iterations: args.iterations,
		warmups: args.warmups,
		studies: args.studies,
		variants: args.variants,
		colorSpaces: args.colorSpaces,
		dithers: args.dithers
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
	const jsonPath = path.join(outDir, 'palette-study.json');
	const csvPath = path.join(outDir, 'palette-study.csv');
	await writeFile(jsonPath, `${JSON.stringify(artifact, null, 2)}\n`);
	await writeFile(csvPath, `${study.paletteStudyResultsToCsv(result)}\n`);
	console.log(study.formatPaletteStudyTable(result));
	console.log(`\nWrote ${path.relative(root, jsonPath)}`);
	console.log(`Wrote ${path.relative(root, csvPath)}`);
} finally {
	await browser?.close();
	await server.close();
}

async function decodeBenchmarkImage(browser, root, imagePath) {
	const start = performance.now();
	const bytes = await readFile(imagePath);
	const dataUrl = `data:${mimeForPath(imagePath)};base64,${bytes.toString('base64')}`;
	const page = await browser.newPage();
	try {
		const decoded = await page.evaluate(async (url) => {
			const image = new Image();
			image.decoding = 'sync';
			const loaded = new Promise((resolve, reject) => {
				image.onload = () => resolve();
				image.onerror = () => reject(new Error('Failed to decode benchmark image'));
			});
			image.src = url;
			await loaded;
			const canvas = document.createElement('canvas');
			canvas.width = image.naturalWidth;
			canvas.height = image.naturalHeight;
			const context = canvas.getContext('2d', { willReadFrequently: true });
			if (!context) throw new Error('2D canvas unavailable');
			context.drawImage(image, 0, 0);
			const data = context.getImageData(0, 0, canvas.width, canvas.height);
			window.__paletteStudyImageBytes = data.data;
			return { width: data.width, height: data.height, length: data.data.length };
		}, dataUrl);
		const bytes = new Uint8ClampedArray(decoded.length);
		const chunkSize = 1 << 20;
		for (let startOffset = 0; startOffset < decoded.length; startOffset += chunkSize) {
			const chunk = await page.evaluate(
				({ start, end }) => Array.from(window.__paletteStudyImageBytes.slice(start, end)),
				{ start: startOffset, end: Math.min(decoded.length, startOffset + chunkSize) }
			);
			bytes.set(chunk, startOffset);
		}
		const imageData = new ImageData(bytes, decoded.width, decoded.height);
		return {
			id: path.basename(imagePath, path.extname(imagePath)),
			label: path.basename(imagePath),
			imageData,
			path: path.relative(root, imagePath),
			decodeMs: performance.now() - start
		};
	} finally {
		await page.close();
	}
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

function parseArgs(argv) {
	const options = {};
	for (let index = 0; index < argv.length; index++) {
		const arg = argv[index];
		if (arg === '--') continue;
		if (!arg.startsWith('--')) throw new Error(`Unexpected positional argument: ${arg}`);
		const [key, inlineValue] = arg.slice(2).split('=', 2);
		const nextValue = () => inlineValue ?? argv[++index];
		switch (key) {
			case 'image':
				options.image = nextValue();
				break;
			case 'iterations':
				options.iterations = Number.parseInt(nextValue(), 10);
				break;
			case 'warmups':
				options.warmups = Number.parseInt(nextValue(), 10);
				break;
			case 'study':
			case 'studies':
				options.studies = splitList(nextValue());
				break;
			case 'variant':
			case 'variants':
				options.variants = splitList(nextValue());
				break;
			case 'color-space':
			case 'colorspace':
				options.colorSpaces = splitList(nextValue());
				break;
			case 'dither':
				options.dithers = splitList(nextValue());
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

function splitList(value) {
	return value
		.split(',')
		.map((item) => item.trim())
		.filter(Boolean);
}

function printHelp() {
	console.log(`Usage: pnpm bench:palette-study -- [options]

Options:
  --image FILE                  Image to decode (default: benchmark-fixtures/Celeste_box_art_full.png)
  --iterations N                Recorded iterations per study row (default: 3)
  --warmups N                   Warmups per study row (default: 1)
  --study id                    Comma-separated studies (default: direct-byte-rgb)
  --variant id                  Comma-separated variants (scan,distance-tables,dense-rgb-distance-tables)
  --color-space id              Comma-separated color spaces
  --out DIR                     Artifact directory

Examples:
  pnpm bench:palette-study -- --image benchmark-fixtures/Celeste_box_art_full.png --study direct-byte-rgb --color-space srgb
`);
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
