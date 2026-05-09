#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const RESIZE_PATH = 'src/lib/processing/resize.ts';
const DEFAULT_BENCH_ARGS = [
	'--scale',
	'--resize',
	'--iterations',
	'5',
	'--warmups',
	'2',
	'--quiet',
	'true'
];

const VARIANTS = [
	{
		id: 'baseline',
		description: 'Committed separable Lanczos implementation',
		apply: (source) => source
	},
	{
		id: 'float32-row-cache',
		description: 'Store cached horizontal Lanczos rows as Float32Array instead of Float64Array',
		apply: applyFloat32Rows
	},
	{
		id: 'transparency-cache',
		description: 'Cache per-ImageData transparency scan results in a WeakMap',
		apply: applyTransparencyCache
	},
	{
		id: 'unrolled-horizontal-taps',
		description: 'Manually unroll the six horizontal Lanczos taps',
		apply: applyUnrolledHorizontalTaps
	}
];

function main() {
	const { outRoot, benchArgs, checkCorrectness } = parseArgs(process.argv.slice(2));
	const originalWorkingTree = readFileSync(RESIZE_PATH, 'utf8');
	const baseline = gitShow(`HEAD:${RESIZE_PATH}`);
	const startedAt = new Date().toISOString();
	const root = outRoot ?? join('benchmark-results', `lanczos-study-${safeTimestamp(startedAt)}`);
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
		checkCorrectness
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
	console.log(`Usage: node scripts/lanczos-optimization-study.mjs [study options] -- [benchmark options]

Runs four variants from the committed resize.ts baseline:
  1. baseline
  2. float32-row-cache
  3. transparency-cache
  4. unrolled-horizontal-taps

Study options:
  --out DIR              Root output directory (default: benchmark-results/lanczos-study-<timestamp>)
  --check-correctness    Run resize.spec.ts for each variant before benchmarking

Benchmark options default to:
  --scale --resize --iterations 5 --warmups 2 --quiet true

Examples:
  pnpm bench:lanczos-study
  pnpm bench:lanczos-study -- --scale --resize --iterations 10 --warmups 3 --quiet true
  pnpm bench:lanczos-study -- --out benchmark-results/lanczos-study-full --profile exhaustive --quiet true
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
		{
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
		join(root, `${variantId}-correctness.log`),
		`${result.stdout ?? ''}\n${result.stderr ?? ''}`
	);
	if (!correctness.passed) {
		console.warn(
			`${variantId} correctness check failed; continuing so perf can still be measured.`
		);
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
	for (const result of data.results) {
		const resize = parseResizeMode(result.case.id);
		byResize[resize] ??= { count: 0, resizeMs: 0, totalMs: 0 };
		byResize[resize].count++;
		byResize[resize].resizeMs += result.stages.resize.meanMs;
		byResize[resize].totalMs += result.stages.total.meanMs;
	}
	return {
		profile: data.profile,
		runAt: data.runAt,
		resultCount: data.results.length,
		resizeMs: sum(data.results, (result) => result.stages.resize.meanMs),
		totalMs: sum(data.results, (result) => result.stages.total.meanMs),
		byResize
	};
}

function parseResizeMode(caseId) {
	return caseId.split('-')[2] ?? 'unknown';
}

function sum(values, getter) {
	return values.reduce((total, value) => total + getter(value), 0);
}

function writeStudySummary(root, startedAt, benchArgs, runs) {
	const serializableRuns = runs.map(({ id, description, outDir, correctness, summary }) => ({
		id,
		description,
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
	const modes = ['nearest', 'bilinear', 'lanczos3', 'area'];
	const rows = [
		[
			'variant',
			'correctness',
			'results',
			'resizeMs',
			'totalMs',
			...modes.map((mode) => `${mode}ResizeMs`)
		]
	];
	for (const run of runs) {
		rows.push([
			run.id,
			run.correctness.skipped ? 'skipped' : run.correctness.passed ? 'passed' : 'failed',
			run.summary.resultCount,
			run.summary.resizeMs,
			run.summary.totalMs,
			...modes.map((mode) => run.summary.byResize[mode]?.resizeMs ?? 0)
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

function applyFloat32Rows(source) {
	return source
		.replaceAll('Map<number, Float64Array>', 'Map<number, Float32Array>')
		.replaceAll('Array<Float64Array | undefined>', 'Array<Float32Array | undefined>')
		.replaceAll('new Float64Array(targetWidth * 3)', 'new Float32Array(targetWidth * 3)')
		.replaceAll('new Float64Array(targetWidth * 4)', 'new Float32Array(targetWidth * 4)');
}

function applyTransparencyCache(source) {
	return source
		.replace(
			'const LANCZOS_RADIUS = 3;\nconst LANCZOS_TAPS = LANCZOS_RADIUS * 2;\n',
			'const LANCZOS_RADIUS = 3;\nconst LANCZOS_TAPS = LANCZOS_RADIUS * 2;\nconst TRANSPARENCY_CACHE = new WeakMap<ImageData, boolean>();\n'
		)
		.replace('if (hasTransparentPixels(source.data)) {', 'if (hasTransparentPixels(source)) {')
		.replace(
			`function hasTransparentPixels(data: Uint8ClampedArray) {
	for (let offset = 3; offset < data.length; offset += 4) {
		if (data[offset] !== 255) return true;
	}
	return false;
}
`,
			`function hasTransparentPixels(source: ImageData) {
	const cached = TRANSPARENCY_CACHE.get(source);
	if (cached !== undefined) return cached;
	const data = source.data;
	for (let offset = 3; offset < data.length; offset += 4) {
		if (data[offset] !== 255) {
			TRANSPARENCY_CACHE.set(source, true);
			return true;
		}
	}
	TRANSPARENCY_CACHE.set(source, false);
	return false;
}
`
		);
}

function applyUnrolledHorizontalTaps(source) {
	return source
		.replace(OPAQUE_ROW_LOOP, UNROLLED_OPAQUE_ROW_LOOP)
		.replace(ALPHA_ROW_LOOP, UNROLLED_ALPHA_ROW_LOOP);
}

const OPAQUE_ROW_LOOP = `	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const xBase = targetX * LANCZOS_TAPS;
		let r = 0;
		let g = 0;
		let b = 0;
		for (let xTap = 0; xTap < LANCZOS_TAPS; xTap++) {
			const weight = xTable.weights[xBase + xTap]!;
			if (weight === 0) continue;
			const offset = sourceRowOffset + xTable.indices[xBase + xTap]! * 4;
			r += sourceData[offset]! * weight;
			g += sourceData[offset + 1]! * weight;
			b += sourceData[offset + 2]! * weight;
		}
		const rowOffset = targetX * 3;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
	}
`;

const UNROLLED_OPAQUE_ROW_LOOP = `	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const xBase = targetX * LANCZOS_TAPS;
		let offset = sourceRowOffset + xTable.indices[xBase]! * 4;
		const weight0 = xTable.weights[xBase]!;
		let r = sourceData[offset]! * weight0;
		let g = sourceData[offset + 1]! * weight0;
		let b = sourceData[offset + 2]! * weight0;

		offset = sourceRowOffset + xTable.indices[xBase + 1]! * 4;
		const weight1 = xTable.weights[xBase + 1]!;
		r += sourceData[offset]! * weight1;
		g += sourceData[offset + 1]! * weight1;
		b += sourceData[offset + 2]! * weight1;

		offset = sourceRowOffset + xTable.indices[xBase + 2]! * 4;
		const weight2 = xTable.weights[xBase + 2]!;
		r += sourceData[offset]! * weight2;
		g += sourceData[offset + 1]! * weight2;
		b += sourceData[offset + 2]! * weight2;

		offset = sourceRowOffset + xTable.indices[xBase + 3]! * 4;
		const weight3 = xTable.weights[xBase + 3]!;
		r += sourceData[offset]! * weight3;
		g += sourceData[offset + 1]! * weight3;
		b += sourceData[offset + 2]! * weight3;

		offset = sourceRowOffset + xTable.indices[xBase + 4]! * 4;
		const weight4 = xTable.weights[xBase + 4]!;
		r += sourceData[offset]! * weight4;
		g += sourceData[offset + 1]! * weight4;
		b += sourceData[offset + 2]! * weight4;

		offset = sourceRowOffset + xTable.indices[xBase + 5]! * 4;
		const weight5 = xTable.weights[xBase + 5]!;
		r += sourceData[offset]! * weight5;
		g += sourceData[offset + 1]! * weight5;
		b += sourceData[offset + 2]! * weight5;

		const rowOffset = targetX * 3;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
	}
`;

const ALPHA_ROW_LOOP = `	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const xBase = targetX * LANCZOS_TAPS;
		let r = 0;
		let g = 0;
		let b = 0;
		let a = 0;
		for (let xTap = 0; xTap < LANCZOS_TAPS; xTap++) {
			const weight = xTable.weights[xBase + xTap]!;
			if (weight === 0) continue;
			const offset = sourceRowOffset + xTable.indices[xBase + xTap]! * 4;
			const alpha = sourceData[offset + 3]! / 255;
			r += sourceData[offset]! * alpha * weight;
			g += sourceData[offset + 1]! * alpha * weight;
			b += sourceData[offset + 2]! * alpha * weight;
			a += sourceData[offset + 3]! * weight;
		}
		const rowOffset = targetX * 4;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
		row[rowOffset + 3] = a;
	}
`;

const UNROLLED_ALPHA_ROW_LOOP = `	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const xBase = targetX * LANCZOS_TAPS;
		let offset = sourceRowOffset + xTable.indices[xBase]! * 4;
		let alpha = sourceData[offset + 3]! / 255;
		const weight0 = xTable.weights[xBase]!;
		let r = sourceData[offset]! * alpha * weight0;
		let g = sourceData[offset + 1]! * alpha * weight0;
		let b = sourceData[offset + 2]! * alpha * weight0;
		let a = sourceData[offset + 3]! * weight0;

		offset = sourceRowOffset + xTable.indices[xBase + 1]! * 4;
		alpha = sourceData[offset + 3]! / 255;
		const weight1 = xTable.weights[xBase + 1]!;
		r += sourceData[offset]! * alpha * weight1;
		g += sourceData[offset + 1]! * alpha * weight1;
		b += sourceData[offset + 2]! * alpha * weight1;
		a += sourceData[offset + 3]! * weight1;

		offset = sourceRowOffset + xTable.indices[xBase + 2]! * 4;
		alpha = sourceData[offset + 3]! / 255;
		const weight2 = xTable.weights[xBase + 2]!;
		r += sourceData[offset]! * alpha * weight2;
		g += sourceData[offset + 1]! * alpha * weight2;
		b += sourceData[offset + 2]! * alpha * weight2;
		a += sourceData[offset + 3]! * weight2;

		offset = sourceRowOffset + xTable.indices[xBase + 3]! * 4;
		alpha = sourceData[offset + 3]! / 255;
		const weight3 = xTable.weights[xBase + 3]!;
		r += sourceData[offset]! * alpha * weight3;
		g += sourceData[offset + 1]! * alpha * weight3;
		b += sourceData[offset + 2]! * alpha * weight3;
		a += sourceData[offset + 3]! * weight3;

		offset = sourceRowOffset + xTable.indices[xBase + 4]! * 4;
		alpha = sourceData[offset + 3]! / 255;
		const weight4 = xTable.weights[xBase + 4]!;
		r += sourceData[offset]! * alpha * weight4;
		g += sourceData[offset + 1]! * alpha * weight4;
		b += sourceData[offset + 2]! * alpha * weight4;
		a += sourceData[offset + 3]! * weight4;

		offset = sourceRowOffset + xTable.indices[xBase + 5]! * 4;
		alpha = sourceData[offset + 3]! / 255;
		const weight5 = xTable.weights[xBase + 5]!;
		r += sourceData[offset]! * alpha * weight5;
		g += sourceData[offset + 1]! * alpha * weight5;
		b += sourceData[offset + 2]! * alpha * weight5;
		a += sourceData[offset + 3]! * weight5;

		const rowOffset = targetX * 4;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
		row[rowOffset + 3] = a;
	}
`;

main();
