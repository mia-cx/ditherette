#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const args = process.argv.slice(2);
if (args.includes('--help') || args.includes('-h')) {
	console.log(`Usage: node scripts/plot-tiling-sweep.mjs INPUT.json [--output OUTPUT.svg]

Plots scalar/default/best tiling speedups from a tiling sweep JSON. The
near-identity region from 0.95x through 1.05x is shaded so resize cliffs around
1x stand out instead of disappearing into the wider scale range.`);
	process.exit(0);
}

const input = args.find((arg) => !arg.startsWith('--'));
if (!input) throw new Error('missing input sweep JSON');
const output = optionValue('--output') ?? defaultOutputPath(input);

const sweep = JSON.parse(fs.readFileSync(input, 'utf8'));
const rows = summarizeSweep(sweep);
const svg = renderSvg(sweep, rows);
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, svg);
console.log(`wrote ${output}`);

function optionValue(name) {
	const index = args.indexOf(name);
	return index === -1 ? undefined : args[index + 1];
}

function defaultOutputPath(inputPath) {
	const parsed = path.parse(inputPath);
	return path.join(parsed.dir, `${parsed.name}.svg`);
}

function summarizeSweep(sweep) {
	const byScale = new Map();
	for (const result of sweep.cases) {
		const scale = parseFloat(result.scale.replace(/x$/, ''));
		const bucket = byScale.get(result.scale) ?? { label: result.scale, scale, cases: [] };
		bucket.cases.push(result);
		byScale.set(result.scale, bucket);
	}

	return [...byScale.values()]
		.map((bucket) => {
			const scalar = bucket.cases.find((result) => result.mode === 'scalar');
			const defaultTiling = bucket.cases.find((result) => result.mode === 'default-tiling');
			const best = bucket.cases
				.filter((result) => result.mode !== 'scalar')
				.reduce((current, result) =>
					result.statsNs.median < current.statsNs.median ? result : current
				);
			const scalarMedian = scalar.statsNs.median;
			return {
				...bucket,
				outputPixels: scalar.output.width * scalar.output.height,
				scalarMs: scalarMedian / 1_000_000,
				defaultSpeedup: scalarMedian / defaultTiling.statsNs.median,
				bestSpeedup: scalarMedian / best.statsNs.median,
				bestBands: best.resolved?.bandCount ?? 1,
				bestWorkers: best.resolved?.workerCount ?? 1,
				bestMode: best.mode,
			};
		})
		.sort((a, b) => a.scale - b.scale);
}

function renderSvg(sweep, rows) {
	const width = 1180;
	const height = 720;
	const margin = { top: 76, right: 180, bottom: 92, left: 78 };
	const plotWidth = width - margin.left - margin.right;
	const plotHeight = height - margin.top - margin.bottom;
	const xMin = Math.min(...rows.map((row) => row.scale));
	const xMax = Math.max(...rows.map((row) => row.scale));
	const yMax = Math.max(1.1, ...rows.map((row) => row.bestSpeedup)) * 1.08;
	const x = (value) => margin.left + ((value - xMin) / (xMax - xMin)) * plotWidth;
	const y = (value) => margin.top + plotHeight - (value / yMax) * plotHeight;
	const line = (selector) =>
		rows.map((row, index) => `${index === 0 ? 'M' : 'L'}${x(row.scale).toFixed(1)} ${y(selector(row)).toFixed(1)}`).join(' ');
	const xTicks = [0.0625, 0.1, 0.25, 0.5, 0.8, 0.95, 1, 1.05, 1.2, 1.5, 2].filter(
		(tick) => tick >= xMin && tick <= xMax
	);
	const yTicks = Array.from({ length: Math.floor(yMax) + 1 }, (_, index) => index).filter(Boolean);
	const nearIdentityLeft = x(0.95);
	const nearIdentityRight = x(1.05);

	return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
<style>
	text { font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; fill: #172033; }
	.axis { stroke: #293241; stroke-width: 1.4; }
	.grid { stroke: #d8dee9; stroke-width: 1; }
	.best { fill: none; stroke: #0f766e; stroke-width: 3; }
	.default { fill: none; stroke: #7c3aed; stroke-width: 2.4; stroke-dasharray: 7 5; }
	.scalar { stroke: #c2410c; stroke-width: 1.8; stroke-dasharray: 5 5; }
	.note { fill: #586174; font-size: 13px; }
	.label { fill: #293241; font-size: 13px; }
	.title { fill: #111827; font-size: 24px; font-weight: 700; }
	.subtitle { fill: #586174; font-size: 14px; }
</style>
<rect width="100%" height="100%" fill="#fbfaf7"/>
<text class="title" x="${margin.left}" y="34">${escapeXml(sweep.target)} tiling sweep</text>
<text class="subtitle" x="${margin.left}" y="56">${escapeXml(sweep.fixture)} · ${sweep.source.width}×${sweep.source.height} · ${rows.length} scales · ${sweep.iterations} iterations</text>
<rect x="${nearIdentityLeft.toFixed(1)}" y="${margin.top}" width="${(nearIdentityRight - nearIdentityLeft).toFixed(1)}" height="${plotHeight}" fill="#fef3c7" opacity="0.65"/>
<text class="note" x="${x(0.952).toFixed(1)}" y="${margin.top + 18}">near identity</text>
${yTicks.map((tick) => `<line class="grid" x1="${margin.left}" x2="${width - margin.right}" y1="${y(tick).toFixed(1)}" y2="${y(tick).toFixed(1)}"/><text class="label" x="${margin.left - 12}" y="${y(tick).toFixed(1)}" text-anchor="end" dominant-baseline="middle">${tick}×</text>`).join('\n')}
${xTicks.map((tick) => `<line class="grid" x1="${x(tick).toFixed(1)}" x2="${x(tick).toFixed(1)}" y1="${margin.top}" y2="${height - margin.bottom}"/><text class="label" x="${x(tick).toFixed(1)}" y="${height - margin.bottom + 24}" text-anchor="middle">${formatScale(tick)}</text>`).join('\n')}
<line class="axis" x1="${margin.left}" x2="${width - margin.right}" y1="${height - margin.bottom}" y2="${height - margin.bottom}"/>
<line class="axis" x1="${margin.left}" x2="${margin.left}" y1="${margin.top}" y2="${height - margin.bottom}"/>
<line class="scalar" x1="${margin.left}" x2="${width - margin.right}" y1="${y(1).toFixed(1)}" y2="${y(1).toFixed(1)}"/>
<path class="default" d="${line((row) => row.defaultSpeedup)}"/>
<path class="best" d="${line((row) => row.bestSpeedup)}"/>
${rows.map((row) => `<circle cx="${x(row.scale).toFixed(1)}" cy="${y(row.bestSpeedup).toFixed(1)}" r="${2 + row.bestBands}" fill="${bandColor(row.bestBands)}" opacity="0.82"><title>${row.label}: best ${row.bestSpeedup.toFixed(2)}×, ${row.bestBands} bands / ${row.bestWorkers} workers</title></circle>`).join('\n')}
<text class="label" x="${margin.left + plotWidth / 2}" y="${height - 30}" text-anchor="middle">resize scale</text>
<text class="label" transform="translate(24 ${margin.top + plotHeight / 2}) rotate(-90)" text-anchor="middle">speedup vs scalar</text>
${legend(width - margin.right + 28, margin.top + 8)}
</svg>`;
}

function legend(x, y) {
	return `<g>
<line class="best" x1="${x}" x2="${x + 34}" y1="${y}" y2="${y}"/><text class="label" x="${x + 44}" y="${y}" dominant-baseline="middle">best tiling</text>
<line class="default" x1="${x}" x2="${x + 34}" y1="${y + 28}" y2="${y + 28}"/><text class="label" x="${x + 44}" y="${y + 28}" dominant-baseline="middle">default tiling</text>
<line class="scalar" x1="${x}" x2="${x + 34}" y1="${y + 56}" y2="${y + 56}"/><text class="label" x="${x + 44}" y="${y + 56}" dominant-baseline="middle">scalar parity</text>
<circle cx="${x + 10}" cy="${y + 96}" r="6" fill="${bandColor(1)}"/><text class="label" x="${x + 28}" y="${y + 96}" dominant-baseline="middle">1 band</text>
<circle cx="${x + 10}" cy="${y + 122}" r="6" fill="${bandColor(4)}"/><text class="label" x="${x + 28}" y="${y + 122}" dominant-baseline="middle">4 bands</text>
<circle cx="${x + 10}" cy="${y + 148}" r="6" fill="${bandColor(8)}"/><text class="label" x="${x + 28}" y="${y + 148}" dominant-baseline="middle">8 bands</text>
</g>`;
}

function bandColor(bands) {
	if (bands <= 1) return '#64748b';
	if (bands <= 2) return '#f59e0b';
	if (bands <= 4) return '#0ea5e9';
	return '#14b8a6';
}

function formatScale(scale) {
	return `${Number(scale.toFixed(4))}x`;
}

function escapeXml(value) {
	return String(value)
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;');
}
