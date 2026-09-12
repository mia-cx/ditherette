import assert from 'node:assert/strict';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';

const root = fileURLToPath(new URL('../', import.meta.url));
const source = await readFile(new URL('./benchmark-wasm-resize.mjs', import.meta.url), 'utf8');
const helpersStart = source.indexOf('\nfunction sweepRunConfigs(');
assert.ok(helpersStart > 0);
const host = vm.createContext({ root, path, readFile, stat, defaultManifestPath: 'scripts/ditherette-wasm-bench.toml' });
vm.runInContext(source.slice(helpersStart), host);
const plain = (value) => JSON.parse(JSON.stringify(value));
const manifest = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));

function browser(pixels) {
	const events = [];
	const context = vm.createContext({
		init: async () => {},
		wasm: new Proxy({}, { get() { throw new Error('No benchmark workload may execute'); } }),
		performance: { now: () => 0 },
		fetch: async () => ({ ok: true, blob: async () => ({}) }),
		createImageBitmap: async () => ({ width: 1, height: 1 }),
		document: { createElement: () => ({ getContext: () => ({ drawImage() {}, getImageData: () => ({ data: pixels }) }) }) },
		console: { debug(message) { if (message.startsWith('bench-event ')) events.push(JSON.parse(message.slice(12))); } },
		navigator: { userAgent: 'configuration-test' }
	});
	const module = host.benchmarkBrowserModule();
	assert.ok(module.startsWith("import init, * as wasm from '/pkg/ditherette_wasm.js';\n"));
	vm.runInContext(module.slice(module.indexOf('\n') + 1), context);
	return { context, events };
}

function options(subjects, periodicScalarRemeasurement = false) {
	return { subjects, periodicScalarRemeasurement, scales: [{ x: 1, y: 1 }], threadCounts: [1, 2], rowBandHeights: [32, 64] };
}

for (const filter of ['nearest', 'area', 'bilinear', 'bicubic', 'lanczos2', 'lanczos3', 'trilinear']) {
	test(`native ${filter} alias selects its actual profile`, async () => {
		const profile = manifest.scripts[`bench:resize:${filter}`].split(' -- run ')[1];
		const native = host.parseTomlSubset(await readFile(new URL('../crates/ditherette-bench/ditherette-bench.toml', import.meta.url), 'utf8'));
		assert.equal(profile, filter);
		assert.equal(native.profiles[profile].domain, 'resize');
	});
}

test('color alias resolves the color profile', async () => {
	const args = manifest.scripts['bench:color:wasm'].split(' -- wasm-resize')[1].trim().split(/\s+/).filter(Boolean);
	const resolved = await host.resolveOptions(args);
	assert.equal(resolved.profile, 'color');
	assert.equal(resolved.domain, 'color');
	assert.ok(resolved.subjects.every((id) => id.startsWith('wasm:color:')));
});

test('all scalar execution policies run once outside worker grids', () => {
	const ids = ['wasm:resize:nearest:scalar', 'wasm:resize:bicubic:catmull-rom', 'wasm:resize:bicubic:catmull-rom-scale-aware', 'wasm:resize:lanczos3:fixed', 'wasm:resize:lanczos3:scale-aware', 'wasm:color:oklab-f32:scalar'];
	const runs = plain(host.sweepRunConfigs(options(ids)));
	assert.equal(runs.length, 1);
	assert.deepEqual(runs[0].subjects, ids);
	for (const subject of ids) assert.ok(host.isScalarResult({ subject }));
	assert.equal(host.requiresThreadedWasm(['wasm:resize:lanczos3:pooled_direct_per_band_plan']), true);
});

test('periodic controls follow each candidate with canonical subject IDs', () => {
	const scalar = 'wasm:resize:lanczos3:fixed';
	const candidates = ['wasm:resize:lanczos3:pooled_direct', 'wasm:resize:lanczos3:pooled_direct_per_band_plan'];
	const unrelated = 'wasm:resize:nearest:scalar';
	const runs = plain(host.sweepRunConfigs(options([scalar, unrelated, ...candidates], true)));
	assert.equal(runs.length, 5);
	assert.deepEqual(runs[0].subjects, [unrelated]);
	const { context } = browser(new Uint8Array(4));
	for (const run of runs.slice(1)) {
		assert.equal(run.periodicScalarRemeasurement, true);
		const schedule = context.scheduledSubjects(run.subjects.map((id) => host.subjectConfig(id)), true);
		assert.deepEqual(Array.from(schedule, (subject) => subject.id), [scalar, candidates[0], scalar, candidates[1], scalar]);
	}
	const disabled = plain(host.sweepRunConfigs(options([scalar, ...candidates])));
	assert.deepEqual(disabled[0].subjects, [scalar]);
	assert.ok(disabled.slice(1).every((run) => !run.periodicScalarRemeasurement && !run.subjects.includes(scalar)));
});

test('color periodic controls match the conversion target', () => {
	const ids = ['wasm:color:oklab-f32:scalar', 'wasm:color:cielab-f32:scalar', 'wasm:color:oklab-f32:pooled_direct'];
	const runs = plain(host.sweepRunConfigs(options(ids, true)));
	assert.deepEqual(runs[0].subjects, [ids[1]]);
	const { context } = browser(new Uint8Array(4));
	const schedule = context.scheduledSubjects(runs[1].subjects.map((id) => host.subjectConfig(id)), true);
	assert.deepEqual(Array.from(schedule, (subject) => subject.id), [ids[0], ids[2], ids[0]]);
});

test('comparisons use the most recent preceding scalar control', () => {
	const scalar = 'wasm:resize:lanczos3:fixed';
	const candidate = 'wasm:resize:lanczos3:pooled_direct';
	const result = (subject, median) => ({ id: 'fixture-decoded-rgba', subject, filter: 'lanczos3', fixture: { name: 'fixture.png', fingerprint: 'rgba8:1x1:fnv1a32:12345678' }, statsNs: { median, mean: median, stdev: 0, p5: median, p95: median }, iterationsPerSample: 1 });
	const comparisons = host.candidateComparisons([result(scalar, 100), result(candidate, 50), result(scalar, 200), result(candidate, 50)]);
	assert.deepEqual(Array.from(comparisons, (value) => value.scalarMedianNs), [100, 200]);
});

test('comparisons require matching decoded content and resize support', () => {
	const scalar = 'wasm:resize:lanczos3:fixed';
	const scaleAware = 'wasm:resize:lanczos3:scale-aware';
	const candidate = 'wasm:resize:lanczos3:pooled_direct';
	const result = (subject, median, fingerprint) => ({
		id: 'fixture-decoded-rgba',
		subject,
		filter: 'lanczos3',
		fixture: { name: 'fixture.png', fingerprint },
		statsNs: { median, mean: median, stdev: 0, p5: median, p95: median },
		iterationsPerSample: 1
	});
	const first = 'rgba8:1x1:fnv1a32:12345678';
	const second = 'rgba8:1x1:fnv1a32:87654321';
	const comparisons = host.candidateComparisons([
		result(scalar, 100, first),
		result(candidate, 50, second),
		result(scaleAware, 999, first),
		result(candidate, 50, first)
	]);
	assert.deepEqual(Array.from(comparisons, (value) => value.scalarMedianNs), [100]);
});

test('decoded content identity reaches fixture metadata and both lanes without measurement', async () => {
	async function decode(pixels) {
		const { context, events } = browser(Uint8Array.from(pixels));
		const config = { fixtures: [{ name: 'same.png', url: '/same.png' }], subjects: [], domain: 'resize', scales: [{ x: 1, y: 1 }], lanes: ['decoded-rgba', 'browser-decode-rgba'], threadedWasm: false };
		const decoded = await context.decodeFixture(config.fixtures[0]);
		const cases = context.makeCases(decoded, config);
		const result = await context.runWasmBench(config);
		assert.match(decoded.fingerprint, /^rgba8:1x1:fnv1a32:[0-9a-f]{8}$/);
		assert.equal(events[0].fixtures[0].fingerprint, decoded.fingerprint);
		assert.equal(result.fixtures[0].fingerprint, decoded.fingerprint);
		assert.ok(cases.every((value) => value.fixture.fingerprint === decoded.fingerprint));
		assert.equal(result.results.length, 0);
		return decoded.fingerprint;
	}
	const first = await decode([0, 0, 0, 255]);
	assert.equal(await decode([0, 0, 0, 255]), first);
	assert.notEqual(await decode([1, 0, 0, 255]), first);
});

test('browser profiles resolve only supported Wasm subjects', async () => {
	const config = host.parseTomlSubset(
		await readFile(new URL('./ditherette-wasm-bench.toml', import.meta.url), 'utf8')
	);
	for (const [name, profile] of Object.entries(config.profiles)) {
		if (!profile.subjects) continue;
		const resolved = await host.resolveOptions(['run', name]);
		for (const subject of resolved.subjects) {
			assert.doesNotThrow(() => host.subjectConfig(subject), name);
		}
	}
});
