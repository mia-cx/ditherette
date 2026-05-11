import { quantizeAlgorithmFor } from './quantize-algorithms/registry';
import {
	prepareQuantize,
	recordMatcherMemoStats,
	type QuantizeCaches,
	type QuantizeResult
} from './quantize-shared';
import type { EnabledPaletteColor, ProcessingSettings } from './types';

export {
	prepareQuantizeColorSpace,
	type ColorVector,
	type ColorVectorImage,
	type PaletteVector,
	type PaletteVectorSpace,
	type QuantizeCaches,
	type QuantizeCounterName,
	type QuantizeDiagnosticsSink,
	type QuantizeResult,
	type QuantizeTimingName
} from './quantize-shared';

export function quantizeImage(
	image: ImageData,
	palette: EnabledPaletteColor[],
	settings: ProcessingSettings,
	caches?: QuantizeCaches
): QuantizeResult {
	const prepared = prepareQuantize(palette, settings, caches);
	if ('indices' in prepared) {
		return {
			...prepared,
			indices: new Uint8Array(image.width * image.height).fill(prepared.transparentIndex)
		};
	}
	const {
		warnings,
		nextPalette,
		transparentIndexValue,
		matcher,
		matte,
		paletteCacheKey,
		fallbackTransparentIndex,
		strength
	} = prepared;
	const indices = new Uint8Array(image.width * image.height);

	quantizeAlgorithmFor(settings.dither.algorithm).quantize({
		image,
		indices,
		matcher,
		settings,
		matte,
		transparentIndexValue,
		fallbackTransparentIndex,
		strength,
		paletteCacheKey,
		caches
	});
	recordMatcherMemoStats(matcher, caches);

	return { indices, palette: nextPalette, transparentIndex: transparentIndexValue, warnings };
}
