import type {
	Curve,
	Ditherette,
	Effect,
	EffectContext,
	RecolourRecipe,
	Rgba8Image
} from 'ditherette';
import { croppedSource } from './package-adapter';
import type { CropRect } from './types';

/** One palette fit's resolved curves, by its position in the input. */
export type AnalysedFit = { readonly index: number; readonly curves: readonly Curve[] };

/**
 * Give each recipe-less recolour and curves-less palette fit the data `process` would derive:
 * analysed on the cropped source, after the steps before it. `analyses` caches each analysis by
 * what it read, for one image. When `fits` is passed, it receives every palette-fit step's
 * resolved curves, the analysed list or its own explicit list, so the UI can show them.
 */
export function resolveRecipes(
	ditherette: Ditherette,
	image: Rgba8Image,
	effects: readonly Effect[],
	context: Required<EffectContext>,
	crop: CropRect | undefined,
	analyses = new Map<string, RecolourRecipe | readonly Curve[]>(),
	fits?: AnalysedFit[]
): Effect[] {
	return effects.map((step, index) => {
		const preceding = effects.slice(0, index);
		if (step.effect === 'recolour') {
			if (step.recipe !== null) return step;
			const key = JSON.stringify(['recipe', preceding, context, crop ?? null]);
			let recipe = analyses.get(key) as RecolourRecipe | undefined;
			if (!recipe) {
				const source = croppedSource(image, crop);
				recipe = ditherette.analyzeRecolour({ version: 1, source, effects: preceding, context });
				analyses.set(key, recipe);
			}
			return { ...step, recipe };
		}
		if (step.effect !== 'palette-fit') return step;
		if (step.curves !== null) {
			fits?.push({ index, curves: step.curves });
			return step;
		}
		// Analysis reads the image reaching the step, the palette, and the step's own space.
		const key = JSON.stringify([
			'fit',
			preceding,
			context.palette,
			crop ?? null,
			step.space,
			step.look
		]);
		let curves = analyses.get(key) as readonly Curve[] | undefined;
		if (!curves) {
			const source = croppedSource(image, crop);
			curves = ditherette.analyzePaletteFit({
				version: 1,
				source,
				effects: preceding,
				look: step.look,
				space: step.space,
				context: { palette: context.palette }
			});
			analyses.set(key, curves);
		}
		fits?.push({ index, curves });
		return { ...step, curves: [...curves] };
	});
}
