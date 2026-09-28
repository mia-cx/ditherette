import type { Ditherette, Effect, EffectContext, Rgba8Image } from 'ditherette';
import { initializePackageProcessor } from '$lib/processing/worker-pipeline';
import {
	LUT_SIZE,
	type SourceEffectsRequest,
	type SourceEffectsResponse
} from '$lib/processing/source-effects';

/**
 * Compiles the effect chain into a 3D lookup table for the Source pane. The source stays here so
 * palette fit can analyse the real image; the table itself is tiny.
 */
let source: Rgba8Image | undefined;
let processor: Promise<Ditherette> | undefined;

/** Every lattice colour once, red fastest, then green, then blue: the layout of a 3D texture. */
const LATTICE = (() => {
	const step = 255 / (LUT_SIZE - 1);
	const data = new Uint8Array(LUT_SIZE ** 3 * 4);
	let offset = 0;
	for (let blue = 0; blue < LUT_SIZE; blue++)
		for (let green = 0; green < LUT_SIZE; green++)
			for (let red = 0; red < LUT_SIZE; red++, offset += 4)
				data.set([red * step, green * step, blue * step, 255], offset);
	return { width: LUT_SIZE, height: LUT_SIZE * LUT_SIZE, data };
})();

/**
 * Give each recipe-less palette fit the recipe it would derive from the real source, since on the
 * lattice it would analyse the lattice instead.
 */
function resolveRecolour(
	ditherette: Ditherette,
	effects: Effect[],
	context: Required<EffectContext>
) {
	return effects.map((step, index) =>
		step.effect === 'recolour' && step.recipe === null && source
			? {
					...step,
					recipe: ditherette.analyzeRecolour({
						version: 1,
						source,
						effects: effects.slice(0, index),
						context
					})
				}
			: step
	);
}

self.onmessage = async ({ data }: MessageEvent<SourceEffectsRequest>) => {
	if (data.type === 'source') {
		const { width, height, data: pixels } = data.source;
		source = {
			width,
			height,
			data: new Uint8Array(pixels.buffer, pixels.byteOffset, pixels.byteLength)
		};
		return;
	}
	const reply = (response: SourceEffectsResponse, transfer: Transferable[] = []) =>
		self.postMessage(response, { transfer });
	try {
		processor ??= initializePackageProcessor().catch((error: unknown) => {
			processor = undefined;
			throw error;
		});
		const ditherette = await processor;
		const { data: lut } = ditherette.applyEffects({
			version: 1,
			source: LATTICE,
			effects: resolveRecolour(ditherette, data.effects, data.context),
			context: data.context
		});
		// applyEffects returns independent JS-owned bytes, or the lattice itself when nothing changes.
		const table = lut === LATTICE.data ? lut.slice() : lut;
		reply({ id: data.id, lut: table }, [table.buffer as ArrayBuffer]);
	} catch (error) {
		reply({ id: data.id, error: error instanceof Error ? error.message : 'Effects failed.' });
	}
};
