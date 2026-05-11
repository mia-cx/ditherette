#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { readdir, readFile, rm } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const criterionDir = path.join(root, 'crates/ditherette-wasm/target/criterion');
const significantNumberFormatter = new Intl.NumberFormat('en-US', {
	maximumSignificantDigits: 4,
	minimumSignificantDigits: 4,
	useGrouping: false
});
const scaleOrder = new Map(
	['0.95x', '0.75x', '0.5x', '0.25x', '0.125x'].map((scale, index) => [scale, index])
);
const variantOrder = new Map(
	[
		'baseline',
		'precomputed-offsets',
		'fast-paths',
		'word-copy',
		'into-reused-output',
		'production',
		'production-into'
	].map((variant, index) => [variant, index])
);

await rm(criterionDir, { recursive: true, force: true });

const cargo = spawnSync(
	'cargo',
	[
		'bench',
		'--manifest-path',
		'crates/ditherette-wasm/Cargo.toml',
		'--bench',
		'resize_nearest',
		'--',
		'--noplot',
		...process.argv.slice(2)
	],
	{ cwd: root, stdio: 'inherit' }
);

if (cargo.status !== 0) {
	process.exit(cargo.status ?? 1);
}

const summaries = await criterionSummaries();
if (summaries.length === 0) {
	console.warn('\nNo Criterion sample data found; percentile summary skipped.');
	process.exit(0);
}

console.log('\nPercentiles from Criterion samples');
console.log(table(summaryRows(summaries)));

async function criterionSummaries() {
	const sampleFiles = await findSampleFiles(criterionDir);
	const summaries = [];

	for (const sampleFile of sampleFiles) {
		const sampleDir = path.dirname(sampleFile);
		const [sample, benchmark] = await Promise.all([
			readJson(sampleFile),
			readJson(path.join(sampleDir, 'benchmark.json'))
		]);
		const timings = sample.times.map((time, index) => time / sample.iters[index]);
		const stats = summarizeNanoseconds(timings);

		summaries.push({
			caseId: benchmark.group_id,
			variant: benchmark.function_id,
			stats,
			samples: timings.length
		});
	}

	return summaries.sort(compareSummaries);
}

async function findSampleFiles(directory) {
	let entries;
	try {
		entries = await readdir(directory, { withFileTypes: true });
	} catch (error) {
		if (error?.code === 'ENOENT') return [];
		throw error;
	}

	const files = [];
	for (const entry of entries) {
		const entryPath = path.join(directory, entry.name);
		if (entry.isDirectory()) {
			files.push(...(await findSampleFiles(entryPath)));
		} else if (entry.name === 'sample.json' && path.basename(directory) === 'new') {
			files.push(entryPath);
		}
	}
	return files;
}

async function readJson(filePath) {
	return JSON.parse(await readFile(filePath, 'utf8'));
}

function summarizeNanoseconds(timings) {
	const sorted = [...timings].sort((left, right) => left - right);
	const total = sorted.reduce((sum, value) => sum + value, 0);

	return {
		mean: total / sorted.length,
		min: sorted[0],
		p50: percentile(sorted, 50),
		p75: percentile(sorted, 75),
		p95: percentile(sorted, 95),
		p99: percentile(sorted, 99),
		max: sorted[sorted.length - 1]
	};
}

function percentile(sortedTimings, percentileValue) {
	if (sortedTimings.length === 1) return sortedTimings[0];

	const rank = (percentileValue / 100) * (sortedTimings.length - 1);
	const lowerIndex = Math.floor(rank);
	const upperIndex = Math.ceil(rank);
	const weight = rank - lowerIndex;

	return sortedTimings[lowerIndex] * (1 - weight) + sortedTimings[upperIndex] * weight;
}

function summaryRows(summaries) {
	const baselineByCase = new Map(
		summaries
			.filter((summary) => summary.variant === 'baseline')
			.map((summary) => [summary.caseId, summary.stats.mean])
	);

	return summaries.map((summary) => {
		const baselineMean = baselineByCase.get(summary.caseId);
		return {
			case: shortCaseId(summary.caseId),
			variant: summary.variant,
			mean: formatDuration(summary.stats.mean),
			p50: formatDuration(summary.stats.p50),
			p75: formatDuration(summary.stats.p75),
			p95: formatDuration(summary.stats.p95),
			p99: formatDuration(summary.stats.p99),
			max: formatDuration(summary.stats.max),
			speedup: formatSpeedup(summary.stats.mean, baselineMean),
			samples: summary.samples
		};
	});
}

function shortCaseId(caseId) {
	return caseId.replace('resize_nearest/celeste_rgba/', '');
}

function formatDuration(nanoseconds) {
	if (nanoseconds === 0) return '0ns';
	if (!Number.isFinite(nanoseconds)) return String(nanoseconds);

	const absolute = Math.abs(nanoseconds);
	if (absolute >= 1_000_000) return `${formatSignificant(nanoseconds / 1_000_000)}ms`;
	if (absolute >= 1_000) return `${formatSignificant(nanoseconds / 1_000)}µs`;
	return `${formatSignificant(nanoseconds)}ns`;
}

function formatSpeedup(nanoseconds, baselineNanoseconds) {
	if (!baselineNanoseconds) return '—';
	return `${formatSignificant(baselineNanoseconds / nanoseconds)}×`;
}

function formatSignificant(value) {
	return significantNumberFormatter.format(value);
}

function compareSummaries(left, right) {
	return (
		scaleIndex(left.caseId) - scaleIndex(right.caseId) ||
		left.caseId.localeCompare(right.caseId, undefined, { numeric: true }) ||
		variantIndex(left.variant) - variantIndex(right.variant) ||
		left.variant.localeCompare(right.variant)
	);
}

function scaleIndex(caseId) {
	return scaleOrder.get(caseId.split('/').at(-1)?.split('-').at(0)) ?? Number.MAX_SAFE_INTEGER;
}

function variantIndex(variant) {
	return variantOrder.get(variant) ?? Number.MAX_SAFE_INTEGER;
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
