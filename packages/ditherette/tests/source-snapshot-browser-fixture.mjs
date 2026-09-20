/** Verify source snapshots through the installed API and its caught input-copy boundary. */
export async function sourceSnapshotBrowserChecks({
	wasmUrl,
	vectors,
	moduleUrl = '/node_modules/ditherette/dist/index.js'
}) {
	const { createDitherette, DitheretteError } = await import(moduleUrl);
	const wasm = await (await fetch(wasmUrl)).arrayBuffer();
	const typedArray = Object.getPrototypeOf(Uint8Array.prototype);
	const buffer = Object.getOwnPropertyDescriptor(typedArray, 'buffer').get;
	const offset = Object.getOwnPropertyDescriptor(typedArray, 'byteOffset').get;
	const length = Object.getOwnPropertyDescriptor(typedArray, 'length').get;
	const same = (actual, expected, label) => {
		if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(label);
	};
	const palette = [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] },
		{ kind: 'transparent' }
	];
	const alpha = { mode: 'preserve', threshold: 127 };
	const matching = 'srgb-euclidean';
	const output = { width: 3, height: 3, resize: { algorithm: 'nearest', anchor: 'center' } };
	const perturb = vectors.cases[0].policy;
	for (const method of ['resize', 'perturb', 'quantize', 'ditherAndQuantize', 'process']) {
		const processor = await createDitherette({ wasm });
		const reference = await createDitherette({ wasm });
		const backing = new Uint8Array([99, ...vectors.source.data, 98]);
		const source = { ...vectors.source, data: backing.subarray(1, backing.length - 1) };
		const quantize = { version: 1, source, palette, alpha, matching };
		const request = {
			resize: { version: 1, source, output },
			perturb: { version: 1, source, perturb },
			quantize,
			ditherAndQuantize: { ...quantize, dither: { family: 'none' } },
			process: {
				source,
				palette,
				recipe: { version: 1, output, alpha, match: matching, dither: { family: 'none' } }
			}
		}[method];
		const run = (copies, extra = {}) => {
			const set = Uint8Array.prototype.set;
			let inputs = 0;
			Uint8Array.prototype.set = function (...args) {
				const input = args[0];
				if (
					input instanceof Uint8Array &&
					buffer.call(input) === buffer.call(source.data) &&
					offset.call(input) === offset.call(source.data) &&
					length.call(input) === length.call(source.data)
				)
					inputs++;
				return Reflect.apply(set, this, args);
			};
			try {
				return processor[method]({ ...request, ...extra });
			} finally {
				Uint8Array.prototype.set = set;
				same(inputs, copies, `${method}: expected ${copies} input copies, received ${inputs}`);
			}
		};
		try {
			const first = run(1);
			const saved = structuredClone(first);
			same(run(0), saved, `${method}: offset-1 snapshot reuse`);
			// A new view with equal contents also reuses the snapshot. Object identity cannot decide it.
			source.data = new Uint8Array(source.data);
			same(run(0), saved, `${method}: equal bytes in another allocation`);
			source.width = 4;
			source.height = 1;
			same(run(1), reference[method](request), `${method}: dimensions participate in identity`);
			same(run(0), reference[method](request), `${method}: changed dimensions become reusable`);
			source.data[source.data.length - 1] = 0;
			const changed = run(1);
			same(changed, reference[method](request), `${method}: mutation of the last byte`);
			try {
				run(0, {
					onProgress(event) {
						if (event.stage === 'complete') throw new Error('completion failed');
					}
				});
				throw new Error(`${method}: completion should fail`);
			} catch (error) {
				if (
					!(error instanceof DitheretteError) ||
					error.code !== 'callback' ||
					error.path !== 'onProgress'
				)
					throw error;
			}
			same(run(1), changed, `${method}: failed completion discards the source snapshot`);
			same(run(0), changed, `${method}: recovered snapshot can be reused`);
			processor.dispose();
			same(first, saved, `${method}: earlier output survives mutation, failure and disposal`);
			same([...backing], [99, ...vectors.source.data, 98], `${method}: offset view guard bytes`);
		} finally {
			processor.dispose();
			reference.dispose();
		}
	}
	return { methods: 5 };
}
