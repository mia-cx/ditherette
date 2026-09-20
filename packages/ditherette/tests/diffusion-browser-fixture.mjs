/** Execute frozen diffusion vectors against the installed package without measurement timers. */
export async function diffusionBrowserChecks({ vectors, wasmUrl }) {
	const { createDitherette } = await import('ditherette');
	const processor = await createDitherette({ wasm: wasmUrl });
	const source = { ...vectors.source, data: new Uint8Array(vectors.source.data) };
	const palette = vectors.palette.flatMap((entry) =>
		entry.kind === 'transparent' ? [0, 0, 0, 0] : [...entry.rgb, 255]
	);
	let retained;
	let snapshot;
	try {
		for (const vector of vectors.cases) {
			const output = processor.ditherAndQuantize({
				version: 1,
				source,
				palette: vectors.palette,
				alpha: vector.alpha,
				matching: vector.matching,
				dither: vector.dither
			});
			if (
				JSON.stringify([...output.indices]) !== JSON.stringify(vector.indices) ||
				JSON.stringify([...output.palette.rgba]) !== JSON.stringify(palette) ||
				output.palette.transparentIndex !== 4 ||
				JSON.stringify(output.warnings) !== JSON.stringify(vector.warnings)
			)
				throw new Error(`Diffusion mismatch: ${JSON.stringify(vector)}`);
			if (
				retained?.indices.buffer === output.indices.buffer ||
				output.indices.buffer === source.data.buffer
			)
				throw new Error('Diffusion output aliases prior storage.');
			retained ??= output;
			snapshot ??= [...output.indices];
		}
		if (JSON.stringify([...source.data]) !== JSON.stringify(vectors.source.data))
			throw new Error('Diffusion mutated its source.');
		processor.dispose();
		if (JSON.stringify([...retained.indices]) !== JSON.stringify(snapshot))
			throw new Error('Disposal invalidated diffusion output.');
		return { diffusion: vectors.cases.length, scalarWithoutIsolation: !crossOriginIsolated };
	} finally {
		processor.dispose();
	}
}
