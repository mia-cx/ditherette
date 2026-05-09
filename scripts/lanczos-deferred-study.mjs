#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const RESIZE_PATH = 'src/lib/processing/resize.ts';
const DEFAULT_BENCH_ARGS = ['--scale', '--iterations', '5', '--warmups', '2', '--quiet', 'true'];

const VARIANTS = [
	{
		id: 'baseline',
		description: 'Committed optimized Lanczos3 implementation',
		kind: 'control',
		apply: (source) => source
	},
	{
		id: 'dynamic-lanczos3-current-window',
		description: 'Generic dynamic-tap implementation with current six-tap Lanczos3 window',
		kind: 'implementation-control',
		apply: (source) => applyDynamicLanczos(source, currentLanczosConfig())
	},
	{
		id: 'fixed-point-lanczos3-current-window',
		description: 'Current six-tap Lanczos3 window with fixed-point contribution weights',
		kind: 'rounding-risk',
		apply: (source) => applyDynamicLanczos(source, { ...currentLanczosConfig(), fixedPoint: true })
	},
	{
		id: 'scale-aware-lanczos3',
		description: 'Pica-style Lanczos3 with downscale-aware widened source window',
		kind: 'output-changing',
		apply: (source) =>
			applyDynamicLanczos(source, {
				filter: 'lanczos3',
				window: 3,
				windowMode: 'pica-scale-aware',
				fixedPoint: false
			})
	},
	{
		id: 'lanczos2',
		description: 'Pica-style Lanczos2 filter candidate',
		kind: 'output-changing',
		apply: (source) =>
			applyDynamicLanczos(source, {
				filter: 'lanczos2',
				window: 2,
				windowMode: 'pica-scale-aware',
				fixedPoint: false
			})
	},
	{
		id: 'hamming',
		description: 'Pica-style Hamming filter candidate',
		kind: 'output-changing',
		apply: (source) =>
			applyDynamicLanczos(source, {
				filter: 'hamming',
				window: 1,
				windowMode: 'pica-scale-aware',
				fixedPoint: false
			})
	},
	{
		id: 'mks2013',
		description: 'Pica-style Magic Kernel Sharp 2013 filter candidate',
		kind: 'output-changing',
		apply: (source) =>
			applyDynamicLanczos(source, {
				filter: 'mks2013',
				window: 2.5,
				windowMode: 'pica-scale-aware',
				fixedPoint: false
			})
	},
	{
		id: 'box',
		description: 'Pica-style box filter candidate',
		kind: 'output-changing',
		apply: (source) =>
			applyDynamicLanczos(source, {
				filter: 'box',
				window: 0.5,
				windowMode: 'pica-scale-aware',
				fixedPoint: false
			})
	}
];

function main() {
	const { outRoot, benchArgs, checkCorrectness, emitSource } = parseArgs(process.argv.slice(2));
	const originalWorkingTree = readFileSync(RESIZE_PATH, 'utf8');
	const baseline = gitShow(`HEAD:${RESIZE_PATH}`);
	if (emitSource) {
		const variant = VARIANTS.find((candidate) => candidate.id === emitSource);
		if (!variant) throw new Error(`Unknown Lanczos variant: ${emitSource}`);
		process.stdout.write(variant.apply(baseline));
		return;
	}
	const startedAt = new Date().toISOString();
	const root =
		outRoot ?? join('benchmark-results', `lanczos-deferred-study-${safeTimestamp(startedAt)}`);
	mkdirSync(root, { recursive: true });

	const runs = [];
	try {
		for (const variant of VARIANTS) {
			const variantSource = variant.apply(baseline);
			writeFileSync(RESIZE_PATH, variantSource);

			const correctness = checkCorrectness
				? runCorrectnessCheck(root, variant.id)
				: { skipped: true };
			const outDir = join(root, variant.id);
			const args = [...withoutOutArg(benchArgs), '--out', outDir];
			console.log(`\n=== ${variant.id}: ${variant.description} ===`);
			console.log(`node scripts/benchmark-processing.mjs ${args.join(' ')}`);
			const bench = spawnSync(process.execPath, ['scripts/benchmark-processing.mjs', ...args], {
				stdio: 'inherit'
			});
			if (bench.status !== 0)
				throw new Error(`${variant.id} benchmark failed with ${bench.status}`);
			const summary = summarizeBenchmark(join(outDir, 'processing-benchmark.json'));
			runs.push({ ...variant, apply: undefined, outDir, correctness, summary });
			writeStudySummary(root, startedAt, benchArgs, runs);
		}
	} finally {
		writeFileSync(RESIZE_PATH, originalWorkingTree);
	}

	writeStudySummary(root, startedAt, benchArgs, runs);
	console.log(`\nWrote ${join(root, 'study-summary.json')}`);
	console.log(`Wrote ${join(root, 'study-summary.csv')}`);
}

function parseArgs(argv) {
	const separator = argv.indexOf('--');
	const scriptArgs = separator === -1 ? argv : argv.slice(0, separator);
	const explicitBenchArgs = separator === -1 ? [] : argv.slice(separator + 1);
	let outRoot;
	let checkCorrectness = false;
	let emitSource;
	const inferredBenchArgs = [];

	for (let index = 0; index < scriptArgs.length; index++) {
		const arg = scriptArgs[index];
		if (arg === '--out') {
			outRoot = scriptArgs[++index];
			continue;
		}
		if (arg === '--check-correctness') {
			checkCorrectness = true;
			continue;
		}
		if (arg === '--emit-source') {
			emitSource = scriptArgs[++index];
			continue;
		}
		if (arg === '--help' || arg === '-h') {
			printHelp();
			process.exit(0);
		}

		inferredBenchArgs.push(arg);
		const next = scriptArgs[index + 1];
		if (next && !next.startsWith('--')) inferredBenchArgs.push(scriptArgs[++index]);
	}

	const benchArgs = explicitBenchArgs.length ? explicitBenchArgs : inferredBenchArgs;
	return {
		outRoot,
		benchArgs: studyBenchArgs(benchArgs),
		checkCorrectness,
		emitSource
	};
}

function studyBenchArgs(args) {
	if (!args.length) return DEFAULT_BENCH_ARGS;
	return withDefaultOption(
		withDefaultOption(withDefaultOption(args, '--iterations', '5'), '--warmups', '2'),
		'--quiet',
		'true'
	);
}

function withDefaultOption(args, option, value) {
	if (args.includes(option)) return args;
	return [...args, option, value];
}

function printHelp() {
	console.log(`Usage: node scripts/lanczos-deferred-study.mjs [study options] -- [benchmark options]

Runs deferred Lanczos variants from committed resize.ts:
  1. baseline optimized current Lanczos3
  2. dynamic-lanczos3-current-window implementation control
  3. fixed-point-lanczos3-current-window rounding-risk variant
  4. scale-aware-lanczos3 pica-style downscale window
  5. lanczos2
  6. hamming
  7. mks2013
  8. box

Study options:
  --out DIR              Root output directory (default: benchmark-results/lanczos-deferred-study-<timestamp>)
  --emit-source ID       Print the transformed resize.ts source for one variant, then exit
  --check-correctness    Run resize.spec.ts for each variant before benchmarking.
                         Output-changing variants are expected to fail exact Lanczos guards;
                         failures are logged but do not stop the study.

Benchmark options default to Lanczos-only resize timing:
  --scale --iterations 5 --warmups 2 --quiet true

Use --resize only when you intentionally want non-Lanczos modes included as controls.

Examples:
  pnpm bench:lanczos-deferred-study
  pnpm bench:lanczos-deferred-study -- --scale --iterations 10 --warmups 3 --quiet true
`);
}

function gitShow(ref) {
	const result = spawnSync('git', ['show', ref], { encoding: 'utf8' });
	if (result.status !== 0) throw new Error(result.stderr || `git show ${ref} failed`);
	return result.stdout;
}

function runCorrectnessCheck(root, variantId) {
	const result = spawnSync(
		'pnpm',
		['test:unit', '--', '--run', 'src/lib/processing/resize.spec.ts'],
		{ encoding: 'utf8' }
	);
	const correctness = {
		passed: result.status === 0,
		status: result.status,
		stdout: result.stdout,
		stderr: result.stderr
	};
	writeFileSync(
		join(root, `${variantId}-correctness.log`),
		`${result.stdout ?? ''}\n${result.stderr ?? ''}`
	);
	if (!correctness.passed) {
		console.warn(`${variantId} correctness check failed; continuing for measurement.`);
	}
	return correctness;
}

function withoutOutArg(args) {
	const cleaned = [];
	for (let index = 0; index < args.length; index++) {
		if (args[index] === '--out') {
			index++;
			continue;
		}
		cleaned.push(args[index]);
	}
	return cleaned;
}

function summarizeBenchmark(path) {
	const data = JSON.parse(readFileSync(path, 'utf8'));
	const byResize = {};
	const byScale = {};
	const bySource = {};
	for (const result of data.results) {
		const resize = parseResizeMode(result.case.id);
		const scale = parseScale(result.case.id);
		byResize[resize] ??= { count: 0, resizeMs: 0, totalMs: 0 };
		byScale[scale] ??= { count: 0, resizeMs: 0, totalMs: 0 };
		bySource[result.source.id] ??= { count: 0, resizeMs: 0, totalMs: 0 };
		for (const bucket of [byResize[resize], byScale[scale], bySource[result.source.id]]) {
			bucket.count++;
			bucket.resizeMs += result.stages.resize.meanMs;
			bucket.totalMs += result.stages.total.meanMs;
		}
	}
	return {
		profile: data.profile,
		runAt: data.runAt,
		resultCount: data.results.length,
		resizeMs: sum(data.results, (result) => result.stages.resize.meanMs),
		totalMs: sum(data.results, (result) => result.stages.total.meanMs),
		byResize,
		byScale,
		bySource
	};
}

function parseResizeMode(caseId) {
	return caseId.split('-')[2] ?? 'unknown';
}

function parseScale(caseId) {
	return caseId.split('-')[1] ?? 'unknown';
}

function sum(values, getter) {
	return values.reduce((total, value) => total + getter(value), 0);
}

function writeStudySummary(root, startedAt, benchArgs, runs) {
	const serializableRuns = runs.map(({ id, description, kind, outDir, correctness, summary }) => ({
		id,
		description,
		kind,
		outDir,
		correctness: correctness?.skipped
			? { skipped: true }
			: { passed: correctness?.passed, status: correctness?.status },
		summary
	}));
	writeFileSync(
		join(root, 'study-summary.json'),
		`${JSON.stringify({ startedAt, benchArgs, runs: serializableRuns }, null, 2)}\n`
	);
	writeFileSync(join(root, 'study-summary.csv'), studyCsv(serializableRuns));
}

function studyCsv(runs) {
	const scales = ['125', '25', '5', '75'];
	const rows = [
		[
			'variant',
			'kind',
			'correctness',
			'results',
			'resizeMs',
			'totalMs',
			'lanczos3ResizeMs',
			...scales.map((scale) => `scale${scale}ResizeMs`)
		]
	];
	for (const run of runs) {
		rows.push([
			run.id,
			run.kind,
			run.correctness.skipped ? 'skipped' : run.correctness.passed ? 'passed' : 'failed',
			run.summary.resultCount,
			run.summary.resizeMs,
			run.summary.totalMs,
			run.summary.byResize.lanczos3?.resizeMs ?? run.summary.resizeMs,
			...scales.map((scale) => run.summary.byScale[scale]?.resizeMs ?? 0)
		]);
	}
	return `${rows.map((row) => row.map(csvCell).join(',')).join('\n')}\n`;
}

function csvCell(value) {
	const text = String(value);
	return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

function safeTimestamp(timestamp) {
	return timestamp.replaceAll(':', '-').replaceAll('.', '-');
}

function currentLanczosConfig() {
	return { filter: 'lanczos3', window: 3, windowMode: 'current-fixed', fixedPoint: false };
}

function applyDynamicLanczos(source, config) {
	const start = source.indexOf('type ContributionTable = {');
	const end = source.indexOf('\nfunction sampleArea', start);
	if (start === -1 || end === -1) throw new Error('Could not find Lanczos implementation block.');
	return `${source.slice(0, start)}${dynamicLanczosSource(config)}${source.slice(end)}`;
}

function dynamicLanczosSource(config) {
	const fixed = Boolean(config.fixedPoint);
	const weightArray = fixed ? 'Int16Array' : 'Float64Array';
	const totalArray = fixed ? 'Int32Array' : 'Float64Array';
	const weightValue = fixed ? 'Math.round(rawWeight * LANCZOS_FIXED_WEIGHT_SCALE)' : 'rawWeight';
	const fixedPointConstant = fixed ? 'const LANCZOS_FIXED_WEIGHT_SCALE = 16_383;\n' : '';
	return `type ContributionTable = {
	indices: Int32Array;
	offsets: Int32Array;
	lengths: Int32Array;
	weights: ${weightArray};
	totals: ${totalArray};
};

const LANCZOS_FILTER = '${config.filter}';
const LANCZOS_FILTER_WINDOW = ${config.window};
const LANCZOS_WINDOW_MODE = '${config.windowMode}';
${fixedPointConstant}
function resizeLanczos3(
	source: ImageData,
	output: ImageData,
	sourceRect: Rect,
	target: TargetBounds
) {
	const xTable = contributionTable(
		target.width,
		sourceRect.x,
		sourceRect.width / target.width,
		source.width
	);
	const yTable = contributionTable(
		target.height,
		sourceRect.y,
		sourceRect.height / target.height,
		source.height
	);

	if (hasTransparentPixels(source.data)) {
		resizeLanczos3WithAlpha(source, output, target, xTable, yTable);
		return;
	}

	resizeOpaqueLanczos3(source, output, target, xTable, yTable);
}

function hasTransparentPixels(data: Uint8ClampedArray) {
	for (let offset = 3; offset < data.length; offset += 4) {
		if (data[offset] !== 255) return true;
	}
	return false;
}

function resizeOpaqueLanczos3(
	source: ImageData,
	output: ImageData,
	target: TargetBounds,
	xTable: ContributionTable,
	yTable: ContributionTable
) {
	const outputData = output.data;
	const rowCache = new Map<number, Float64Array>();
	const lastUse = lastUseTable(source.height, target.height, yTable);

	for (let y = target.top; y < target.bottom; y++) {
		const targetY = y - target.top;
		const yOffset = yTable.offsets[targetY]!;
		const yLength = yTable.lengths[targetY]!;
		const yTotal = yTable.totals[targetY]!;
		const rows = lanczosRowsForTargetY(source, target.width, xTable, yTable, targetY, rowCache, false);

		for (let x = target.left; x < target.right; x++) {
			const targetX = x - target.left;
			const total = xTable.totals[targetX]! * yTotal;
			const targetOffset = (y * output.width + x) * 4;
			if (total === 0) {
				outputData[targetOffset] = 0;
				outputData[targetOffset + 1] = 0;
				outputData[targetOffset + 2] = 0;
				outputData[targetOffset + 3] = 0;
				continue;
			}

			let r = 0;
			let g = 0;
			let b = 0;
			for (let yTap = 0; yTap < yLength; yTap++) {
				const weight = yTable.weights[yOffset + yTap]!;
				if (weight === 0) continue;
				const row = rows[yTap];
				if (!row) continue;
				const rowOffset = targetX * 3;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
			}

			outputData[targetOffset] = clampByte(r / total);
			outputData[targetOffset + 1] = clampByte(g / total);
			outputData[targetOffset + 2] = clampByte(b / total);
			outputData[targetOffset + 3] = 255;
		}

		evictLanczosRows(rowCache, lastUse, targetY, yTable);
	}
}

function resizeLanczos3WithAlpha(
	source: ImageData,
	output: ImageData,
	target: TargetBounds,
	xTable: ContributionTable,
	yTable: ContributionTable
) {
	const outputData = output.data;
	const rowCache = new Map<number, Float64Array>();
	const lastUse = lastUseTable(source.height, target.height, yTable);

	for (let y = target.top; y < target.bottom; y++) {
		const targetY = y - target.top;
		const yOffset = yTable.offsets[targetY]!;
		const yLength = yTable.lengths[targetY]!;
		const yTotal = yTable.totals[targetY]!;
		const rows = lanczosRowsForTargetY(source, target.width, xTable, yTable, targetY, rowCache, true);

		for (let x = target.left; x < target.right; x++) {
			const targetX = x - target.left;
			const total = xTable.totals[targetX]! * yTotal;
			const targetOffset = (y * output.width + x) * 4;
			if (total === 0) {
				outputData[targetOffset] = 0;
				outputData[targetOffset + 1] = 0;
				outputData[targetOffset + 2] = 0;
				outputData[targetOffset + 3] = 0;
				continue;
			}

			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;
			for (let yTap = 0; yTap < yLength; yTap++) {
				const weight = yTable.weights[yOffset + yTap]!;
				if (weight === 0) continue;
				const row = rows[yTap];
				if (!row) continue;
				const rowOffset = targetX * 4;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
				a += row[rowOffset + 3]! * weight;
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

		evictLanczosRows(rowCache, lastUse, targetY, yTable);
	}
}

function lastUseTable(sourceHeight: number, targetHeight: number, yTable: ContributionTable) {
	const lastUse = new Int32Array(sourceHeight);
	lastUse.fill(-1);
	for (let targetY = 0; targetY < targetHeight; targetY++) {
		const offset = yTable.offsets[targetY]!;
		const length = yTable.lengths[targetY]!;
		for (let yTap = 0; yTap < length; yTap++) {
			if (yTable.weights[offset + yTap] === 0) continue;
			lastUse[yTable.indices[offset + yTap]!] = targetY;
		}
	}
	return lastUse;
}

function lanczosRowsForTargetY(
	source: ImageData,
	targetWidth: number,
	xTable: ContributionTable,
	yTable: ContributionTable,
	targetY: number,
	rowCache: Map<number, Float64Array>,
	withAlpha: boolean
) {
	const yOffset = yTable.offsets[targetY]!;
	const yLength = yTable.lengths[targetY]!;
	const rows: Array<Float64Array | undefined> = [];
	for (let yTap = 0; yTap < yLength; yTap++) {
		if (yTable.weights[yOffset + yTap] === 0) {
			rows[yTap] = undefined;
			continue;
		}
		const sourceY = yTable.indices[yOffset + yTap]!;
		let row = rowCache.get(sourceY);
		if (!row) {
			row = withAlpha
				? lanczosAlphaRow(source, sourceY, targetWidth, xTable)
				: lanczosOpaqueRow(source, sourceY, targetWidth, xTable);
			rowCache.set(sourceY, row);
		}
		rows[yTap] = row;
	}
	return rows;
}

function evictLanczosRows(
	rowCache: Map<number, Float64Array>,
	lastUse: Int32Array,
	targetY: number,
	yTable: ContributionTable
) {
	const offset = yTable.offsets[targetY]!;
	const length = yTable.lengths[targetY]!;
	for (let yTap = 0; yTap < length; yTap++) {
		if (yTable.weights[offset + yTap] === 0) continue;
		const sourceY = yTable.indices[offset + yTap]!;
		if (lastUse[sourceY] === targetY) rowCache.delete(sourceY);
	}
}

function lanczosOpaqueRow(
	source: ImageData,
	sourceY: number,
	targetWidth: number,
	xTable: ContributionTable
) {
	const sourceData = source.data;
	const row = new Float64Array(targetWidth * 3);
	const sourceRowOffset = sourceY * source.width * 4;
	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const offset = xTable.offsets[targetX]!;
		const length = xTable.lengths[targetX]!;
		let r = 0;
		let g = 0;
		let b = 0;
		for (let xTap = 0; xTap < length; xTap++) {
			const weight = xTable.weights[offset + xTap]!;
			if (weight === 0) continue;
			const sourceOffset = sourceRowOffset + xTable.indices[offset + xTap]! * 4;
			r += sourceData[sourceOffset]! * weight;
			g += sourceData[sourceOffset + 1]! * weight;
			b += sourceData[sourceOffset + 2]! * weight;
		}
		const rowOffset = targetX * 3;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
	}
	return row;
}

function lanczosAlphaRow(
	source: ImageData,
	sourceY: number,
	targetWidth: number,
	xTable: ContributionTable
) {
	const sourceData = source.data;
	const row = new Float64Array(targetWidth * 4);
	const sourceRowOffset = sourceY * source.width * 4;
	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const offset = xTable.offsets[targetX]!;
		const length = xTable.lengths[targetX]!;
		let r = 0;
		let g = 0;
		let b = 0;
		let a = 0;
		for (let xTap = 0; xTap < length; xTap++) {
			const weight = xTable.weights[offset + xTap]!;
			if (weight === 0) continue;
			const sourceOffset = sourceRowOffset + xTable.indices[offset + xTap]! * 4;
			const alpha = sourceData[sourceOffset + 3]! / 255;
			r += sourceData[sourceOffset]! * alpha * weight;
			g += sourceData[sourceOffset + 1]! * alpha * weight;
			b += sourceData[sourceOffset + 2]! * alpha * weight;
			a += sourceData[sourceOffset + 3]! * weight;
		}
		const rowOffset = targetX * 4;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
		row[rowOffset + 3] = a;
	}
	return row;
}

function contributionTable(count: number, sourceStart: number, sourceStep: number, sourceLimit: number) {
	const offsets = new Int32Array(count);
	const lengths = new Int32Array(count);
	const totals = new ${totalArray}(count);
	const indices: number[] = [];
	const weights: number[] = [];
	for (let target = 0; target < count; target++) {
		const sourcePosition = sourceStart + (target + 0.5) * sourceStep - 0.5;
		const scaleClamped = LANCZOS_WINDOW_MODE === 'pica-scale-aware' ? Math.min(1, 1 / sourceStep) : 1;
		const sourceWindow = LANCZOS_FILTER_WINDOW / scaleClamped;
		let first: number;
		let last: number;
		if (LANCZOS_WINDOW_MODE === 'current-fixed') {
			const floor = Math.floor(sourcePosition);
			first = floor - Math.ceil(LANCZOS_FILTER_WINDOW) + 1;
			last = floor + Math.ceil(LANCZOS_FILTER_WINDOW);
		} else {
			first = Math.max(0, Math.floor(sourcePosition - sourceWindow));
			last = Math.min(sourceLimit - 1, Math.ceil(sourcePosition + sourceWindow));
		}

		offsets[target] = weights.length;
		let total = 0;
		for (let sourceIndex = first; sourceIndex <= last; sourceIndex++) {
			const clampedIndex = Math.min(sourceLimit - 1, Math.max(0, sourceIndex));
			const distance = (sourcePosition - sourceIndex) * scaleClamped;
			const rawWeight = lanczosFilterWeight(distance);
			const weight = ${weightValue};
			indices.push(clampedIndex);
			weights.push(weight);
			total += weight;
		}
		lengths[target] = weights.length - offsets[target]!;
		totals[target] = total;
	}
	return {
		indices: new Int32Array(indices),
		offsets,
		lengths,
		weights: new ${weightArray}(weights),
		totals
	} satisfies ContributionTable;
}

function lanczosFilterWeight(value: number) {
	const absolute = Math.abs(value);
	if (LANCZOS_FILTER === 'box') return absolute < 0.5 ? 1 : 0;
	if (LANCZOS_FILTER === 'hamming') {
		if (absolute >= 1) return 0;
		if (absolute < 1.1920929e-7) return 1;
		const x = absolute * Math.PI;
		return (Math.sin(x) / x) * (0.54 + 0.46 * Math.cos(x));
	}
	if (LANCZOS_FILTER === 'mks2013') {
		if (absolute >= 2.5) return 0;
		if (absolute >= 1.5) return -0.125 * (absolute - 2.5) * (absolute - 2.5);
		if (absolute >= 0.5) return 0.25 * (4 * absolute * absolute - 11 * absolute + 7);
		return 1.0625 - 1.75 * absolute * absolute;
	}
	return lanczos(absolute, LANCZOS_FILTER === 'lanczos2' ? 2 : 3);
}
`;
}

main();
