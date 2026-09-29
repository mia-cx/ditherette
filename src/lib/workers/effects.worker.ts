import {
	compileEffects,
	indexColours,
	type Ditherette,
	type Effect,
	type EffectContext,
	type RecolourRecipe,
	type Rgba8Image
} from 'ditherette';
import type { LiveEffectsRequest, LiveEffectsResponse } from '$lib/processing/live-effects';
import { croppedSource } from '$lib/processing/package-adapter';
import type { CropRect } from '$lib/processing/types';
import { initializePackageProcessor } from '$lib/processing/worker-pipeline';

/**
 * Compiles effects for the live Source preview and for processing to reuse. It keeps the source
 * for palette fit's analysis, and its distinct colours, so each edit runs the chain once per colour.
 */
let processor: Promise<Ditherette> | undefined;
let loaded: { sourceId: number; image: Rgba8Image; colours: Uint32Array } | undefined;
/** Palette fit recipes for the loaded source, by what their analysis read. */
const recipes = new Map<string, RecolourRecipe>();

const post = (response: LiveEffectsResponse, transfer: Transferable[] = []) =>
	self.postMessage(response, { transfer });

/** Resolve each recipe-less palette fit, analysing the cropped source only once per input. */
function withRecipes(
	ditherette: Ditherette,
	image: Rgba8Image,
	effects: Effect[],
	context: Required<EffectContext>,
	crop: CropRect | undefined
): Effect[] {
	return effects.map((step, index) => {
		if (step.effect !== 'recolour' || step.recipe !== null) return step;
		const preceding = effects.slice(0, index);
		const key = JSON.stringify([preceding, context, crop ?? null]);
		let recipe = recipes.get(key);
		if (!recipe) {
			const source = croppedSource(image, crop);
			recipe = ditherette.analyzeRecolour({ version: 1, source, effects: preceding, context });
			recipes.set(key, recipe);
		}
		return { ...step, recipe };
	});
}

function index(sourceId: number, source: ImageData) {
	const image = {
		width: source.width,
		height: source.height,
		data: new Uint8Array(source.data.buffer, source.data.byteOffset, source.data.length)
	};
	const { colours, indices } = indexColours(image);
	loaded = { sourceId, image, colours };
	recipes.clear();
	const pixels = new Uint32Array(indices.length);
	for (let pixel = 0; pixel < indices.length; pixel++)
		pixels[pixel] = (indices[pixel]! | (image.data[pixel * 4 + 3]! << 24)) >>> 0;
	post({ type: 'indexed', sourceId, pixels, count: colours.length }, [pixels.buffer]);
}

self.onmessage = async ({ data }: MessageEvent<LiveEffectsRequest>) => {
	if (data.type === 'source') return index(data.sourceId, data.source);
	const { id, sourceId, key, effects, context, crop } = data;
	try {
		processor ??= initializePackageProcessor().catch((error: unknown) => {
			processor = undefined;
			throw error;
		});
		const ditherette = await processor;
		if (loaded?.sourceId !== sourceId) throw new Error('The effects worker has another image.');
		const results = compileEffects(ditherette, {
			version: 1,
			colours: loaded.colours,
			effects: withRecipes(ditherette, loaded.image, effects, context, crop),
			context
		});
		post({ type: 'compiled', id, sourceId, key, results }, [results.buffer]);
	} catch (error) {
		const message = error instanceof Error ? error.message : 'Effects failed.';
		post({ type: 'failed', id, sourceId, key, message });
	}
};
