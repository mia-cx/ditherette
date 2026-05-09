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
		description: 'Committed resize implementation',
		apply: (source) => source
	},
	{
		id: 'inline-bilinear',
		description: 'Inline bilinear offset math; remove getPixel tuple and accumulator object',
		apply: applyInlineBilinear
	},
	{
		id: 'bilinear-local-opaque-fast-path',
		description: 'Inline bilinear with a four-sample opaque fast path',
		apply: applyBilinearOpaqueFastPath
	},
	{
		id: 'inline-area',
		description: 'Inline area offset math; remove getPixel tuple and accumulator object',
		apply: applyInlineArea
	},
	{
		id: 'inline-bilinear-area',
		description: 'Combine inline bilinear and inline area scalar sampling',
		apply: (source) => applyInlineArea(applyInlineBilinear(source))
	},
	{
		id: 'inline-area-bilinear-opaque',
		description: 'Combine inline area with bilinear local opaque fast path',
		apply: (source) => applyInlineArea(applyBilinearOpaqueFastPath(source))
	}
];

function main() {
	const { outRoot, benchArgs, checkCorrectness } = parseArgs(process.argv.slice(2));
	const originalWorkingTree = readFileSync(RESIZE_PATH, 'utf8');
	const baseline = gitShow(`HEAD:${RESIZE_PATH}`);
	const startedAt = new Date().toISOString();
	const root = outRoot ?? join('benchmark-results', `sampler-study-${safeTimestamp(startedAt)}`);
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
	let checkCorrectness = true;
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
		if (arg === '--skip-correctness') {
			checkCorrectness = false;
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
	console.log(`Usage: node scripts/sampler-optimization-study.mjs [study options] -- [benchmark options]

Runs exact-output sampler variants from the committed resize.ts baseline:
  1. baseline
  2. inline-bilinear
  3. bilinear-local-opaque-fast-path
  4. inline-area
  5. inline-bilinear-area
  6. inline-area-bilinear-opaque

Study options:
  --out DIR              Root output directory (default: benchmark-results/sampler-study-<timestamp>)
  --skip-correctness     Do not run resize.spec.ts for each variant
  --check-correctness    Run resize.spec.ts for each variant (default)

Benchmark options default to:
  --scale --resize --iterations 5 --warmups 2 --quiet true

Examples:
  pnpm bench:sampler-study
  pnpm bench:sampler-study -- --scale --resize --iterations 10 --warmups 3 --quiet true
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

function applyInlineBilinear(source) {
	return replaceOnce(source, BILINEAR_FUNCTION, INLINE_BILINEAR_FUNCTION);
}

function applyBilinearOpaqueFastPath(source) {
	return replaceOnce(source, BILINEAR_FUNCTION, BILINEAR_OPAQUE_FAST_PATH_FUNCTION);
}

function applyInlineArea(source) {
	return replaceOnce(source, AREA_FUNCTION, INLINE_AREA_FUNCTION);
}

function replaceOnce(source, oldText, newText) {
	if (!source.includes(oldText)) throw new Error('Could not find expected source block.');
	return source.replace(oldText, newText);
}

const BILINEAR_FUNCTION = `function sampleBilinear(source: ImageData, x: number, y: number) {
	const x0 = Math.floor(x);
	const y0 = Math.floor(y);
	const tx = x - x0;
	const ty = y - y0;
	const accumulator = { r: 0, g: 0, b: 0, a: 0, total: 0 };
	addWeightedSample(
		accumulator,
		getPixel(source.data, source.width, source.height, x0, y0),
		(1 - tx) * (1 - ty)
	);
	addWeightedSample(
		accumulator,
		getPixel(source.data, source.width, source.height, x0 + 1, y0),
		tx * (1 - ty)
	);
	addWeightedSample(
		accumulator,
		getPixel(source.data, source.width, source.height, x0, y0 + 1),
		(1 - tx) * ty
	);
	addWeightedSample(
		accumulator,
		getPixel(source.data, source.width, source.height, x0 + 1, y0 + 1),
		tx * ty
	);
	return finishWeightedSample(accumulator);
}
`;

const INLINE_BILINEAR_FUNCTION = `function sampleBilinear(source: ImageData, x: number, y: number) {
	const data = source.data;
	const x0 = Math.floor(x);
	const y0 = Math.floor(y);
	const x1 = x0 + 1;
	const y1 = y0 + 1;
	const clampedX0 = Math.min(source.width - 1, Math.max(0, x0));
	const clampedX1 = Math.min(source.width - 1, Math.max(0, x1));
	const clampedY0 = Math.min(source.height - 1, Math.max(0, y0));
	const clampedY1 = Math.min(source.height - 1, Math.max(0, y1));
	const tx = x - x0;
	const ty = y - y0;
	const weight00 = (1 - tx) * (1 - ty);
	const weight10 = tx * (1 - ty);
	const weight01 = (1 - tx) * ty;
	const weight11 = tx * ty;
	let r = 0;
	let g = 0;
	let b = 0;
	let a = 0;
	let total = 0;

	let offset = (clampedY0 * source.width + clampedX0) * 4;
	let alpha = data[offset + 3]! / 255;
	r += data[offset]! * alpha * weight00;
	g += data[offset + 1]! * alpha * weight00;
	b += data[offset + 2]! * alpha * weight00;
	a += data[offset + 3]! * weight00;
	total += weight00;

	offset = (clampedY0 * source.width + clampedX1) * 4;
	alpha = data[offset + 3]! / 255;
	r += data[offset]! * alpha * weight10;
	g += data[offset + 1]! * alpha * weight10;
	b += data[offset + 2]! * alpha * weight10;
	a += data[offset + 3]! * weight10;
	total += weight10;

	offset = (clampedY1 * source.width + clampedX0) * 4;
	alpha = data[offset + 3]! / 255;
	r += data[offset]! * alpha * weight01;
	g += data[offset + 1]! * alpha * weight01;
	b += data[offset + 2]! * alpha * weight01;
	a += data[offset + 3]! * weight01;
	total += weight01;

	offset = (clampedY1 * source.width + clampedX1) * 4;
	alpha = data[offset + 3]! / 255;
	r += data[offset]! * alpha * weight11;
	g += data[offset + 1]! * alpha * weight11;
	b += data[offset + 2]! * alpha * weight11;
	a += data[offset + 3]! * weight11;
	total += weight11;

	if (total === 0) return [0, 0, 0, 0] as const;
	return unpremultiplySample(r / total, g / total, b / total, a / total);
}
`;

const BILINEAR_OPAQUE_FAST_PATH_FUNCTION = `function sampleBilinear(source: ImageData, x: number, y: number) {
	const data = source.data;
	const x0 = Math.floor(x);
	const y0 = Math.floor(y);
	const x1 = x0 + 1;
	const y1 = y0 + 1;
	const clampedX0 = Math.min(source.width - 1, Math.max(0, x0));
	const clampedX1 = Math.min(source.width - 1, Math.max(0, x1));
	const clampedY0 = Math.min(source.height - 1, Math.max(0, y0));
	const clampedY1 = Math.min(source.height - 1, Math.max(0, y1));
	const tx = x - x0;
	const ty = y - y0;
	const weight00 = (1 - tx) * (1 - ty);
	const weight10 = tx * (1 - ty);
	const weight01 = (1 - tx) * ty;
	const weight11 = tx * ty;
	const offset00 = (clampedY0 * source.width + clampedX0) * 4;
	const offset10 = (clampedY0 * source.width + clampedX1) * 4;
	const offset01 = (clampedY1 * source.width + clampedX0) * 4;
	const offset11 = (clampedY1 * source.width + clampedX1) * 4;
	const total = weight00 + weight10 + weight01 + weight11;

	if (
		data[offset00 + 3] === 255 &&
		data[offset10 + 3] === 255 &&
		data[offset01 + 3] === 255 &&
		data[offset11 + 3] === 255
	) {
		return [
			clampByte(
				(data[offset00]! * weight00 +
					data[offset10]! * weight10 +
					data[offset01]! * weight01 +
					data[offset11]! * weight11) /
					total
			),
			clampByte(
				(data[offset00 + 1]! * weight00 +
					data[offset10 + 1]! * weight10 +
					data[offset01 + 1]! * weight01 +
					data[offset11 + 1]! * weight11) /
					total
			),
			clampByte(
				(data[offset00 + 2]! * weight00 +
					data[offset10 + 2]! * weight10 +
					data[offset01 + 2]! * weight01 +
					data[offset11 + 2]! * weight11) /
					total
			),
			255
		] as const;
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

	if (total === 0) return [0, 0, 0, 0] as const;
	return unpremultiplySample(r / total, g / total, b / total, a / total);
}
`;

const AREA_FUNCTION = `function sampleArea(source: ImageData, x: number, y: number, scaleX: number, scaleY: number) {
	if (scaleX < 1 && scaleY < 1) return sampleBilinear(source, x, y);
	const left = Math.floor(x - scaleX / 2);
	const right = Math.ceil(x + scaleX / 2);
	const top = Math.floor(y - scaleY / 2);
	const bottom = Math.ceil(y + scaleY / 2);
	const accumulator = { r: 0, g: 0, b: 0, a: 0, total: 0 };
	for (let yy = top; yy <= bottom; yy++) {
		for (let xx = left; xx <= right; xx++) {
			addWeightedSample(accumulator, getPixel(source.data, source.width, source.height, xx, yy), 1);
		}
	}
	return finishWeightedSample(accumulator);
}
`;

const INLINE_AREA_FUNCTION = `function sampleArea(source: ImageData, x: number, y: number, scaleX: number, scaleY: number) {
	if (scaleX < 1 && scaleY < 1) return sampleBilinear(source, x, y);
	const data = source.data;
	const left = Math.floor(x - scaleX / 2);
	const right = Math.ceil(x + scaleX / 2);
	const top = Math.floor(y - scaleY / 2);
	const bottom = Math.ceil(y + scaleY / 2);
	let r = 0;
	let g = 0;
	let b = 0;
	let a = 0;
	let total = 0;
	for (let yy = top; yy <= bottom; yy++) {
		const clampedY = Math.min(source.height - 1, Math.max(0, yy));
		for (let xx = left; xx <= right; xx++) {
			const clampedX = Math.min(source.width - 1, Math.max(0, xx));
			const offset = (clampedY * source.width + clampedX) * 4;
			const alpha = data[offset + 3]! / 255;
			r += data[offset]! * alpha;
			g += data[offset + 1]! * alpha;
			b += data[offset + 2]! * alpha;
			a += data[offset + 3]!;
			total += 1;
		}
	}
	if (total === 0) return [0, 0, 0, 0] as const;
	return unpremultiplySample(r / total, g / total, b / total, a / total);
}
`;

main();
