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

function hasPreservedTransparentPixels(source: Uint8ClampedArray, alphaThreshold: number) {
	for (let offset = 3; offset < source.length; offset += 4) {
		if (source[offset]! <= alphaThreshold) return true;
	}
	return false;
}

type ErrorScatter = (
	work: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	workOffset: number,
	reverse: boolean,
	error0: number,
	error1: number,
	error2: number,
	strengthMask: number
) => void;

function scatterForAlgorithm(
	algorithm: string,
	kernel: readonly (readonly [number, number, number])[]
): ErrorScatter {
	switch (algorithm) {
		case 'floyd-steinberg':
			return scatterFloyd;
		case 'sierra':
			return scatterSierra;
		case 'sierra-lite':
			return scatterSierraLite;
		default:
			return (
				work,
				width,
				height,
				x,
				y,
				workOffset,
				reverse,
				error0,
				error1,
				error2,
				strengthMask
			) =>
				scatterGeneric(
					work,
					width,
					height,
					x,
					y,
					workOffset,
					reverse,
					error0,
					error1,
					error2,
					strengthMask,
					kernel
				);
	}
}

function scatterFloyd(
	work: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	workOffset: number,
	reverse: boolean,
	error0: number,
	error1: number,
	error2: number,
	strengthMask: number
) {
	if (reverse) {
		if (x > 0) addError(work, workOffset - 3, error0, error1, error2, (7 / 16) * strengthMask);
		if (y + 1 < height) {
			const nextRow = workOffset + width * 3;
			if (x + 1 < width)
				addError(work, nextRow + 3, error0, error1, error2, (3 / 16) * strengthMask);
			addError(work, nextRow, error0, error1, error2, (5 / 16) * strengthMask);
			if (x > 0) addError(work, nextRow - 3, error0, error1, error2, (1 / 16) * strengthMask);
		}
		return;
	}
	if (x + 1 < width)
		addError(work, workOffset + 3, error0, error1, error2, (7 / 16) * strengthMask);
	if (y + 1 < height) {
		const nextRow = workOffset + width * 3;
		if (x > 0) addError(work, nextRow - 3, error0, error1, error2, (3 / 16) * strengthMask);
		addError(work, nextRow, error0, error1, error2, (5 / 16) * strengthMask);
		if (x + 1 < width) addError(work, nextRow + 3, error0, error1, error2, (1 / 16) * strengthMask);
	}
}

function scatterGeneric(
	work: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	_workOffset: number,
	reverse: boolean,
	error0: number,
	error1: number,
	error2: number,
	strengthMask: number,
	kernel: readonly (readonly [number, number, number])[]
) {
	for (const [dxBase, dy, weight] of kernel) {
		const dx = reverse ? -dxBase : dxBase;
		const xx = x + dx;
		const yy = y + dy;
		if (xx < 0 || xx >= width || yy < 0 || yy >= height) continue;
		addError(work, (yy * width + xx) * 3, error0, error1, error2, weight * strengthMask);
	}
}

function scatterSierra(
	work: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	workOffset: number,
	reverse: boolean,
	error0: number,
	error1: number,
	error2: number,
	strengthMask: number
) {
	if (reverse) {
		if (x > 0) addError(work, workOffset - 3, error0, error1, error2, (5 / 32) * strengthMask);
		if (x > 1) addError(work, workOffset - 6, error0, error1, error2, (3 / 32) * strengthMask);
		if (y + 1 < height) {
			const nextRow = workOffset + width * 3;
			if (x + 2 < width)
				addError(work, nextRow + 6, error0, error1, error2, (2 / 32) * strengthMask);
			if (x + 1 < width)
				addError(work, nextRow + 3, error0, error1, error2, (4 / 32) * strengthMask);
			addError(work, nextRow, error0, error1, error2, (5 / 32) * strengthMask);
			if (x > 0) addError(work, nextRow - 3, error0, error1, error2, (4 / 32) * strengthMask);
			if (x > 1) addError(work, nextRow - 6, error0, error1, error2, (2 / 32) * strengthMask);
		}
		if (y + 2 < height) {
			const next2Row = workOffset + width * 6;
			if (x + 1 < width)
				addError(work, next2Row + 3, error0, error1, error2, (2 / 32) * strengthMask);
			addError(work, next2Row, error0, error1, error2, (3 / 32) * strengthMask);
			if (x > 0) addError(work, next2Row - 3, error0, error1, error2, (2 / 32) * strengthMask);
		}
		return;
	}
	if (x + 1 < width)
		addError(work, workOffset + 3, error0, error1, error2, (5 / 32) * strengthMask);
	if (x + 2 < width)
		addError(work, workOffset + 6, error0, error1, error2, (3 / 32) * strengthMask);
	if (y + 1 < height) {
		const nextRow = workOffset + width * 3;
		if (x > 1) addError(work, nextRow - 6, error0, error1, error2, (2 / 32) * strengthMask);
		if (x > 0) addError(work, nextRow - 3, error0, error1, error2, (4 / 32) * strengthMask);
		addError(work, nextRow, error0, error1, error2, (5 / 32) * strengthMask);
		if (x + 1 < width) addError(work, nextRow + 3, error0, error1, error2, (4 / 32) * strengthMask);
		if (x + 2 < width) addError(work, nextRow + 6, error0, error1, error2, (2 / 32) * strengthMask);
	}
	if (y + 2 < height) {
		const next2Row = workOffset + width * 6;
		if (x > 0) addError(work, next2Row - 3, error0, error1, error2, (2 / 32) * strengthMask);
		addError(work, next2Row, error0, error1, error2, (3 / 32) * strengthMask);
		if (x + 1 < width)
			addError(work, next2Row + 3, error0, error1, error2, (2 / 32) * strengthMask);
	}
}

function scatterSierraLite(
	work: Float32Array,
	width: number,
	height: number,
	x: number,
	y: number,
	workOffset: number,
	reverse: boolean,
	error0: number,
	error1: number,
	error2: number,
	strengthMask: number
) {
	if (reverse) {
		if (x > 0) addError(work, workOffset - 3, error0, error1, error2, (2 / 4) * strengthMask);
		if (y + 1 < height) {
			const nextRow = workOffset + width * 3;
			if (x + 1 < width)
				addError(work, nextRow + 3, error0, error1, error2, (1 / 4) * strengthMask);
			addError(work, nextRow, error0, error1, error2, (1 / 4) * strengthMask);
		}
		return;
	}
	if (x + 1 < width) addError(work, workOffset + 3, error0, error1, error2, (2 / 4) * strengthMask);
	if (y + 1 < height) {
		const nextRow = workOffset + width * 3;
		if (x > 0) addError(work, nextRow - 3, error0, error1, error2, (1 / 4) * strengthMask);
		addError(work, nextRow, error0, error1, error2, (1 / 4) * strengthMask);
	}
}

function addError(
	work: Float32Array,
	target: number,
	error0: number,
	error1: number,
	error2: number,
	weight: number
) {
	work[target] += error0 * weight;
	work[target + 1] += error1 * weight;
	work[target + 2] += error2 * weight;
}

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
	const scatter = scatterForAlgorithm(settings.dither.algorithm, kernel);
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
	const checkTransparency =
		alphaMode === 'preserve' && hasPreservedTransparentPixels(source, alphaThreshold);

	for (let index = 0; index < width * height; index++) {
		throwIfCanceled(caches, index);
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
		throwIfCanceled(caches, 0);
		const reverse = settings.dither.serpentine && y % 2 === 1;
		const start = reverse ? width - 1 : 0;
		const end = reverse ? -1 : width;
		const step = reverse ? -1 : 1;
		for (let x = start; x !== end; x += step) {
			const index = y * width + x;
			if (checkTransparency) {
				const alpha = source[index * 4 + 3]!;
				if (alpha <= alphaThreshold) {
					indices[index] =
						transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
					continue;
				}
			}

			const workOffset = index * 3;
			const current0 = work[workOffset]!;
			const current1 = work[workOffset + 1]!;
			const current2 = work[workOffset + 2]!;
			const ordinal = vectorMatcher.nearestOrdinal(current0, current1, current2);
			if (ordinal === -1) throw new Error('No visible palette colors are enabled');
			indices[index] = vectorMatcher.indexForOrdinal(ordinal);
			const strengthMask = useAdaptivePlacement
				? strength *
					placementMask(source, width, height, x, y, settings, vectorSpace, sourceVectors)
				: strength;
			const error0 = current0 - vectorMatcher.vector0ForOrdinal(ordinal);
			const error1 = current1 - vectorMatcher.vector1ForOrdinal(ordinal);
			const error2 = current2 - vectorMatcher.vector2ForOrdinal(ordinal);
			scatter(work, width, height, x, y, workOffset, reverse, error0, error1, error2, strengthMask);
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
	const scatter = scatterForAlgorithm(settings.dither.algorithm, kernel);
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
	const checkTransparency =
		alphaMode === 'preserve' && hasPreservedTransparentPixels(source, alphaThreshold);

	if (alphaMode === 'preserve') {
		for (
			let index = 0, sourceOffset = 0, workOffset = 0;
			index < width * height;
			index++, sourceOffset += 4, workOffset += 3
		) {
			throwIfCanceled(caches, index);
			work[workOffset] = source[sourceOffset]!;
			work[workOffset + 1] = source[sourceOffset + 1]!;
			work[workOffset + 2] = source[sourceOffset + 2]!;
		}
	} else {
		for (
			let index = 0, sourceOffset = 0, workOffset = 0;
			index < width * height;
			index++, sourceOffset += 4, workOffset += 3
		) {
			throwIfCanceled(caches, index);
			const alpha = source[sourceOffset + 3]!;
			let r = source[sourceOffset]!;
			let g = source[sourceOffset + 1]!;
			let b = source[sourceOffset + 2]!;
			if (alpha !== 255) {
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
	}
	recordTiming(caches, 'quantize rgb diffusion work init', initStart);
	let nearestRgbCount = 0;
	const loopStart = performance.now();

	for (let y = 0; y < height; y++) {
		throwIfCanceled(caches, 0);
		const reverse = settings.dither.serpentine && y % 2 === 1;
		const start = reverse ? width - 1 : 0;
		const end = reverse ? -1 : width;
		const step = reverse ? -1 : 1;
		for (let x = start; x !== end; x += step) {
			const index = y * width + x;
			if (checkTransparency) {
				const alpha = source[index * 4 + 3]!;
				if (alpha <= alphaThreshold) {
					indices[index] =
						transparentIndexValue !== -1 ? transparentIndexValue : fallbackTransparentIndex;
					continue;
				}
			}

			const workOffset = index * 3;
			const r = clampByte(work[workOffset]!);
			const g = clampByte(work[workOffset + 1]!);
			const b = clampByte(work[workOffset + 2]!);
			nearestRgbCount++;
			const match = matcher.nearestIndexByteRgb(r, g, b);
			indices[index] = match;
			const strengthMask = useAdaptivePlacement
				? strength *
					placementMask(source, width, height, x, y, settings, vectorSpace, sourceVectors)
				: strength;
			const errorR = r - matcher.paletteRedAt(match);
			const errorG = g - matcher.paletteGreenAt(match);
			const errorB = b - matcher.paletteBlueAt(match);
			if (settings.dither.algorithm === 'floyd-steinberg') {
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
				scatter(
					work,
					width,
					height,
					x,
					y,
					workOffset,
					reverse,
					errorR,
					errorG,
					errorB,
					strengthMask
				);
			}
		}
	}
	caches?.recordCount?.('nearest rgb', nearestRgbCount);
	recordTiming(caches, 'quantize rgb diffusion dither+match loop', loopStart);
}

const CANCEL_CHECK_PIXELS = 8192;

function throwIfCanceled(caches: QuantizeAlgorithmContext['caches'], pixelIndex: number) {
	if (pixelIndex % CANCEL_CHECK_PIXELS !== 0) return;
	if (caches?.shouldCancel?.()) throw new Error('Processing was canceled.');
}
