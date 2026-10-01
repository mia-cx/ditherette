import type { Ditherette, Effect, EffectContext, RecolourRecipe, Rgba8Image } from 'ditherette';
import { croppedSource } from './package-adapter';
import type { CropRect } from './types';

/**
 * Give each recipe-less palette fit the recipe `process` would derive: analysed on the cropped
 * source, after the steps before it. `recipes` caches each analysis by what it read, for one image.
 */
export function resolveRecipes(
	ditherette: Ditherette,
	image: Rgba8Image,
	effects: readonly Effect[],
	context: Required<EffectContext>,
	crop: CropRect | undefined,
	recipes = new Map<string, RecolourRecipe>()
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
