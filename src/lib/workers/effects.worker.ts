import {
	compileEffectMask,
	compileEffects,
	indexColours,
	type Curve,
	type Ditherette,
	type RecolourRecipe,
	type Rgba8Image
} from 'ditherette';
import type { LiveEffectsRequest, LiveEffectsResponse } from '$lib/processing/live-effects';
import { resolveRecipes, type AnalysedFit } from '$lib/processing/recipes';
import { initializePackageProcessor } from '$lib/processing/worker-pipeline';

/**
 * Compiles effects for the live Source preview and for processing to reuse. It keeps the source
 * for palette fit's analysis, and its distinct colours, so each edit runs the chain once per colour.
 */
let processor: Promise<Ditherette> | undefined;
let loaded: { sourceId: number; image: Rgba8Image; colours: Uint32Array } | undefined;
/** Palette fit analyses for the loaded source, by what their analysis read. */
const analyses = new Map<string, RecolourRecipe | readonly Curve[]>();

const post = (response: LiveEffectsResponse, transfer: Transferable[] = []) =>
	self.postMessage(response, { transfer });

function index(sourceId: number, source: ImageData) {
	const image = {
		width: source.width,
		height: source.height,
		data: new Uint8Array(source.data.buffer, source.data.byteOffset, source.data.length)
	};
	const { colours, indices } = indexColours(image);
	loaded = { sourceId, image, colours };
	analyses.clear();
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
		const fits: AnalysedFit[] = [];
		const resolved = resolveRecipes(
			ditherette,
			loaded.image,
			effects,
			context,
			crop,
			analyses,
			fits
		);
		const { colours } = loaded;
		if (data.type === 'mask') {
			const results = compileEffectMask(ditherette, {
				version: 1,
				colours,
				effects: resolved,
				mask: data.mask,
				context
			});
			post({ type: 'masked', id, sourceId, key, results }, [results.buffer]);
			return;
		}
		const compiled = effects.length
			? compileEffects(ditherette, { version: 1, colours, effects: resolved, context })
			: colours;
		post({ type: 'compiled', id, sourceId, key, results: compiled, fits }, [compiled.buffer]);
	} catch (error) {
		const message = error instanceof Error ? error.message : 'Effects failed.';
		post({ type: data.type === 'mask' ? 'mask-failed' : 'failed', id, sourceId, key, message });
	}
};
