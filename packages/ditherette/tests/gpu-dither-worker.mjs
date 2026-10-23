import { createDitherette, createGpuDither } from '../dist/index.js';

self.onmessage = async (event) => {
	if (event.origin && event.origin !== self.location.origin) return;
	try {
		const source = event.data.source ?? {
			width: 53,
			height: 37,
			data: Uint8Array.from({ length: 53 * 37 * 4 }, (_, i) => {
				const pixel = i >>> 2;
				const x = pixel % 53;
				const y = Math.floor(pixel / 53);
				if (i % 4 === 3) return [0, 127, 128, 255][pixel % 4];
				if (x === 28) return 5;
				return (64 + x + Math.floor(y / 3) + [0, 25, 10][i % 4]) & 255;
			})
		};
		const palette = event.data.palette ?? [
			{ kind: 'color', rgb: [0, 0, 0] },
			{ kind: 'color', rgb: [255, 255, 255] },
			{ kind: 'color', rgb: [90, 120, 100] },
			{ kind: 'color', rgb: [160, 90, 120] },
			{ kind: 'color', rgb: [30, 70, 210] },
			{ kind: 'color', rgb: [90, 120, 100] },
			{ kind: 'transparent' }
		];
		if (event.data.tiling) {
			// Proves explicit schedules reach both real scalar and shared-memory artifacts.
			const input = {
				version: 1,
				source,
				palette,
				matching: 'oklab-euclidean',
				alpha: { mode: 'preserve', threshold: 128 },
				onProgress() {}
			};
			const calls = [
				(cpu) =>
					cpu.resize({
						version: 1,
						source,
						output: {
							width: 31,
							height: 23,
							resize: { algorithm: 'lanczos3', anchor: 'center', support: 'scale-aware' }
						},
						onProgress() {}
					}).data,
				(cpu) =>
					cpu.ditherAndQuantize({
						...input,
						dither: {
							family: 'separable',
							perturb: {
								field: { algorithm: 'bayer', size: '16' },
								space: 'oklab',
								strength: 0.3,
								placement: { mode: 'everywhere' }
							}
						}
					}).indices,
				(cpu) =>
					cpu.ditherAndQuantize({
						...input,
						dither: { family: 'yliluoma', size: '4', placement: { mode: 'everywhere' } }
					}).indices
			];
			const baseline = await createDitherette({ tiling: 'disabled' });
			let expected;
			try {
				expected = calls.map((call) => call(baseline).join(','));
			} finally {
				baseline.dispose();
			}
			let count = 0;
			for (const threads of ['disabled', 'required'])
				for (const tiling of ['auto', 'disabled', { height: 3, workers: 2 }]) {
					const cpu = await createDitherette({ threads, tiling });
					try {
						for (const [i, call] of calls.entries()) {
							if (call(cpu).join(',') !== expected[i])
								throw Error(`Schedule changed output: ${threads}/${JSON.stringify(tiling)}/${i}`);
							count++;
						}
					} finally {
						cpu.dispose();
					}
				}
			self.postMessage({ calls: count });
			return;
		}
		if (event.data.resize) {
			const whole = await createGpuDither({ fallback: 'error', tiling: 'disabled' });
			const tiled = await createGpuDither({ fallback: 'error', tiling: { height: 3 } });
			const cpu = await createDitherette();
			let calls = 0,
				maximumByteDifference = 0;
			try {
				for (const anchor of ['top-left', 'center', 'bottom-right']) {
					const filters = [
						{ algorithm: 'nearest', anchor },
						{ algorithm: 'area' },
						{ algorithm: 'bilinear', anchor },
						{ algorithm: 'trilinear', anchor },
						...['bicubic', 'lanczos2', 'lanczos3'].flatMap((algorithm) =>
							['fixed', 'scale-aware'].map((support) => ({ algorithm, anchor, support }))
						)
					];
					for (const resize of filters) {
						const input = { version: 1, source, output: { width: 29, height: 23, resize } };
						const reference = cpu.resize(input);
						const a = await whole.resize(input);
						const b = await tiled.resize(input);
						if (whole.execution.backend !== 'gpu' || tiled.execution.backend !== 'gpu')
							throw Error('GPU resize required');
						for (let i = 0; i < a.data.length; i++) {
							if (a.data[i] !== b.data[i]) throw Error(`Tile seam at byte ${i}`);
							maximumByteDifference = Math.max(
								maximumByteDifference,
								Math.abs(a.data[i] - reference.data[i])
							);
						}
						calls += 2;
					}
				}
				let callbackRejected = false;
				const input = {
					version: 1,
					source,
					output: { width: 29, height: 23, resize: { algorithm: 'trilinear', anchor: 'center' } }
				};
				try {
					await tiled.resize({
						...input,
						onProgress({ stage }) {
							if (stage === 'resize') throw Error('callback failure');
						}
					});
				} catch (error) {
					callbackRejected = error.code === 'callback';
				}
				if (!callbackRejected) throw Error('callback failure was ignored');
				await tiled.resize(input);
				if (tiled.execution.backend !== 'gpu') throw Error('callback retired the device');
				self.postMessage({ calls, maximumByteDifference, callbackRecovered: true });
			} finally {
				whole.dispose();
				tiled.dispose();
				cpu.dispose();
			}
			return;
		}
		const matching = [
			'srgb-euclidean',
			'srgb-compuphase',
			'srgb-rec601',
			'srgb-rec709',
			'linear-rgb-euclidean',
			'oklab-euclidean',
			'oklch-euclidean',
			'oklch-circular-hue',
			'oklch-hue-arc',
			'cielab-euclidean',
			'cielab-ciede2000',
			'cielch-euclidean',
			'cielch-circular-hue',
			'cielch-hue-arc',
			'ycbcr-euclidean'
		];
		const working = (matching) =>
			matching.startsWith('linear') ? 'linear-rgb' : matching.split('-')[0];
		const fields = [
			...['2', '4', '8', '16'].map((size) => ({ algorithm: 'bayer', size })),
			...['3x1', '4x1', '4x2', '5x3'].map((tile) => ({ algorithm: 'ordered', tile })),
			{ algorithm: 'blue-noise' },
			{ algorithm: 'random', seed: 0xfffffff1 }
		];
		const area = { mode: 'smooth-area-v1', sampleSize: 20, contrastThreshold: 0.12 };
		const cases = [];
		for (const match of matching) {
			cases.push({ matching: match, dither: { family: 'none' } });
			for (const [index, field] of fields.entries())
				for (const placement of [{ mode: 'everywhere' }, area]) {
					cases.push({
						matching: match,
						dither: {
							family: 'separable',
							perturb: {
								field,
								space: working(match),
								strength: [0.3, 96 / 63.75, 4][index % 3],
								placement
							}
						}
					});
				}
		}
		for (const kernel of [
			'floyd-steinberg',
			'sierra',
			'sierra-lite',
			'atkinson',
			'jarvis-judice-ninke',
			'stucki',
			'burkes',
			'two-row-sierra',
			'fan',
			'shiau-fan',
			'shiau-fan-2',
			'simple-2d'
		]) {
			for (const feedback of ['matching', 'srgb-bytes'])
				for (const serpentine of [false, true]) {
					cases.push({
						matching: 'oklab-euclidean',
						dither: {
							family: 'diffusion',
							kernel,
							feedback,
							serpentine,
							strength: 1,
							placement: area
						}
					});
				}
		}
		const full = [];
		if (event.data.full)
			for (const sampleSize of [20, 40, 60, 100])
				for (const contrastThreshold of [0.06, 0.12]) {
					for (const strength of [0.3, 1]) {
						const placement = { mode: 'smooth-area-v1', sampleSize, contrastThreshold };
						full.push({
							matching: 'oklab-euclidean',
							dither: {
								family: 'separable',
								perturb: {
									field: { algorithm: 'bayer', size: '16' },
									space: 'oklab',
									strength,
									placement
								}
							}
						});
						full.push({
							matching: 'oklab-euclidean',
							dither: {
								family: 'diffusion',
								kernel: 'sierra',
								feedback: 'matching',
								serpentine: true,
								strength,
								placement
							}
						});
					}
				}
		const selected = event.data.full
			? full
			: event.data.photo
				? [
						cases[0],
						cases.find(
							(item) =>
								item.matching === 'oklab-euclidean' &&
								item.dither.perturb?.field.size === '16' &&
								item.dither.perturb.placement.mode === 'smooth-area-v1'
						),
						cases.find((item) => item.dither.kernel === 'sierra')
					]
				: event.data.fallback
					? [cases[0], cases[8], cases.at(-1)]
					: cases;
		const requests = selected.map((item, index) => ({
			...item,
			version: 1,
			source,
			palette,
			alpha: [
				{ mode: 'preserve', threshold: 127.9 },
				{ mode: 'premultiplied' },
				{ mode: 'matte', rgb: [7, 200, 90] }
			][index % 3]
		}));
		let cpu = await createDitherette();
		const expected = requests.map((request) => cpu.ditherAndQuantize(request));
		cpu.dispose();
		cpu = undefined;
		const gpu = await createGpuDither();
		let differences = 0;
		let maximumByteDifference = 0;
		const backends = {};
		const records = [];
		let analyses = 0,
			reuses = 0;
		for (let n = 0; n < requests.length; n++) {
			const { dither, ...quantize } = requests[n];
			const actual =
				dither.family === 'none'
					? await gpu.quantize(quantize)
					: await gpu.ditherAndQuantize(requests[n]);
			const reference = expected[n];
			if (
				JSON.stringify(actual.palette) !== JSON.stringify(reference.palette) ||
				JSON.stringify(actual.warnings) !== JSON.stringify(reference.warnings)
			)
				throw Error('Palette metadata differs');
			let changed = 0;
			for (let i = 0; i < actual.indices.length; i++) {
				const a = actual.indices[i];
				const b = reference.indices[i];
				if (a !== b) changed++;
				for (let c = 0; c < 4; c++)
					maximumByteDifference = Math.max(
						maximumByteDifference,
						Math.abs(actual.palette.rgba[a * 4 + c] - reference.palette.rgba[b * 4 + c])
					);
			}
			if (changed > Math.max(4, source.width * source.height * 0.001))
				throw Error(
					`Case ${n} has ${changed} differing indices: ${JSON.stringify(requests[n].dither)}`
				);
			differences += changed;
			if (gpu.execution.analysisReused === true) reuses++;
			if (gpu.execution.analysisReused === false) analyses++;
			if (event.data.full) records.push({ dither: requests[n].dither, differences: changed });
			backends[gpu.execution.backend] = (backends[gpu.execution.backend] ?? 0) + 1;
			if (!event.data.fallback) {
				const backend = requests[n].dither.family === 'diffusion' ? 'hybrid' : 'gpu';
				if (gpu.execution.backend !== backend)
					throw Error(`Expected ${backend}, got ${JSON.stringify(gpu.execution)}`);
			}
		}
		let cancelled = false;
		let callbackRecovered = false;
		if (!event.data.fallback && !event.data.full && !event.data.photo) {
			const diffusion = requests.find((request) => request.dither.family === 'diffusion');
			try {
				await gpu.ditherAndQuantize({
					...diffusion,
					onProgress: (progress) => {
						if (progress.stage === 'dither-and-quantize' && progress.completed > 0)
							throw Error('row callback failed');
					}
				});
				throw Error('Row callback must reject');
			} catch (error) {
				if (error.code !== 'callback') throw error;
			}
			const abort = new AbortController();
			const cancellation = {
				...requests[8],
				dither: {
					...requests[8].dither,
					perturb: {
						...requests[8].dither.perturb,
						placement: { ...requests[8].dither.perturb.placement, contrastThreshold: 0.1215 }
					}
				}
			};
			const pending = gpu.ditherAndQuantize(cancellation, abort.signal);
			setTimeout(() => abort.abort(), 0);
			try {
				await pending;
			} catch (error) {
				if (error.name !== 'AbortError') throw error;
				cancelled = true;
			}
			if (!cancelled) throw Error('GPU cancellation must reject without output');
			await gpu.ditherAndQuantize(requests[8]);
			if (gpu.execution.backend !== 'gpu') throw Error('Cancelled device was not reusable');
			callbackRecovered = true;
		}
		const adapter = gpu.execution.adapter;
		gpu.dispose();
		const nativeAdapter = await navigator.gpu?.requestAdapter();
		self.postMessage({
			cases: requests.length,
			pixels: source.width * source.height * requests.length,
			differences,
			maximumByteDifference,
			backends,
			records,
			analyses,
			reuses,
			cancelled,
			callbackRecovered,
			adapter,
			webgpu: nativeAdapter
				? {
						vendor: nativeAdapter.info.vendor,
						architecture: nativeAdapter.info.architecture,
						isFallbackAdapter:
							nativeAdapter.info.isFallbackAdapter ?? nativeAdapter.isFallbackAdapter
					}
				: null
		});
	} catch (error) {
		self.postMessage({ error: error.stack ?? String(error) });
	}
};
