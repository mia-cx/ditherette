#!/usr/bin/env node
import { createServer } from 'node:http';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const pkgDir = path.join(root, 'static/wasm/ditherette-wasm');
const defaultManifestPath = path.join(root, 'scripts/ditherette-wasm-bench.toml');
const artifactSchema = 'ditherette-wasm-bench';
const artifactSchemaVersion = 1;
const significantNumberFormatter = new Intl.NumberFormat('en-US', {
	maximumSignificantDigits: 4,
	minimumSignificantDigits: 4,
	useGrouping: false
});

const options = await resolveOptions(process.argv.slice(2));
const outDir = path.resolve(
	root,
	options.outputDir ?? `benchmark-results/wasm-resize-${timestamp}`
);

await assertFile(
	path.join(pkgDir, 'ditherette_wasm.js'),
	'Run `pnpm wasm:build` before benchmarking.'
);
await assertFile(
	path.join(pkgDir, 'ditherette_wasm_bg.wasm'),
	'Run `pnpm wasm:build` before benchmarking.'
);

const fixtures = await benchmarkFixtures(options.fixtures);
const server = await startBenchmarkServer({ pkgDir, fixtures });
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

	const browserResult = await page.evaluate(async (config) => globalThis.runResizeBench(config), {
		fixtures: fixtures.map((fixture) => ({
			name: fixture.name,
			url: `/fixtures/${encodeURIComponent(fixture.name)}`
		})),
		subjects: options.subjects.map(subjectConfig),
		scales: options.scales,
		lanes: options.lanes,
		sampleSize: options.sampleSize,
		warmUpIterations: options.warmUpIterations
	});
	const artifact = benchRunArtifact(options, browserResult);

	await mkdir(outDir, { recursive: true });
	const jsonPath = path.join(outDir, `${sanitizePathComponent(options.profile ?? 'ad-hoc')}.json`);
	await writeFile(jsonPath, `${JSON.stringify(artifact, null, 2)}\n`);

	clearStatusLine();
	console.log(formatResultTable(artifact));
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

async function benchmarkFixtures(values) {
	const fixtures = [];
	for (const value of values) {
		const file = await resolveFixtureFile(value);
		fixtures.push({ name: path.basename(file), file });
	}
	return fixtures;
}

async function resolveFixtureFile(value) {
	const candidates = path.isAbsolute(value)
		? [value]
		: [
				path.resolve(root, value),
				path.resolve(root, 'benchmark-fixtures', value),
				path.resolve(root, 'benchmark-fixtures', `${value}.png`),
				path.resolve(root, 'benchmark-fixtures', `${value}.jpg`),
				path.resolve(root, 'benchmark-fixtures', `${value}.webp`)
			];

	for (const candidate of candidates) {
		if (await fileExists(candidate)) return candidate;
	}
	throw new Error(`Benchmark fixture not found: ${value}`);
}

async function startBenchmarkServer({ pkgDir, fixtures }) {
	const fixturesByName = new Map(fixtures.map((fixture) => [fixture.name, fixture.file]));
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
				const fileName = decodeURIComponent(path.basename(requestUrl.pathname));
				const fixtureFile = fixturesByName.get(fileName);
				if (!fixtureFile) throw new Error(`Unknown fixture: ${fileName}`);
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
	return String.raw`import init, { resizeRgba8 } from '/pkg/ditherette_wasm.js';

const RGBA_CHANNEL_COUNT = 4;

globalThis.runResizeBench = async function runResizeBench(config) {
	await init('/pkg/ditherette_wasm_bg.wasm');

	const decodedFixtures = [];
	for (const fixture of config.fixtures) decodedFixtures.push(await decodeFixture(fixture));

	const cases = decodedFixtures.flatMap((decodedFixture) => makeScaleCases(decodedFixture, config.scales, config.lanes));
	const totalRuns = cases.length * config.subjects.length;
	let completedRuns = 0;
	const results = [];
	reportProgress(completedRuns, totalRuns, 'starting');

	for (const benchmarkCase of cases) {
		for (const subject of config.subjects) {
			reportProgress(completedRuns, totalRuns, benchmarkCase.id + ' / ' + subject.id);
			results.push(await measureResizeSubject(benchmarkCase, subject, config));
			completedRuns += 1;
			reportProgress(completedRuns, totalRuns, benchmarkCase.id + ' / ' + subject.id);
		}
	}

	assertStableChecksums(results);

	return {
		crossOriginIsolated: globalThis.crossOriginIsolated,
		userAgent: navigator.userAgent,
		fixtures: decodedFixtures.map((fixture) => ({
			name: fixture.name,
			width: fixture.sourceWidth,
			height: fixture.sourceHeight,
			decodeNs: millisecondsToNanoseconds(fixture.decodeMs),
			normalizeNs: millisecondsToNanoseconds(fixture.normalizeMs)
		})),
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

	return { name: fixture.name, sourceWidth, sourceHeight, sourceRgba, decodeMs, normalizeMs };
}

function makeScaleCases(decodedFixture, scales, lanes) {
	const cases = [];

	for (const scale of scales) {
		const outputWidth = scaledDimension(decodedFixture.sourceWidth, scale);
		const outputHeight = scaledDimension(decodedFixture.sourceHeight, scale);
		const id = decodedFixture.name + '-' + scaleLabel(scale);

		if (lanes.includes('browser-decode-rgba')) {
			cases.push({
				id: id + '-browser-decode',
				fixture: { name: decodedFixture.name, width: decodedFixture.sourceWidth, height: decodedFixture.sourceHeight },
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
		}

		if (lanes.includes('decoded-rgba')) {
			cases.push({
				id: id + '-decoded-rgba',
				fixture: { name: decodedFixture.name, width: decodedFixture.sourceWidth, height: decodedFixture.sourceHeight },
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
	}

	return cases;
}

async function measureResizeSubject(benchmarkCase, subject, config) {
	const expectedByteLength = benchmarkCase.outputWidth * benchmarkCase.outputHeight * RGBA_CHANNEL_COUNT;

	for (let index = 0; index < config.warmUpIterations; index += 1) {
		resizeSubject(benchmarkCase, subject);
	}

	const timingsMs = [];
	const checksums = [];
	let outputByteLength = 0;

	for (let index = 0; index < config.sampleSize; index += 1) {
		const started = performance.now();
		const outputRgba = resizeSubject(benchmarkCase, subject);
		const wasmMs = performance.now() - started;

		outputByteLength = outputRgba.byteLength;
		timingsMs.push(wasmMs);
		checksums.push(checksumBytes(outputRgba));
	}

	return resizeResult(benchmarkCase, subject, timingsMs, checksums, outputByteLength, expectedByteLength);
}

function resizeSubject(benchmarkCase, subject) {
	return resizeRgba8(
		benchmarkCase.sourceRgba,
		benchmarkCase.sourceWidth,
		benchmarkCase.sourceHeight,
		benchmarkCase.outputWidth,
		benchmarkCase.outputHeight,
		subject.filter,
		subject.anchor,
		subject.supportPolicy,
		subject.parallelizationPolicy
	);
}

function resizeResult(benchmarkCase, subject, timingsMs, checksums, outputByteLength, expectedByteLength) {
	const resultId = benchmarkCase.id + '-' + subject.id;

	if (outputByteLength !== expectedByteLength) {
		throw new Error(resultId + ' produced ' + outputByteLength + ' bytes; expected ' + expectedByteLength);
	}

	if (new Set(checksums).size !== 1) {
		throw new Error(resultId + ' produced unstable checksums: ' + checksums.join(', '));
	}

	return {
		id: benchmarkCase.id,
		subject: subject.id,
		filter: subject.filter,
		supportPolicy: subject.supportPolicy,
		fixture: benchmarkCase.fixture,
		lane: benchmarkCase.lane,
		scale: benchmarkCase.scale,
		source: { width: benchmarkCase.sourceWidth, height: benchmarkCase.sourceHeight },
		output: { width: benchmarkCase.outputWidth, height: benchmarkCase.outputHeight, byteLength: outputByteLength },
		decodeNs: millisecondsToNanoseconds(benchmarkCase.decodeMs),
		normalizeNs: millisecondsToNanoseconds(benchmarkCase.normalizeMs),
		statsNs: summarizeTimings(timingsMs),
		checksum: checksums[0]
	};
}

function assertStableChecksums(results) {
	for (const result of results) {
		if (!Number.isInteger(result.checksum)) throw new Error(result.id + ' missing checksum');
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

function benchRunArtifact(options, browserResult) {
	const createdAtUnix = Math.floor(Date.now() / 1000);
	return {
		schema: artifactSchema,
		schemaVersion: artifactSchemaVersion,
		artifactKind: 'run',
		runId: runId(createdAtUnix),
		createdAtUnix,
		createdAt: new Date(createdAtUnix * 1000).toISOString(),
		command: options.command,
		domain: options.domain,
		profile: options.profile ?? 'ad-hoc',
		tool: {
			name: 'ditherette-wasm-bench',
			script: 'scripts/benchmark-wasm-resize.mjs'
		},
		environment: {
			userAgent: browserResult.userAgent,
			crossOriginIsolated: browserResult.crossOriginIsolated
		},
		measurement: {
			sampleSize: options.sampleSize,
			warmUpIterations: options.warmUpIterations,
			sampleMode: 'interactive'
		},
		config: {
			subjects: options.subjects,
			fixtures: browserResult.fixtures,
			scales: options.scales,
			lanes: options.lanes
		},
		results: browserResult.results
	};
}

function runId(createdAtUnix) {
	return `${createdAtUnix}-${Math.random().toString(16).slice(2, 10)}`;
}

function subjectConfig(id) {
	const parts = id.split(':');
	if (parts.length !== 4 || parts[0] !== 'wasm' || parts[1] !== 'resize') {
		throw new Error(`Unsupported Wasm bench subject: ${id}`);
	}
	const family = parts[2];
	const variant = parts[3];
	if (variant === 'scale-aware') {
		return {
			id,
			filter: family,
			anchor: 'center',
			supportPolicy: 'scale-aware',
			parallelizationPolicy: true
		};
	}
	if (variant !== 'scalar' && variant !== 'fixed') {
		throw new Error(`Unsupported Wasm resize subject variant: ${id}`);
	}
	return {
		id,
		filter: family,
		anchor: 'center',
		supportPolicy: 'fixed',
		parallelizationPolicy: true
	};
}

function formatResultTable(run) {
	const baselines = baselineResultsByCase(run.results);
	const rows = run.results.map((entry) => {
		const baseline = baselines.get(`${entry.fixture.name}:${entry.id}`);

		return {
			fixture: entry.fixture.name,
			case: entry.id,
			subject: entry.subject,
			lane: entry.lane,
			scale: `${entry.scale}×`,
			source: `${entry.source.width}×${entry.source.height}px`,
			output: `${entry.output.width}×${entry.output.height}px`,
			decode: formatDuration(entry.decodeNs),
			normalize: formatDuration(entry.normalizeNs),
			mean: formatDurationDelta(entry.statsNs.mean, baseline?.statsNs.mean),
			median: formatDurationDelta(entry.statsNs.median, baseline?.statsNs.median),
			mode: formatDurationDelta(entry.statsNs.mode, baseline?.statsNs.mode),
			min: formatDurationDelta(entry.statsNs.min, baseline?.statsNs.min),
			p1: formatDurationDelta(entry.statsNs.p1, baseline?.statsNs.p1),
			p2: formatDurationDelta(entry.statsNs.p2, baseline?.statsNs.p2),
			p5: formatDurationDelta(entry.statsNs.p5, baseline?.statsNs.p5),
			p25: formatDurationDelta(entry.statsNs.p25, baseline?.statsNs.p25),
			p50: formatDurationDelta(entry.statsNs.p50, baseline?.statsNs.p50),
			p75: formatDurationDelta(entry.statsNs.p75, baseline?.statsNs.p75),
			p95: formatDurationDelta(entry.statsNs.p95, baseline?.statsNs.p95),
			p98: formatDurationDelta(entry.statsNs.p98, baseline?.statsNs.p98),
			p99: formatDurationDelta(entry.statsNs.p99, baseline?.statsNs.p99),
			max: formatDurationDelta(entry.statsNs.max, baseline?.statsNs.max),
			checksum: entry.checksum
		};
	});

	return [
		`Profile: ${run.profile}`,
		`Subjects: ${run.config.subjects.join(', ')}`,
		`Fixtures: ${run.config.fixtures.map((fixture) => `${fixture.name} (${fixture.width}×${fixture.height})`).join(', ')}`,
		`Browser: ${run.environment.userAgent}`,
		`Cross-origin isolated: ${run.environment.crossOriginIsolated}`,
		`Samples: ${run.measurement.sampleSize}, warmups: ${run.measurement.warmUpIterations}`,
		table(rows)
	].join('\n');
}

function baselineResultsByCase(results) {
	const baselines = new Map();
	for (const result of results) {
		const key = `${result.fixture.name}:${result.id}`;
		if (!baselines.has(key)) baselines.set(key, result);
	}
	return baselines;
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

async function resolveOptions(rawArgs) {
	const parsed = parseArgs(rawArgs);
	const manifest = await loadManifest(parsed.configPath);
	const defaults = manifest.defaults ?? {};
	const profile = parsed.profile ?? parsed.runProfile;
	const profileValues = profile ? (manifest.profiles?.[profile] ?? unknownProfile(profile)) : {};
	const merged = { ...defaults, ...profileValues, ...parsed.overrides };

	return {
		command: stringValue(merged.command, 'perf'),
		domain: stringValue(merged.domain, 'resize'),
		profile,
		subjects: stringArray(merged.subjects, ['wasm:resize:nearest:scalar']),
		fixtures: stringArray(merged.fixtures ?? merged.image, ['Celeste_box_art.png']),
		scales: numberArray(merged.scales, [2, 0.95, 0.75, 0.5, 0.25, 0.125]),
		lanes: stringArray(merged.lanes, ['browser-decode-rgba', 'decoded-rgba']),
		sampleSize: positiveInteger(
			merged.sample_size ?? merged.sampleSize ?? merged.iterations,
			'--sample-size',
			5
		),
		warmUpIterations: positiveInteger(
			merged.warm_up_iterations ?? merged.warmUpIterations ?? merged.warmups,
			'--warm-up-iterations',
			2
		),
		outputDir: stringValue(merged.output_dir ?? merged.outputDir ?? merged.out, undefined)
	};
}

function parseArgs(rawArgs) {
	const parsed = {
		runProfile: undefined,
		profile: undefined,
		configPath: undefined,
		overrides: {}
	};
	const args = [...rawArgs];
	if (args[0] === '--') args.shift();
	if (args[0] === 'run') {
		args.shift();
		parsed.runProfile = args.shift();
		if (!parsed.runProfile) throw new Error('run requires a profile name');
	}

	for (let index = 0; index < args.length; index += 1) {
		const arg = args[index];
		const nextValue = () => {
			index += 1;
			if (index >= args.length) throw new Error(`Missing value for ${arg}`);
			return args[index];
		};

		switch (arg) {
			case '--profile':
				parsed.profile = nextValue();
				break;
			case '--config':
				parsed.configPath = nextValue();
				break;
			case '--image':
				parsed.overrides.fixtures = [nextValue()];
				break;
			case '--fixtures':
				parsed.overrides.fixtures = commaList(nextValue());
				break;
			case '--subjects':
				parsed.overrides.subjects = commaList(nextValue());
				break;
			case '--scales':
				parsed.overrides.scales = commaList(nextValue()).map(Number);
				break;
			case '--lanes':
				parsed.overrides.lanes = commaList(nextValue());
				break;
			case '--sample-size':
			case '--iterations':
				parsed.overrides.sample_size = Number(nextValue());
				break;
			case '--warm-up-iterations':
			case '--warmups':
				parsed.overrides.warm_up_iterations = Number(nextValue());
				break;
			case '--output-dir':
			case '--out':
				parsed.overrides.output_dir = nextValue();
				break;
			case '--help':
				console.log(helpText());
				process.exit(0);
				break;
			default:
				throw new Error(`Unknown argument: ${arg}\n\n${helpText()}`);
		}
	}

	return parsed;
}

async function loadManifest(configPath) {
	const pathToManifest = path.resolve(root, configPath ?? defaultManifestPath);
	if (!(await fileExists(pathToManifest))) return {};
	return parseTomlSubset(await readFile(pathToManifest, 'utf8'));
}

function parseTomlSubset(text) {
	const root = {};
	let current = root;
	let pendingArray;

	for (const rawLine of text.split(/\r?\n/)) {
		const line = stripComment(rawLine).trim();
		if (!line) continue;

		if (pendingArray) {
			pendingArray.value += ` ${line}`;
			if (line.includes(']')) {
				pendingArray.target[pendingArray.key] = parseValue(pendingArray.value);
				pendingArray = undefined;
			}
			continue;
		}

		const sectionMatch = line.match(/^\[([^\]]+)]$/);
		if (sectionMatch) {
			current = sectionFor(root, sectionMatch[1]);
			continue;
		}

		const keyValue = line.match(/^([A-Za-z0-9_-]+)\s*=\s*(.+)$/);
		if (!keyValue) throw new Error(`Unsupported manifest line: ${rawLine}`);
		const [, key, value] = keyValue;
		if (value.startsWith('[') && !value.includes(']')) {
			pendingArray = { target: current, key, value };
			continue;
		}
		current[key] = parseValue(value);
	}

	if (pendingArray) throw new Error(`Unclosed manifest array for ${pendingArray.key}`);
	return root;
}

function sectionFor(root, dottedName) {
	let current = root;
	for (const part of dottedName.split('.')) {
		current[part] ??= {};
		current = current[part];
	}
	return current;
}

function stripComment(line) {
	let inString = false;
	for (let index = 0; index < line.length; index += 1) {
		const char = line[index];
		if (char === '"' && line[index - 1] !== '\\') inString = !inString;
		if (char === '#' && !inString) return line.slice(0, index);
	}
	return line;
}

function parseValue(value) {
	const trimmed = value.trim().replace(/,$/, '');
	if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
		return splitArray(trimmed.slice(1, -1)).map(parseValue);
	}
	if (trimmed.startsWith('"') && trimmed.endsWith('"')) return trimmed.slice(1, -1);
	if (trimmed === 'true') return true;
	if (trimmed === 'false') return false;
	const numeric = Number(trimmed);
	if (Number.isFinite(numeric)) return numeric;
	return trimmed;
}

function splitArray(value) {
	const values = [];
	let inString = false;
	let start = 0;
	for (let index = 0; index < value.length; index += 1) {
		const char = value[index];
		if (char === '"' && value[index - 1] !== '\\') inString = !inString;
		if (char !== ',' || inString) continue;
		values.push(value.slice(start, index).trim());
		start = index + 1;
	}
	const tail = value.slice(start).trim();
	if (tail) values.push(tail);
	return values;
}

function unknownProfile(profile) {
	throw new Error(`unknown Wasm benchmark profile ${JSON.stringify(profile)}`);
}

function stringValue(value, fallback) {
	if (value === undefined) return fallback;
	return String(value);
}

function stringArray(value, fallback) {
	if (value === undefined) return fallback;
	if (Array.isArray(value)) return value.map(String);
	return commaList(String(value));
}

function numberArray(value, fallback) {
	const values =
		value === undefined ? fallback : Array.isArray(value) ? value : commaList(String(value));
	return values.map((entry) => {
		const parsed = Number(entry);
		if (!Number.isFinite(parsed) || parsed <= 0)
			throw new Error(`scale must be positive: ${entry}`);
		return parsed;
	});
}

function commaList(value) {
	return String(value)
		.split(',')
		.map((entry) => entry.trim())
		.filter(Boolean);
}

function positiveInteger(value, name, fallback) {
	const parsed = value === undefined ? fallback : Number(value);
	if (!Number.isInteger(parsed) || parsed <= 0)
		throw new Error(`${name} must be a positive integer`);
	return parsed;
}

function helpText() {
	return `Usage:
  pnpm bench:resize:wasm -- run PROFILE [overrides...]
  pnpm bench:resize:wasm -- [overrides...]

Manifest flags:
  --profile NAME             Load [profiles.NAME] from scripts/ditherette-wasm-bench.toml.
  --config PATH              Use an alternate TOML manifest.

Measurement flags:
  --sample-size N            Recorded samples per case. Default from manifest: 5.
  --iterations N             Alias for --sample-size.
  --warm-up-iterations N     Warmup iterations per case. Default from manifest: 2.
  --warmups N                Alias for --warm-up-iterations.

Case flags:
  --subjects IDS             Comma-separated subject ids, e.g. wasm:resize:nearest:scalar.
  --fixtures FILES           Comma-separated fixture paths or benchmark-fixtures stems.
  --image FILE               Legacy alias for one fixture.
  --scales VALUES            Comma-separated isotropic scales.
  --lanes LANES              browser-decode-rgba,decoded-rgba.
  --output-dir DIR           Output directory. Default: benchmark-results/wasm-resize-<timestamp>.
  --out DIR                  Alias for --output-dir.

Examples:
  pnpm bench:resize:wasm -- run nearest-smoke
  pnpm bench:resize:wasm -- run nearest --sample-size 20
  pnpm bench:resize:wasm -- --subjects wasm:resize:lanczos3:scale-aware --fixtures Celeste_box_art --scales 0.5`;
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

function sanitizePathComponent(value) {
	return value.replace(/[^A-Za-z0-9_.-]+/g, '-').replace(/^-|-$/g, '') || 'run';
}
