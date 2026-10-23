import { createDitherette } from '../dist/index.js';
import { tilingCases } from './tiling-cases.mjs';

self.onmessage = async (event) => {
	if (event.origin && event.origin !== self.location.origin) return;
	const source = {
		width: 53,
		height: 37,
		data: Uint8Array.from({ length: 53 * 37 * 4 }, (_, i) =>
			i % 4 === 3 ? [0, 127, 255][(i >>> 2) % 3] : 40 + ((i >>> 2) % 53) * 2 + (i % 4) * 12
		)
	};
	const palette = [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] },
		{ kind: 'color', rgb: [110, 140, 160] },
		{ kind: 'transparent' }
	];
	let calls = 0;
	try {
		for (const engine of ['cpu', 'gpu']) {
			const whole = await createDitherette({
				engine,
				tiling: false,
				...(engine === 'gpu' ? { fallback: 'error' } : {})
			});
			const tiled = await createDitherette({
				engine,
				tiling: { height: 3, workers: 4 },
				...(engine === 'gpu' ? { fallback: 'error' } : { threads: 'required' })
			});
			try {
				for (const item of tilingCases(source, palette)) {
					if (engine === 'gpu' && item.cpuOnly) continue;
					const a = await whole[item.method](item.request),
						b = await tiled[item.method](item.request);
					if (JSON.stringify(a) !== JSON.stringify(b))
						throw Error(`${engine} ${item.id}: tiling changed output`);
					if (
						engine === 'gpu' &&
						(whole.execution.backend !== 'gpu' || tiled.execution.backend !== 'gpu')
					)
						throw Error(`${item.id}: unexpected fallback`);
					calls++;
				}
			} finally {
				tiled.dispose();
				whole.dispose();
			}
		}
		self.postMessage({ calls, exact: true, threadsRequired: true });
	} catch (error) {
		self.postMessage({ error: error.stack ?? String(error) });
	}
};
