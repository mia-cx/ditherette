import { createDitherette, createGpuDither, createGpuEffects } from '../dist/index.js';

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
		if (event.data.pixels) {
			// Proves standalone RGBA calls use GPU execution, preserve alpha/hidden
			// colours and retain their ordinary result across row-band policies.
			const cpu = await createDitherette();
			const gpu = await createGpuDither({ fallback: 'error', tiling: 'disabled' });
			const tiled = await createGpuDither({ fallback: 'error', tiling: { height: 3 } });
			const masks = await createGpuEffects({ fallback: 'error', tiling: { height: 3 } });
			let calls = 0,
				maximum = 0;
			const compare = (actual, expected) => {
				if (actual.width !== source.width || actual.height !== source.height)
					throw Error('RGBA dimensions differ');
				for (let i = 0; i < expected.data.length; i++) {
					maximum = Math.max(maximum, Math.abs(actual.data[i] - expected.data[i]));
					if (i % 4 === 3 && actual.data[i] !== source.data[i]) throw Error('Alpha changed');
				}
				calls++;
			};
			try {
				for (const space of ['srgb', 'linear-rgb', 'oklab', 'oklch', 'cielab', 'cielch', 'ycbcr'])
					for (const field of [
						{ algorithm: 'bayer', size: '16' },
						{ algorithm: 'blue-noise' },
						{ algorithm: 'random', seed: 4294967281 },
						{ algorithm: 'ordered', tile: '5x3' }
					])
						for (const placement of [
							{ mode: 'everywhere' },
							{ mode: 'smooth-area-v1', sampleSize: 20, contrastThreshold: 0.12 }
						]) {
							const request = {
								version: 1,
								source,
								perturb: { space, field, strength: 0.3, placement }
							};
							const expected = cpu.perturb(request),
								actual = await gpu.perturb(request);
							compare(actual, expected);
							const banded = await tiled.perturb(request);
							if (actual.data.some((value, i) => value !== banded.data[i]))
								throw Error('Tile phase differs');
							if (gpu.execution.backend !== 'gpu' || tiled.execution.backend !== 'gpu')
								throw Error('RGBA left GPU');
						}
				for (const effects of [
					[],
					[{ effect: 'exposure', enabled: true, stops: 0.33 }],
					[
						{ effect: 'exposure', enabled: true, stops: 2 },
						{ effect: 'hue-saturation', enabled: true, hue: 30, saturation: 0.2, lightness: 0 }
					]
				])
					for (const mask of [
						[],
						[
							{
								x: { model: 'srgb', channel: 'red' },
								points: [
									[0, 0],
									[0.4, 0.8],
									[1, 1]
								]
							}
						]
					]) {
						const request = { version: 1, source, effects, mask };
						const actual = await masks.effectMask(request);
						compare(actual, cpu.effectMask(request));
						if (masks.execution.backend !== 'gpu') throw Error('Mask left GPU');
						for (let i = 0; i < actual.data.length; i += 4)
							if (actual.data[i] !== actual.data[i + 1] || actual.data[i] !== actual.data[i + 2])
								throw Error('Mask is not grey');
					}
				if (maximum > 1) throw Error(`RGBA difference ${maximum}`);
				self.postMessage({ calls, maximum, exactTiles: true, alphaPreserved: true });
			} finally {
				cpu.dispose();
				gpu.dispose();
				tiled.dispose();
				masks.dispose();
			}
			return;
		}
		if (event.data.gpuTiling) {
			const whole = await createGpuDither({ fallback: 'error', tiling: 'disabled' });
			const tiled = await createGpuDither({ fallback: 'error', tiling: { height: 3 } });
			const base = {
				version: 1,
				source,
				palette,
				alpha: { mode: 'preserve', threshold: 127.9 },
				matching: 'oklab-euclidean'
			};
			let calls = 0;
			try {
				for (const placement of [
					{ mode: 'everywhere' },
					{ mode: 'smooth-area-v1', sampleSize: 20, contrastThreshold: 0.12 }
				]) {
					for (const dither of [
						{ family: 'none' },
						{ family: 'yliluoma', size: '4', placement },
						...[
							{ algorithm: 'bayer', size: '16' },
							{ algorithm: 'random', seed: 0xfffffff1 },
							{ algorithm: 'blue-noise' },
							{ algorithm: 'ordered', tile: '5x3' }
						].map((field) => ({
							family: 'separable',
							perturb: { field, space: 'oklab', strength: 0.3, placement }
						}))
					]) {
						const input = { ...base, dither };
						const a = await whole.ditherAndQuantize(input),
							b = await tiled.ditherAndQuantize(input);
						if (whole.execution.backend !== 'gpu' || tiled.execution.backend !== 'gpu')
							throw Error('Indexed row tiles left GPU');
						if (a.indices.some((value, index) => value !== b.indices[index]))
							throw Error('Indexed row tiles changed global phase');
						const recipe = {
							version: 2,
							effects: [
								{ effect: 'hue-saturation', enabled: true, hue: 30, saturation: 0.2, lightness: 0 }
							],
							output: {
								width: 29,
								height: 23,
								resize: { algorithm: 'lanczos3', anchor: 'center', support: 'scale-aware' }
							},
							alpha: base.alpha,
							match: base.matching,
							dither
						};
						const x = await whole.process({ source, palette, recipe }),
							y = await tiled.process({ source, palette, recipe });
						if (y.indices.some((value, index) => value !== x.indices[index]))
							throw Error('Combined row tiles changed output');
						if (tiled.execution.backend !== 'gpu') throw Error('Combined row tiles left GPU');
						calls += 4;
					}
				}
				let unsupported = false;
				// 4096² crosses the old 65535-workgroup single-dispatch limit.
				// Gray 127 is strictly closer to black under sRGB Euclidean matching.
				const large = {
					...base,
					source: { width: 4096, height: 4096, data: new Uint8Array(4096 * 4096 * 4).fill(127) },
					palette: [
						{ kind: 'color', rgb: [0, 0, 0] },
						{ kind: 'color', rgb: [255, 255, 255] }
					],
					matching: 'srgb-euclidean',
					alpha: { mode: 'preserve', threshold: 0 }
				};
				for (const processor of [whole, tiled]) {
					const image = await processor.quantize(large);
					if (
						processor.execution.backend !== 'gpu' ||
						image.indices.length !== 4096 * 4096 ||
						image.indices.some((index) => index !== 0)
					)
						throw Error('4k GPU quantization differs');
					calls++;
				}
				try {
					await tiled.ditherAndQuantize({
						...base,
						dither: {
							family: 'diffusion',
							kernel: 'sierra',
							feedback: 'matching',
							serpentine: true,
							strength: 1,
							placement: { mode: 'everywhere' }
						}
					});
				} catch (error) {
					unsupported = error.code === 'unsupported-operation';
				}
				if (!unsupported) throw Error('Independent row tiles cannot restart continuous diffusion');
				self.postMessage({ calls, exact: true, largeCorrect: true, diffusionRejected: true });
			} finally {
				whole.dispose();
				tiled.dispose();
			}
			return;
		}
		if (event.data.process) {
			// Proves the public combined call composes the real GPU stages at the
			// same RGBA8 boundary and recipe-v2 coverage semantics as CPU process.
			const cpu = await createDitherette();
			const gpu = await createGpuDither({ fallback: 'error', diffusionBackend: 'gpu' });
			const area = { mode: 'smooth-area-v1', sampleSize: 20, contrastThreshold: 0.12 };
			if (event.data.fullProcess) {
				const effects = [
					{ effect: 'exposure', enabled: true, stops: 0.5 },
					{ effect: 'hue-saturation', enabled: true, hue: 30, saturation: 0.2, lightness: 0 }
				];
				const records = [];
				try {
					for (let i = 0; i < 4; i++) {
						const width = i === 3 ? 1300 : source.width,
							height = i === 3 ? 2084 : source.height;
						const input = {
							source,
							palette,
							recipe: {
								version: 2,
								effects,
								output: {
									width,
									height,
									resize: { algorithm: 'lanczos3', anchor: 'center', support: 'scale-aware' }
								},
								alpha: { mode: 'preserve', threshold: 0 },
								match: 'oklab-euclidean',
								dither:
									i === 0
										? { family: 'none' }
										: {
												family: 'separable',
												perturb: {
													field: { algorithm: 'bayer', size: '16' },
													space: 'oklab',
													strength: i === 2 ? 0.5 : 0.3,
													placement: area
												}
											}
							}
						};
						const expected = cpu.process(input),
							actual = await gpu.process(input);
						if (gpu.execution.backend !== 'gpu') throw Error('Full process left GPU');
						if (
							JSON.stringify(actual.palette) !== JSON.stringify(expected.palette) ||
							JSON.stringify(actual.warnings) !== JSON.stringify(expected.warnings)
						)
							throw Error('Full process metadata differs');
						let changed = 0,
							maxByte = 0,
							alphaDifferences = 0;
						for (let pixel = 0; pixel < actual.indices.length; pixel++) {
							const a = actual.indices[pixel],
								b = expected.indices[pixel];
							if (a !== b) changed++;
							for (let c = 0; c < 4; c++)
								maxByte = Math.max(
									maxByte,
									Math.abs(actual.palette.rgba[4 * a + c] - expected.palette.rgba[4 * b + c])
								);
							if (actual.palette.rgba[4 * a + 3] !== expected.palette.rgba[4 * b + 3])
								alphaDifferences++;
						}
						if (alphaDifferences) throw Error('Full process alpha differs');
						const hash = Array.from(
							new Uint8Array(await crypto.subtle.digest('SHA-256', actual.indices)),
							(b) => b.toString(16).padStart(2, '0')
						).join('');
						records.push({
							i,
							width,
							height,
							changed,
							maxByte,
							alphaDifferences,
							hash,
							...gpu.execution
						});
					}
					if (!records[2].analysisReused || records[3].analysisReused)
						throw Error('Full prepared geometry cache identity differs');
					self.postMessage({ calls: 4, records });
				} finally {
					cpu.dispose();
					gpu.dispose();
				}
				return;
			}
			const dithers = [
				{ family: 'none' },
				{
					family: 'separable',
					perturb: {
						field: { algorithm: 'bayer', size: '16' },
						space: 'oklab',
						strength: 0.3,
						placement: area
					}
				},
				{
					family: 'diffusion',
					kernel: 'sierra',
					feedback: 'matching',
					serpentine: false,
					strength: 1,
					placement: area
				},
				{ family: 'yliluoma', size: '4', placement: area }
			];
			const filters = [
				{ algorithm: 'nearest', anchor: 'center' },
				{ algorithm: 'area' },
				{ algorithm: 'bilinear', anchor: 'bottom-right' },
				{ algorithm: 'trilinear', anchor: 'center' },
				...['bicubic', 'lanczos2', 'lanczos3'].map((algorithm) => ({
					algorithm,
					anchor: 'center',
					support: 'scale-aware'
				}))
			];
			const profiles = [
				undefined,
				[],
				[
					{ effect: 'exposure', enabled: true, stops: 0.5 },
					{ effect: 'hue-saturation', enabled: true, hue: 30, saturation: 0.2, lightness: 0 }
				],
				[
					{
						effect: 'curves',
						enabled: true,
						curves: [
							{
								kind: 'adjust',
								x: { model: 'hsl', channel: 'hue' },
								y: { model: 'oklab', channel: 'a' },
								points: [
									[0, 0.6],
									[0.5, 0.4],
									[1, 0.6]
								]
							}
						],
						mask: [
							{
								x: { model: 'srgb', channel: 'red' },
								points: [
									[0, 0.1],
									[1, 1]
								]
							}
						]
					}
				]
			];
			let calls = 0,
				differences = 0,
				maximumByteDifference = 0;
			let last;
			try {
				for (const resize of filters)
					for (const effects of profiles)
						for (const dither of dithers)
							for (const alpha of [
								{ mode: 'preserve', threshold: 127.9 },
								{ mode: 'matte', rgb: [7, 200, 90] }
							]) {
								const request = {
									source,
									palette,
									recipe: {
										version: effects === undefined ? 1 : 2,
										...(effects === undefined ? {} : { effects }),
										output: { width: 29, height: 23, resize },
										alpha,
										match: 'oklab-euclidean',
										dither
									}
								};
								const expected = cpu.process(request);
								const actual = await gpu.process(request);
								if (gpu.execution.backend !== 'gpu')
									throw Error('Combined process must stay on GPU');
								if (
									actual.width !== 29 ||
									actual.height !== 23 ||
									actual.indices.length !== 29 * 23
								)
									throw Error('Output dimensions changed');
								if (
									JSON.stringify(actual.palette) !== JSON.stringify(expected.palette) ||
									JSON.stringify(actual.warnings) !== JSON.stringify(expected.warnings)
								)
									throw Error('Palette metadata changed');
								let changed = 0;
								for (let i = 0; i < actual.indices.length; i++) {
									if (actual.indices[i] !== expected.indices[i]) changed++;
									for (let c = 0; c < 4; c++)
										maximumByteDifference = Math.max(
											maximumByteDifference,
											Math.abs(
												actual.palette.rgba[4 * actual.indices[i] + c] -
													expected.palette.rgba[4 * expected.indices[i] + c]
											)
										);
								}
								if (changed > 40)
									throw Error(
										`Combined case ${calls} changed ${changed} indices: ${JSON.stringify(request.recipe)}`
									);
								differences += changed;
								calls++;
								last = request;
							}
				// Unchanged prepared bytes reuse coverage across dither settings; changed
				// effects invalidate it even though the caller's original source is identical.
				last.recipe.dither = dithers[1];
				await gpu.process(last);
				await gpu.process(last);
				if (!gpu.execution.analysisReused) throw Error('Prepared-source coverage did not reuse');
				last.recipe.effects[0].curves[0].points[1][1] = 0.8;
				await gpu.process(last);
				if (gpu.execution.analysisReused) throw Error('Changed effects reused stale coverage');
				const pendingSource = { ...source, data: source.data.slice() };
				const capturedRequest = { ...last, source: pendingSource };
				const expected = cpu.process(capturedRequest);
				const pending = gpu.process(capturedRequest);
				pendingSource.data.fill(0);
				const captured = await pending;
				if (captured.indices.some((index, i) => index !== expected.indices[i]))
					throw Error('Source changed after capture');
				const controller = new AbortController();
				let cancelled = false;
				try {
					await gpu.process(
						{
							...last,
							onProgress({ stage }) {
								if (stage === 'resize') controller.abort();
							}
						},
						controller.signal
					);
				} catch (error) {
					cancelled = error.name === 'AbortError';
				}
				if (!cancelled) throw Error('Preparation cancellation did not reject');
				let callbackRejected = false;
				try {
					await gpu.process({
						...last,
						onProgress({ stage }) {
							if (stage === 'resize') throw Error('callback');
						}
					});
				} catch (error) {
					callbackRejected = error.code === 'callback';
				}
				if (!callbackRejected) throw Error('Resize callback failure did not reject');
				await gpu.process(last);
				if (gpu.execution.backend !== 'gpu') throw Error('Callback retired a healthy device');
				const hybrid = await createGpuDither();
				try {
					await hybrid.process({
						...last,
						recipe: { ...last.recipe, dither: { ...dithers[2], placement: { mode: 'everywhere' } } }
					});
					if (hybrid.execution.backend !== 'hybrid')
						throw Error('CPU diffusion must report GPU preparation as hybrid');
				} finally {
					hybrid.dispose();
				}
				self.postMessage({
					calls,
					differences,
					maximumByteDifference,
					reused: true,
					hybrid: true,
					cancelled,
					callbackRecovered: true
				});
			} finally {
				cpu.dispose();
				gpu.dispose();
			}
			return;
		}
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
		const mixtures = matching.flatMap((match) =>
			['2', '4', '8', '16'].flatMap((size) =>
				[{ mode: 'everywhere' }, area].map((placement) => ({
					matching: match,
					dither: { family: 'yliluoma', size, placement }
				}))
			)
		);
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
		const selected =
			event.data.full === 'yliluoma'
				? mixtures.filter(
						(item) =>
							['srgb-euclidean', 'oklab-euclidean'].includes(item.matching) &&
							item.dither.size === '16'
					)
				: event.data.gpuDiffusion
					? cases.filter((item) => item.dither.family === 'diffusion')
					: event.data.yliluoma
						? mixtures
						: event.data.full
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
		if (event.data.yliluoma) {
			// Proves the truncation warning marker is not admitted as colour 257.
			requests.push({
				...requests[0],
				palette: Array.from({ length: 257 }, (_, i) => ({
					kind: 'color',
					rgb: [i % 2 ? 255 : 0, i % 2 ? 255 : 0, i % 2 ? 255 : 0]
				}))
			});
		}
		let cpu = await createDitherette();
		const expected = requests.map((request) => cpu.ditherAndQuantize(request));
		cpu.dispose();
		cpu = undefined;
		const gpu = await createGpuDither(
			event.data.gpuDiffusion ? { diffusionBackend: 'gpu', fallback: 'error' } : undefined
		);
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
				const backend =
					requests[n].dither.family === 'diffusion' && !event.data.gpuDiffusion ? 'hybrid' : 'gpu';
				if (gpu.execution.backend !== backend)
					throw Error(`Expected ${backend}, got ${JSON.stringify(gpu.execution)}`);
			}
		}
		let cancelled = false;
		let callbackRecovered = false;
		if (
			!event.data.fallback &&
			!event.data.full &&
			!event.data.photo &&
			!event.data.yliluoma &&
			!event.data.gpuDiffusion
		) {
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
