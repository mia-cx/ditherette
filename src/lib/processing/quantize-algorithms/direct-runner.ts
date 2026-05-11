import { bayerSizeForAlgorithm, normalizedBayerMatrix } from '../bayer';
import { clampByte } from '../color';
import {
	RGB_DITHER_NOISE_SCALE,
	cachedColorVectorImage,
	colorSpaceThresholdIndexRgb,
	colorSpaceThresholdIndexValues,
	createPaletteVectorMatcher,
	createThresholdByteVectorMatcher,
	createThresholdRgbVectorMatcher,
	paletteVectorSpace,
	placementMask,
	recordTiming,
	supportsCachedVectorMatching,
	supportsVectorDither,
	usesAdaptivePlacement
} from '../quantize-shared';
import type { QuantizeAlgorithmContext } from './types';

export function quantizeDirect(context: QuantizeAlgorithmContext) {
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

	let randomState = settings.dither.seed >>> 0;
	const bayerSize = bayerSizeForAlgorithm(settings.dither.algorithm);
	const bayer = bayerSize ? normalizedBayerMatrix(bayerSize) : undefined;
	const bayerMask = bayerSize ? bayerSize - 1 : 0;
	const bayerShift =
		bayerSize === 2 ? 1 : bayerSize === 4 ? 2 : bayerSize === 8 ? 3 : bayerSize === 16 ? 4 : 0;
	const source = image.data;
	const pixels = image.width * image.height;
	const alphaMode = settings.output.alphaMode;
	const alphaThreshold = settings.output.alphaThreshold;
	const useBayer = Boolean(bayer && bayerSize && strength > 0);
	const useRandom = settings.dither.algorithm === 'random' && strength > 0;
	const useVectorDither = (useBayer || useRandom) && supportsVectorDither(settings);
	const noiseScale = RGB_DITHER_NOISE_SCALE * strength;
	const vectorSpace = paletteVectorSpace(matcher, settings, paletteCacheKey, caches);
	const vectorMatcher = createPaletteVectorMatcher(vectorSpace, settings, caches);
	const useAdaptivePlacement = usesAdaptivePlacement(settings);
	const thresholdVectorMatcher =
		useBayer && bayer && useVectorDither && !useAdaptivePlacement
			? (createThresholdByteVectorMatcher(
					vectorSpace,
					settings.colorSpace,
					bayer,
					strength,
					pixels,
					caches
				) ??
				createThresholdRgbVectorMatcher(vectorSpace, settings, bayer, strength, pixels, caches))
			: undefined;
	const needsCompositedVectors =
		supportsCachedVectorMatching(settings.colorSpace) && useVectorDither && !thresholdVectorMatcher;
	const compositedVectors = needsCompositedVectors
		? cachedColorVectorImage(image, settings, matte, 'composited', caches)
		: undefined;
	const sourceVectors =
		(useBayer || useRandom) && useAdaptivePlacement
			? cachedColorVectorImage(image, settings, matte, 'source', caches)
			: undefined;
	const useDirectVectorMatch = Boolean(compositedVectors && !useBayer && !useRandom);
	let nearestRgbCount = 0;
	const loopStart = performance.now();

	if (!useBayer && !useRandom) {
		if (useDirectVectorMatch && compositedVectors) {
			for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
				const alpha = source[offset + 3]!;
				if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
					indices[index] =
						transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
					continue;
				}
				indices[index] = vectorMatcher.nearestIndex(
					compositedVectors.v0[index]!,
					compositedVectors.v1[index]!,
					compositedVectors.v2[index]!
				);
			}
			vectorMatcher.flushCounts();
			recordTiming(caches, 'quantize direct dither+match loop', loopStart);
			return;
		}

		for (let index = 0, offset = 0; index < pixels; index++, offset += 4) {
			const alpha = source[offset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			let r = source[offset]!;
			let g = source[offset + 1]!;
			let b = source[offset + 2]!;
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
			nearestRgbCount++;
			indices[index] = matcher.nearestIndexByteRgb(r, g, b);
		}
		caches?.recordCount?.('nearest rgb', nearestRgbCount);
		recordTiming(caches, 'quantize direct dither+match loop', loopStart);
		return;
	}

	for (let y = 0; y < image.height; y++) {
		const rowOffset = y * image.width;
		const bayerRow = bayerSize ? (y & bayerMask) << bayerShift : 0;
		for (let x = 0; x < image.width; x++) {
			const index = rowOffset + x;
			const offset = index * 4;
			const alpha = source[offset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				indices[index] =
					transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
				continue;
			}

			let r = source[offset]!;
			let g = source[offset + 1]!;
			let b = source[offset + 2]!;
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

			if (useDirectVectorMatch && compositedVectors) {
				indices[index] = vectorMatcher.nearestIndex(
					compositedVectors.v0[index]!,
					compositedVectors.v1[index]!,
					compositedVectors.v2[index]!
				);
				continue;
			}

			if (useBayer && bayer && bayerSize) {
				const thresholdIndex = bayerRow + (x & bayerMask);
				if (useVectorDither && thresholdVectorMatcher) {
					indices[index] = thresholdVectorMatcher.nearestIndexRgb(r, g, b, thresholdIndex);
					continue;
				}
				const threshold = bayer[thresholdIndex]!;
				const mask = useAdaptivePlacement
					? placementMask(
							source,
							image.width,
							image.height,
							x,
							y,
							settings,
							vectorSpace,
							sourceVectors
						)
					: 1;
				if (useVectorDither) {
					const ditherStrength = strength * mask;
					const match = compositedVectors
						? colorSpaceThresholdIndexValues(
								compositedVectors.v0[index]!,
								compositedVectors.v1[index]!,
								compositedVectors.v2[index]!,
								threshold,
								vectorSpace,
								vectorMatcher,
								ditherStrength
							)
						: colorSpaceThresholdIndexRgb(
								r,
								g,
								b,
								threshold,
								vectorSpace,
								vectorMatcher,
								settings,
								ditherStrength
							);
					indices[index] = match === -1 ? matcher.nearestIndexByteRgb(r, g, b) : match;
					continue;
				}
				r = clampByte(r + threshold * noiseScale * mask);
				g = clampByte(g + threshold * noiseScale * mask);
				b = clampByte(b + threshold * noiseScale * mask);
			} else if (useRandom) {
				const mask = useAdaptivePlacement
					? placementMask(
							source,
							image.width,
							image.height,
							x,
							y,
							settings,
							vectorSpace,
							sourceVectors
						)
					: 1;
				randomState = (randomState + 0x6d2b79f5) >>> 0;
				let randomValue = randomState;
				randomValue = Math.imul(randomValue ^ (randomValue >>> 15), randomValue | 1);
				randomValue ^= randomValue + Math.imul(randomValue ^ (randomValue >>> 7), randomValue | 61);
				const noise = ((randomValue ^ (randomValue >>> 14)) >>> 0) / 4294967296 - 0.5;
				if (useVectorDither) {
					const ditherStrength = strength * mask;
					const match = compositedVectors
						? colorSpaceThresholdIndexValues(
								compositedVectors.v0[index]!,
								compositedVectors.v1[index]!,
								compositedVectors.v2[index]!,
								noise,
								vectorSpace,
								vectorMatcher,
								ditherStrength
							)
						: colorSpaceThresholdIndexRgb(
								r,
								g,
								b,
								noise,
								vectorSpace,
								vectorMatcher,
								settings,
								ditherStrength
							);
					indices[index] = match === -1 ? matcher.nearestIndexByteRgb(r, g, b) : match;
					continue;
				}
				r = clampByte(r + noise * noiseScale * mask);
				g = clampByte(g + noise * noiseScale * mask);
				b = clampByte(b + noise * noiseScale * mask);
			}
			nearestRgbCount++;
			indices[index] = matcher.nearestIndexByteRgb(r, g, b);
		}
	}
	caches?.recordCount?.('nearest rgb', nearestRgbCount);
	vectorMatcher.flushCounts();
	thresholdVectorMatcher?.flushCounts();
	recordTiming(caches, 'quantize direct dither+match loop', loopStart);
}
