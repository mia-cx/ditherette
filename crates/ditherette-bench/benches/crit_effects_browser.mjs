//! Focused scalar-Chromium comparison for recipe-v2 effects.
//!
//! Build two package `dist` directories first, then run this script under
//! `ditherette-bench-lease --quiet --`. JPEG and PNG decoding happen before timing.

import { createHash } from 'node:crypto';
import { readFile, stat } from 'node:fs/promises';
import { extname, join, resolve, sep } from 'node:path';
import { chromium } from 'playwright';

const [acceptedArgument, candidateArgument, samplesArgument = '20'] = process.argv.slice(2);
const samples = Number(samplesArgument);
if (!acceptedArgument || !candidateArgument || !Number.isInteger(samples) || samples < 20) {
	throw new Error(
		'Usage: crit_effects_browser.mjs ACCEPTED_DIST CANDIDATE_DIST [SAMPLES_AT_LEAST_20]'
	);
}
if (process.env.DITHERETTE_BENCH_QUIET !== '1') {
	throw new Error('Run through ditherette-bench-lease --quiet after the host is quiet.');
}

const root = resolve(new URL('../../..', import.meta.url).pathname);
const roots = {
	accepted: resolve(acceptedArgument),
	candidate: resolve(candidateArgument)
};
const fixtures = {
	// A local photo can stand in for the committed fixture, e.g. a 24 MP camera JPEG.
	large: resolve(process.env.DITHERETTE_BENCH_PHOTO ?? resolve(root, 'benchmark-fixtures/Picking_at_thread.jpg')),
	small: resolve(root, 'benchmark-fixtures/Celeste_Insta_selfie.png')
};
for (const path of [...Object.values(roots), ...Object.values(fixtures)]) await stat(path);
const artifactDescriptions = Object.fromEntries(
	await Promise.all(
		Object.entries(roots).map(async ([role, path]) => [
			role,
			{
				path,
				sha256: createHash('sha256')
					.update(await readFile(join(path, 'wasm/scalar/ditherette_wasm_bg.wasm')))
					.digest('hex')
			}
		])
	)
);

const browser = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--js-flags=--expose-gc'] });
try {
	const page = await browser.newPage();
	await page.route('https://ditherette.test/**', async (route) => {
		const url = new URL(route.request().url());
		if (url.pathname === '/') {
			await route.fulfill({
				contentType: 'text/html',
				body: '<!doctype html><title>Ditherette effects benchmark</title>'
			});
			return;
		}
		const fixture = url.pathname.match(/^\/fixture\/(large|small)$/)?.[1];
		if (fixture) return fulfill(route, fixtures[fixture]);
		const asset = url.pathname.match(/^\/(accepted|candidate)\/(.+)$/);
		if (!asset) return route.fulfill({ status: 404, body: 'Not found' });
		const base = roots[asset[1]];
		const path = resolve(base, asset[2]);
		if (path !== base && !path.startsWith(`${base}${sep}`))
			return route.fulfill({ status: 404, body: 'Not found' });
		return fulfill(route, path);
	});
	await page.goto('https://ditherette.test/');
	const measured = await page.evaluate(
		async ({ samples }) => {
			const palette = [
				{ kind: 'color', rgb: [0, 0, 0] },
				{ kind: 'color', rgb: [255, 255, 255] },
				{ kind: 'color', rgb: [237, 28, 36] },
				{ kind: 'color', rgb: [255, 127, 39] },
				{ kind: 'color', rgb: [249, 221, 59] },
				{ kind: 'color', rgb: [14, 185, 104] },
				{ kind: 'color', rgb: [40, 80, 158] },
				{ kind: 'color', rgb: [120, 12, 153] }
			];
			const hue = {
				effect: 'hue-saturation',
				enabled: true,
				hue: 25,
				saturation: 0.3,
				lightness: 0.05
			};
			const curves = {
				effect: 'curves',
				enabled: true,
				channel: 'rgb',
				points: [
					[0, 0],
					[0.25, 0.2],
					[0.75, 0.85],
					[1, 1]
				]
			};
			const cases = [
				{ name: 'large-none', fixture: 'large', effects: [] },
				{ name: 'large-curves', fixture: 'large', effects: [curves] },
				{ name: 'large-hue-saturation', fixture: 'large', effects: [hue] },
				{ name: '800x800-hue-saturation', fixture: 'small', effects: [hue] }
			];
			const artifacts = {};
			for (const role of ['accepted', 'candidate']) {
				artifacts[role] = {
					api: await import(`/${role}/index.js`),
					wasm: await WebAssembly.compileStreaming(
						fetch(`/${role}/wasm/scalar/ditherette_wasm_bg.wasm`)
					)
				};
			}
			const sources = {
				large: await decode('/fixture/large'),
				small: await decode('/fixture/small')
			};
			const results = {};
			for (const benchmark of cases) {
				const roleSamples = { accepted: [], candidate: [] };
				const signatures = {};
				for (const role of ['accepted', 'candidate']) await run(role, benchmark);
				for (let index = 0; index < samples; index++) {
					const order = index % 2 === 0 ? ['accepted', 'candidate'] : ['candidate', 'accepted'];
					for (const role of order) {
						const sample = await run(role, benchmark);
						roleSamples[role].push(sample.elapsed);
						if (signatures[role] && signatures[role] !== sample.signature)
							throw new Error(`${benchmark.name} ${role} output changed between samples`);
						signatures[role] = sample.signature;
					}
				}
				if (signatures.accepted !== signatures.candidate)
					throw new Error(`${benchmark.name} differs between accepted and candidate`);
				results[benchmark.name] = { samples: roleSamples, signature: signatures.accepted };
			}
			return { userAgent: navigator.userAgent, sources: dimensions(sources), results };

			async function run(role, benchmark) {
				const artifact = artifacts[role];
				const processor = await artifact.api.createDitherette({
					wasm: artifact.wasm,
					threads: 'disabled'
				});
				try {
					const start = performance.now();
					const output = processor.process({
						source: sources[benchmark.fixture],
						palette,
						recipe: {
							version: 2,
							effects: benchmark.effects,
							output: { width: 480, height: 320, resize: { algorithm: 'area' } },
							alpha: { mode: 'preserve', threshold: 127.5 },
							match: 'oklab-euclidean',
							dither: { family: 'none' }
						}
					});
					const elapsed = performance.now() - start;
					return { elapsed, signature: await digest(output) };
				} finally {
					processor.dispose();
					// Released Wasm memories linger until collection; fresh processors per sample exhaust the tab.
					gc();
				}
			}

			async function decode(url) {
				const bitmap = await createImageBitmap(await (await fetch(url)).blob());
				try {
					const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
					const context = canvas.getContext('2d', { willReadFrequently: true });
					context.drawImage(bitmap, 0, 0);
					return {
						width: bitmap.width,
						height: bitmap.height,
						data: new Uint8Array(context.getImageData(0, 0, bitmap.width, bitmap.height).data)
					};
				} finally {
					bitmap.close();
				}
			}

			async function digest(output) {
				const bytes = new Uint8Array(
					output.indices.length + output.palette.rgba.length + Uint32Array.BYTES_PER_ELEMENT * 3
				);
				bytes.set(output.indices);
				bytes.set(output.palette.rgba, output.indices.length);
				new Uint32Array(bytes.buffer, bytes.length - 12).set([
					output.width,
					output.height,
					output.palette.transparentIndex ?? 0xffff_ffff
				]);
				const hash = await crypto.subtle.digest('SHA-256', bytes);
				return [...new Uint8Array(hash)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
			}

			function dimensions(values) {
				return Object.fromEntries(
					Object.entries(values).map(([name, source]) => [name, [source.width, source.height]])
				);
			}
		},
		{ samples }
	);

	const report = {
		schema: 1,
		samples,
		artifacts: artifactDescriptions,
		...measured,
		results: Object.fromEntries(
			Object.entries(measured.results).map(([name, result]) => {
				const accepted = summary(result.samples.accepted);
				const candidate = summary(result.samples.candidate);
				return [
					name,
					{
						accepted,
						candidate,
						ratio: candidate.median / accepted.median,
						speedup: accepted.median / candidate.median,
						signature: result.signature,
						samples: result.samples
					}
				];
			})
		)
	};
	console.log(JSON.stringify(report, null, 2));
} finally {
	await browser.close();
}

async function fulfill(route, path) {
	await route.fulfill({ contentType: contentType(path), body: await readFile(path) });
}

function contentType(path) {
	return (
		{
			'.js': 'text/javascript',
			'.wasm': 'application/wasm',
			'.jpg': 'image/jpeg',
			'.png': 'image/png'
		}[extname(path)] ?? 'application/octet-stream'
	);
}

function summary(values) {
	const sorted = [...values].sort((left, right) => left - right);
	return {
		median: quantile(sorted, 0.5),
		q1: quantile(sorted, 0.25),
		q3: quantile(sorted, 0.75),
		iqr: quantile(sorted, 0.75) - quantile(sorted, 0.25)
	};
}

function quantile(sorted, fraction) {
	const position = (sorted.length - 1) * fraction;
	const lower = Math.floor(position);
	const share = position - lower;
	return sorted[lower] + (sorted[Math.min(lower + 1, sorted.length - 1)] - sorted[lower]) * share;
}
