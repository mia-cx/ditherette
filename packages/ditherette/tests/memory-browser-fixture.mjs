/** Check public boundaries and actual page high-water without private accounting controls. */
export async function memoryBrowserChecks({ wasmUrl, moduleUrl }) {
	const { createDitherette, DitheretteError } = await import(moduleUrl);
	const wasm = await WebAssembly.compile(await (await fetch(wasmUrl)).arrayBuffer());
	const assert = (condition, label) => {
		if (!condition) throw new Error(label);
	};
	const same = (actual, expected, label) =>
		assert(JSON.stringify(actual) === JSON.stringify(expected), label);
	const rejects = (call, code, path) => {
		try {
			call();
		} catch (error) {
			assert(error instanceof DitheretteError, 'structured package error');
			same([error.code, error.path], [code, path], 'error code and path');
			return;
		}
		throw new Error(`Expected ${code} at ${path}`);
	};
	const create = async (memoryLimitBytes) => {
		const instantiate = WebAssembly.instantiate;
		let memory;
		WebAssembly.instantiate = async (...args) => {
			const result = await Reflect.apply(instantiate, WebAssembly, args);
			memory = (result.instance ?? result).exports.memory;
			return result;
		};
		let processor;
		try {
			processor = await createDitherette({ wasm, memoryLimitBytes, threads: 'disabled' });
		} finally {
			WebAssembly.instantiate = instantiate;
		}
		assert(memory instanceof WebAssembly.Memory, 'observe the actual installed scalar memory');
		return { processor, memory };
	};
	const palette = [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] }
	];
	const alpha = { mode: 'premultiplied' };
	const matching = 'srgb-euclidean';
	const perturb = {
		space: 'srgb',
		strength: 0,
		field: { algorithm: 'random', seed: 7 },
		placement: { mode: 'everywhere' }
	};
	const diffusion = {
		family: 'diffusion',
		kernel: 'floyd-steinberg',
		feedback: 'srgb-bytes',
		strength: 0,
		serpentine: true,
		placement: { mode: 'everywhere' }
	};
	const output = (width, height) => ({
		width,
		height,
		resize: { algorithm: 'nearest', anchor: 'center' }
	});
	const requests = (source, size) => [
		['resize', { version: 1, source, output: size }],
		['perturb', { version: 1, source, perturb }],
		['quantize', { version: 1, source, palette, alpha, matching }],
		['ditherAndQuantize', { version: 1, source, palette, alpha, matching, dither: diffusion }],
		[
			'process',
			{
				source,
				palette,
				recipe: { version: 1, output: size, alpha, match: matching, dither: { family: 'none' } }
			}
		]
	];
	const boundaryLimitBytes = 8 * 1024 * 1024;
	const boundedLimitBytes = 256 * 1024;
	const boundary = await create(boundaryLimitBytes);
	const bounded = await create(boundedLimitBytes);
	assert(boundary.memory !== bounded.memory, 'initialized instances own separate memories');
	let maximumAxisCalls = 0;
	let dimensionRejections = 0;
	let budgetRejections = 0;
	try {
		// Maximum axes require only 128 KiB of caller input, not maximum-area images.
		for (const vertical of [false, true]) {
			const source = {
				width: vertical ? 1 : 32768,
				height: vertical ? 32768 : 1,
				data: new Uint8Array(32768 * 4).fill(255)
			};
			const size = output(vertical ? 1 : 16384, vertical ? 16384 : 1);
			for (const [method, request] of requests(source, size)) {
				const result = boundary.processor[method](request);
				const dimensions = method === 'resize' || method === 'process' ? size : source;
				same(
					[result.width, result.height],
					[dimensions.width, dimensions.height],
					`${method}: maximum axis`
				);
				if (result.data) {
					assert(
						result.data.length === dimensions.width * dimensions.height * 4,
						`${method}: RGBA extent`
					);
					assert(
						result.data.every((value) => value === 255),
						`${method}: constant RGBA`
					);
				} else {
					assert(
						result.indices.length === dimensions.width * dimensions.height,
						`${method}: indexed extent`
					);
					assert(
						result.indices.every((value) => value === 1),
						`${method}: constant white index`
					);
					same([...result.palette.rgba], [0, 0, 0, 255, 255, 255, 255, 255], 'palette metadata');
					same(result.warnings, [], 'boundary warnings');
				}
				assert(
					source.data.every((value) => value === 255),
					'maximum-axis input unchanged'
				);
				maximumAxisCalls++;
			}
		}
		// At the area limit only missing storage fails; one row above it fails dimensions first.
		for (const [width, height, path] of [
			[32769, 1, 'source.width'],
			[1, 32769, 'source.height'],
			[32768, 2048, 'source.data'],
			[32768, 2049, 'source']
		]) {
			for (const [method, request] of requests(
				{ width, height, data: new Uint8Array(0) },
				output(1, 1)
			)) {
				rejects(() => boundary.processor[method](request), 'invalid-image', path);
				dimensionRejections++;
			}
		}
		const tiny = { width: 1, height: 1, data: new Uint8Array([255, 255, 255, 255]) };
		for (const [width, height, path] of [
			[16385, 1, 'output.width'],
			[1, 16385, 'output.height'],
			[16384, 4097, 'output'],
			[0, 1, 'output.width']
		]) {
			rejects(
				() => bounded.processor.resize({ version: 1, source: tiny, output: output(width, height) }),
				'invalid-settings',
				path
			);
			dimensionRejections++;
		}
		// Exactly 67,108,864 output pixels pass shape validation but exceed this instance budget.
		for (const [method, request] of requests(tiny, output(16384, 4096)).filter(
			([method]) => method === 'resize' || method === 'process'
		)) {
			const stages = [];
			rejects(
				() =>
					bounded.processor[method]({ ...request, onProgress: ({ stage }) => stages.push(stage) }),
				'memory-limit',
				'memoryLimitBytes'
			);
			assert(
				stages.every((stage) => stage === 'prepare'),
				`${method}: rejection precedes processing`
			);
			budgetRejections++;
		}

		const source = { width: 33, height: 25, data: new Uint8Array(33 * 25 * 4) };
		const size = output(17, 13);
		const processRequest = requests(source, size).at(-1)[1];
		const quantizeRequest = requests(source, size)[2][1];
		const fill = (value) => {
			for (let offset = 0; offset < source.data.length; offset += 4)
				source.data.set([value, value, value, 255], offset);
		};
		fill(255);
		const durable = bounded.processor.process(processRequest);
		const saved = structuredClone(durable);
		const batch = () => {
			for (let value = 0; value < 128; value++) {
				fill(value * 2);
				const expected = value < 64 ? 0 : 1;
				for (const [method, request] of [
					['process', processRequest],
					['quantize', quantizeRequest]
				]) {
					const result = bounded.processor[method](request);
					const dimensions = method === 'process' ? size : source;
					same(
						[result.width, result.height, result.indices.length],
						[dimensions.width, dimensions.height, dimensions.width * dimensions.height],
						`${method}: repeated extent`
					);
					assert(
						result.indices.every((index) => index === expected),
						`${method}: changed-source result`
					);
					same(result.warnings, [], `${method}: repeated warnings`);
				}
				assert(
					source.data.every((byte, index) => byte === (index % 4 === 3 ? 255 : value * 2)),
					'repeated input stays caller-owned'
				);
			}
		};
		const initialBytes = bounded.memory.buffer.byteLength;
		batch();
		const steadyBytes = bounded.memory.buffer.byteLength;
		const repeatedBytes = [];
		for (let repetition = 0; repetition < 2; repetition++) {
			batch();
			repeatedBytes.push(bounded.memory.buffer.byteLength);
			assert(
				bounded.memory.buffer.byteLength === steadyBytes,
				'repeated changing calls reach stable Wasm page high-water'
			);
		}
		fill(255);
		same(
			bounded.processor.process(processRequest),
			saved,
			'first input recomputes correctly after bounded churn'
		);
		same(durable, saved, 'repeated calls preserve earlier output');
		bounded.processor.dispose();
		same(durable, saved, 'disposal preserves earlier output');
		rejects(() => bounded.processor.process(processRequest), 'disposed', 'instance');
		const survivor = boundary.processor.resize({ version: 1, source: tiny, output: output(1, 1) });
		boundary.processor.dispose();
		assert(
			survivor.data.every((value) => value === 255),
			'other instance and durable output survive disposal'
		);
		return {
			maximumAxisCalls,
			dimensionRejections,
			budgetRejections,
			warmupCalls: 256,
			repeatedCalls: 512,
			durableOutputs: 2,
			memory: {
				boundaryLimitBytes,
				boundedLimitBytes,
				initialBytes,
				steadyBytes,
				repeatedBytes,
				boundaryHighWaterBytes: boundary.memory.buffer.byteLength,
				afterDisposeBytes: bounded.memory.buffer.byteLength
			}
		};
	} finally {
		boundary.processor.dispose();
		bounded.processor.dispose();
	}
}
