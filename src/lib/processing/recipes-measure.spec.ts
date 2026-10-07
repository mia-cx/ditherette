import { describe, expect, it } from 'vitest';
import { readFile } from 'node:fs/promises';
import { createDitherette } from 'ditherette';
import { measureFits } from './recipes';

const wasm = await WebAssembly.compile(
	await readFile(
		new URL(
			'../../../packages/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm',
			import.meta.url
		)
	)
);

const ramp = () => {
	const data = new Uint8Array(32 * 32 * 4);
	for (let i = 0; i < data.length; i += 4)
		data.set([i & 255, (i >> 2) & 255, (i >> 4) & 255, 255], i);
	return { width: 32, height: 32, data };
};
import type { PaletteEntry } from 'ditherette';
const palette = [
	{ kind: 'color', rgb: [0, 0, 0] },
	{ kind: 'color', rgb: [255, 0, 0] },
	{ kind: 'color', rgb: [0, 0, 255] },
	{ kind: 'color', rgb: [255, 255, 255] }
] satisfies PaletteEntry[];

describe('measureFits', () => {
	it('returns the measurements every palette-fit step analysed', async () => {
		const ditherette = await createDitherette({ wasm, memoryLimitBytes: 16 * 1024 * 1024 });
		const fits = measureFits(
			ditherette,
			ramp(),
			[
				{
					effect: 'levels',
					enabled: true,
					channel: 'rgb',
					input: { black: 0, white: 1 },
					gamma: 1,
					output: { black: 0, white: 1 }
				},
				{
					effect: 'palette-fit',
					enabled: true,
					look: 'fitted',
					space: 'oklab',
					strength: 1,
					curves: null
				}
			],
			{ palette, space: 'oklab' },
			undefined
		);
		expect(fits).toHaveLength(1);
		expect(fits[0].index).toBe(1);
		expect(fits[0].edited).toBe(false);
		const measurements = fits[0].measurements!;
		expect(measurements.toneQuantiles).toHaveLength(5);
		expect(measurements.reach).toHaveLength(5);
		expect(measurements.reach.every((row) => row.length === 12)).toBe(true);
	});
});
