#!/usr/bin/env node
// Preparation only. This command cannot launch a browser or collect timings.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tilingCases } from '../packages/ditherette/tests/tiling-cases.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
const [imagePath, outputPath] = process.argv.slice(2);
if (!imagePath || !outputPath)
	throw Error('Usage: node scripts/plan-tiling-benchmarks.mjs SOURCE.rgba PLAN.json');
if (execFileSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).trim())
	throw Error('Commit the tested source before preparing its benchmark plan.');
const revision = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
const source = { width: 2600, height: 4168, data: new Uint8Array() };
const rgba = await readFile(imagePath);
if (rgba.byteLength !== source.width * source.height * 4)
	throw Error('Expected the full 2600×4168 Celeste RGBA source at 1×.');
const text = await readFile(new URL('../src/lib/palette/wplace.ts', import.meta.url), 'utf8');
const palette = [...text.matchAll(/\['(?!#)[^']+', '(#[0-9A-F]{6})'\]/g)].map(([, hex]) => ({
	kind: 'color',
	rgb: [1, 3, 5].map((offset) => parseInt(hex.slice(offset, offset + 2), 16))
}));
palette.push({ kind: 'transparent' });
if (palette.length !== 64) throw Error('Expected the complete Wplace palette.');
const configurations = [
	{ id: 'scalar-whole', engine: 'cpu', threads: 'disabled', tiling: false },
	{ id: 'threads-whole', engine: 'cpu', threads: 'required', tiling: false },
	{ id: 'threads-auto', engine: 'cpu', threads: 'required', tiling: true },
	...[8, 16, 32, 64, 128, 256].flatMap((height) =>
		[1, 2, 4, 8].map((workers) => ({
			id: `threads-${height}-${workers}`,
			engine: 'cpu',
			threads: 'required',
			tiling: { height, workers }
		}))
	),
	{ id: 'gpu-whole', engine: 'gpu', fallback: 'error', tiling: false },
	{ id: 'gpu-auto', engine: 'gpu', fallback: 'error', tiling: true },
	...[8, 16, 32, 64, 128, 256].map((height) => ({
		id: `gpu-${height}`,
		engine: 'gpu',
		fallback: 'error',
		tiling: { height }
	}))
];
const cases = tilingCases(source, palette).map(({ id, method, schedule, cpuOnly }) => ({
	id,
	method,
	schedule,
	cpuOnly
}));
const pairs = cases.flatMap((item) =>
	configurations
		.filter((config) => {
			if (config.id === 'scalar-whole' || config.id === 'gpu-whole') return false;
			if (item.cpuOnly && config.engine === 'gpu') return false;
			// Global analysis and CPU diffusion have no independent row-worker schedule.
			if ((item.schedule === 'global' || item.schedule === 'dependent') && config.engine === 'cpu')
				return config.id === 'threads-whole';
			// Larger dependency batches alias the existing 64-dispatch safety cap.
			if (
				item.schedule === 'dependent' &&
				config.engine === 'gpu' &&
				typeof config.tiling === 'object' &&
				config.tiling.height > 64
			)
				return false;
			return true;
		})
		.map((config) => ({
			case: item.id,
			baseline: config.engine === 'gpu' ? 'gpu-whole' : 'scalar-whole',
			candidate: config.id,
			order: [
				['baseline', 'candidate'],
				['candidate', 'baseline'],
				['baseline', 'candidate'],
				['candidate', 'baseline'],
				['baseline', 'candidate'],
				['candidate', 'baseline']
			]
		}))
);
const artifacts = await Promise.all(
	['scalar', 'threads', 'gpu'].map(async (variant) => ({
		variant,
		sha256: digest(
			await readFile(
				new URL(
					`../packages/ditherette/dist/wasm/${variant}/ditherette_wasm_bg.wasm`,
					import.meta.url
				)
			)
		)
	}))
);
const plan = {
	schema: 1,
	state: 'HELD_FOR_MIA',
	sourceRevision: revision,
	source: {
		path: resolve(imagePath),
		width: source.width,
		height: source.height,
		sha256: digest(rgba)
	},
	palette,
	artifacts,
	configurations,
	cases,
	pairs,
	protocol: {
		authorization: 'Do not collect timings until Mia explicitly says to start.',
		preparation:
			'Freeze the complete installed package, harness and browser identities before the quiet phase; rebuild outside measurement after any source change.',
		coordination:
			'Drain owned builds/tests/agents; acquire the shared benchmark lease and execution lock; reject external CUDA processes including resident Remora. Never stop unrelated workloads.',
		contamination:
			'Bounded five-consecutive-sample host preflight, then external-process/GPU checks around every individual trial. Retain contaminated rows but exclude them from recommendations; no infinite retry.',
		measurement:
			'Run sequentially in a real Chromium processing worker. Decode/load and backend initialization stay outside process timers. Report cold compute and identical-request output-cache hits separately; do not select tile policy from cache hits.',
		correctness:
			'Compare output hash against the same-engine whole-image control outside timing. GPU strict policy must report GPU, and required-thread runs must verify the actual pool. Keep complete original source and alpha.',
		selection:
			'Screen per kernel, image-size class, matching cost, placement and scan order. Validate finalists with fresh alternating pairs; keep the simpler whole/auto policy when evidence is inconclusive. CPU diffusion and global recipe reductions stay continuous.'
	}
};
await writeFile(outputPath, `${JSON.stringify(plan, null, 2)}\n`);
console.log(
	JSON.stringify({
		state: plan.state,
		revision,
		cases: cases.length,
		configurations: configurations.length,
		candidatePairs: pairs.length,
		path: resolve(outputPath)
	})
);
