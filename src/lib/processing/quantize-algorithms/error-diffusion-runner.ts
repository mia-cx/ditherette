import { clampByte } from '../color';
import {
	cachedColorVectorImage,
	createPaletteVectorMatcher,
	paletteVectorSpace,
	placementMask,
	recordTiming,
	supportsVectorDither,
	usesAdaptivePlacement,
	vectorForCompositedPixel
} from '../quantize-shared';
import { errorKernelForAlgorithm } from './error-kernels';
import type { QuantizeAlgorithmContext } from './types';

function quantizeVectorErrorDiffusion(context: QuantizeAlgorithmContext) {
	const {
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
	} = context;

	const kernel = errorKernelForAlgorithm(settings.dither.algorithm);
	if (!kernel)
		throw new Error(`Unsupported error diffusion algorithm: ${settings.dither.algorithm}`);
	const width = image.width;
	const height = image.height;
	const source = image.data;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const initStart = performance.now();
	const work = new Float32Array(width * height * 3);
	const vectorSpace = paletteVectorSpace(matcher, settings, paletteCacheKey, caches);
	const vectorMatcher = createPaletteVectorMatcher(vectorSpace, settings, caches);
	const compositedVectors = cachedColorVectorImage(image, settings, matte, 'composited', caches);
	const useAdaptivePlacement = usesAdaptivePlacement(settings);
	const sourceVectors = useAdaptivePlacement
		? cachedColorVectorImage(image, settings, matte, 'source', caches)
		: undefined;

	for (let index = 0; index < width * height; index++) {
		const sourceOffset = index * 4;
		const workOffset = index * 3;
		if (compositedVectors) {
			work[workOffset] = compositedVectors.v0[index]!;
			work[workOffset + 1] = compositedVectors.v1[index]!;
			work[workOffset + 2] = compositedVectors.v2[index]!;
		} else {
			const vector = vectorForCompositedPixel(source, sourceOffset, settings, matte);
			work[workOffset] = vector[0];
			work[workOffset + 1] = vector[1];
			work[workOffset + 2] = vector[2];
		}
	}
	recordTiming(caches, 'quantize vector diffusion work init', initStart);
	const loopStart = performance.now();

	for (let y = 0; y < height; y++) {
		const reverse = settings.dither.serpentine && y % 2 === 1;
		const start = reverse ? width - 1 : 0;
		const end = reverse ? -1 : width;
		const step = reverse ? -1 : 1;
		for (let x = start; x !== end; x += step) {
			const index = y * width + x;
			const sourceOffset = index * 4;
			const alpha = source[sourceOffset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			const workOffset = index * 3;
			const current0 = work[workOffset]!;
			const current1 = work[workOffset + 1]!;
			const current2 = work[workOffset + 2]!;
			const ordinal = vectorMatcher.nearestOrdinal(current0, current1, current2);
			if (ordinal === -1) throw new Error('No visible palette colors are enabled');
			indices[index] = vectorMatcher.indexForOrdinal(ordinal);
			const mask = useAdaptivePlacement
				? placementMask(source, width, height, x, y, settings, vectorSpace, sourceVectors)
				: 1;
			const error0 = current0 - vectorMatcher.vector0ForOrdinal(ordinal);
			const error1 = current1 - vectorMatcher.vector1ForOrdinal(ordinal);
			const error2 = current2 - vectorMatcher.vector2ForOrdinal(ordinal);
			if (settings.dither.algorithm === 'floyd-steinberg') {
				const strengthMask = strength * mask;
				if (reverse) {
					if (x > 0) {
						const target = workOffset - 3;
						const scaledWeight = (7 / 16) * strengthMask;
						work[target] += error0 * scaledWeight;
						work[target + 1] += error1 * scaledWeight;
						work[target + 2] += error2 * scaledWeight;
					}
					if (y + 1 < height) {
						const nextRow = workOffset + width * 3;
						if (x + 1 < width) {
							const target = nextRow + 3;
							const scaledWeight = (3 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
						{
							const target = nextRow;
							const scaledWeight = (5 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
						if (x > 0) {
							const target = nextRow - 3;
							const scaledWeight = (1 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
					}
				} else {
					if (x + 1 < width) {
						const target = workOffset + 3;
						const scaledWeight = (7 / 16) * strengthMask;
						work[target] += error0 * scaledWeight;
						work[target + 1] += error1 * scaledWeight;
						work[target + 2] += error2 * scaledWeight;
					}
					if (y + 1 < height) {
						const nextRow = workOffset + width * 3;
						if (x > 0) {
							const target = nextRow - 3;
							const scaledWeight = (3 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
						{
							const target = nextRow;
							const scaledWeight = (5 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
						if (x + 1 < width) {
							const target = nextRow + 3;
							const scaledWeight = (1 / 16) * strengthMask;
							work[target] += error0 * scaledWeight;
							work[target + 1] += error1 * scaledWeight;
							work[target + 2] += error2 * scaledWeight;
						}
					}
				}
			} else {
				for (const [dxBase, dy, weight] of kernel) {
					const dx = reverse ? -dxBase : dxBase;
					const xx = x + dx;
					const yy = y + dy;
					if (xx < 0 || xx >= width || yy < 0 || yy >= height) continue;
					const target = (yy * width + xx) * 3;
					const scaledWeight = weight * strength * mask;
					work[target] += error0 * scaledWeight;
					work[target + 1] += error1 * scaledWeight;
					work[target + 2] += error2 * scaledWeight;
				}
			}
		}
	}
	vectorMatcher.flushCounts();
	recordTiming(caches, 'quantize vector diffusion dither+match loop', loopStart);
}

export function quantizeErrorDiffusion(context: QuantizeAlgorithmContext) {
	const {
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
	} = context;

	if (supportsVectorDither(settings)) {
		quantizeVectorErrorDiffusion(context);
		return;
	}

	const kernel = errorKernelForAlgorithm(settings.dither.algorithm);
	if (!kernel)
		throw new Error(`Unsupported error diffusion algorithm: ${settings.dither.algorithm}`);
	const width = image.width;
	const height = image.height;
	const source = image.data;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const initStart = performance.now();
	const work = new Float32Array(width * height * 3);
	const vectorSpace = paletteVectorSpace(matcher, settings, paletteCacheKey, caches);
	const useAdaptivePlacement = usesAdaptivePlacement(settings);
	const sourceVectors = useAdaptivePlacement
		? cachedColorVectorImage(image, settings, matte, 'source', caches)
		: undefined;

	for (
		let index = 0, sourceOffset = 0, workOffset = 0;
		index < width * height;
		index++, sourceOffset += 4, workOffset += 3
	) {
		const alpha = source[sourceOffset + 3]!;
		let r = source[sourceOffset]!;
		let g = source[sourceOffset + 1]!;
		let b = source[sourceOffset + 2]!;
		if (alphaMode !== 'preserve' && alpha !== 255) {
			const opacity = alpha / 255;
			if (alphaMode === 'premultiplied') {
				r = Math.round(r * opacity);
				g = Math.round(g * opacity);
				b = Math.round(b * opacity);
			} else {
				const background = 1 - opacity;
				r = Math.round(r * opacity + matte.r * background);
				g = Math.round(g * opacity + matte.g * background);
				b = Math.round(b * opacity + matte.b * background);
			}
		}
		work[workOffset] = r;
		work[workOffset + 1] = g;
		work[workOffset + 2] = b;
	}
	recordTiming(caches, 'quantize rgb diffusion work init', initStart);
	let nearestRgbCount = 0;
	const loopStart = performance.now();

	for (let y = 0; y < height; y++) {
		const reverse = settings.dither.serpentine && y % 2 === 1;
		const start = reverse ? width - 1 : 0;
		const end = reverse ? -1 : width;
		const step = reverse ? -1 : 1;
		for (let x = start; x !== end; x += step) {
			const index = y * width + x;
			const sourceOffset = index * 4;
			const alpha = source[sourceOffset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			const workOffset = index * 3;
			const r = clampByte(work[workOffset]!);
			const g = clampByte(work[workOffset + 1]!);
			const b = clampByte(work[workOffset + 2]!);
			nearestRgbCount++;
			const match = matcher.nearestIndexByteRgb(r, g, b);
			indices[index] = match;
			const mask = useAdaptivePlacement
				? placementMask(source, width, height, x, y, settings, vectorSpace, sourceVectors)
				: 1;
			const errorR = r - matcher.paletteRedAt(match);
			const errorG = g - matcher.paletteGreenAt(match);
			const errorB = b - matcher.paletteBlueAt(match);
			if (settings.dither.algorithm === 'floyd-steinberg') {
				const strengthMask = strength * mask;
				if (reverse) {
					if (x > 0) {
						const target = workOffset - 3;
						const scaledWeight = (7 / 16) * strengthMask;
						work[target] += errorR * scaledWeight;
						work[target + 1] += errorG * scaledWeight;
						work[target + 2] += errorB * scaledWeight;
					}
					if (y + 1 < height) {
						const nextRow = workOffset + width * 3;
						if (x + 1 < width) {
							const target = nextRow + 3;
							const scaledWeight = (3 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
						{
							const target = nextRow;
							const scaledWeight = (5 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
						if (x > 0) {
							const target = nextRow - 3;
							const scaledWeight = (1 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
					}
				} else {
					if (x + 1 < width) {
						const target = workOffset + 3;
						const scaledWeight = (7 / 16) * strengthMask;
						work[target] += errorR * scaledWeight;
						work[target + 1] += errorG * scaledWeight;
						work[target + 2] += errorB * scaledWeight;
					}
					if (y + 1 < height) {
						const nextRow = workOffset + width * 3;
						if (x > 0) {
							const target = nextRow - 3;
							const scaledWeight = (3 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
						{
							const target = nextRow;
							const scaledWeight = (5 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
						if (x + 1 < width) {
							const target = nextRow + 3;
							const scaledWeight = (1 / 16) * strengthMask;
							work[target] += errorR * scaledWeight;
							work[target + 1] += errorG * scaledWeight;
							work[target + 2] += errorB * scaledWeight;
						}
					}
				}
			} else {
				for (const [dxBase, dy, weight] of kernel) {
					const dx = reverse ? -dxBase : dxBase;
					const xx = x + dx;
					const yy = y + dy;
					if (xx < 0 || xx >= width || yy < 0 || yy >= height) continue;
					const target = (yy * width + xx) * 3;
					const scaledWeight = weight * strength * mask;
					work[target] += errorR * scaledWeight;
					work[target + 1] += errorG * scaledWeight;
					work[target + 2] += errorB * scaledWeight;
				}
			}
		}
	}
	caches?.recordCount?.('nearest rgb', nearestRgbCount);
	recordTiming(caches, 'quantize rgb diffusion dither+match loop', loopStart);
}
