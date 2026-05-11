import { bayerSizeForAlgorithm, normalizedBayerMatrix } from './bayer';
import { clampByte, vectorForRgb } from './color';
import { compositedRgb } from './compositing';
import {
	RGB_DITHER_NOISE_SCALE,
	colorSpaceThresholdIndexRgb,
	createPaletteVectorMatcher,
	createThresholdByteVectorMatcher,
	createThresholdRgbVectorMatcher,
	paletteVectorSpace,
	prepareQuantize,
	recordMatcherMemoStats,
	supportsVectorDither
} from './quantize-shared';
import type { EnabledPaletteColor, ProcessingSettings } from './types';

export type RowQuantizeJob = {
	id: number;
	width: number;
	height: number;
	startY: number;
	endY: number;
	settings: ProcessingSettings;
	palette: EnabledPaletteColor[];
	sourceBuffer?: SharedArrayBuffer;
	sourceRows?: Uint8ClampedArray;
	outputBuffer?: SharedArrayBuffer;
	cancelBuffer?: SharedArrayBuffer;
	randomStepStart?: number;
};

export type RowQuantizeDone = {
	id: number;
	type: 'done';
	startY: number;
	endY: number;
	indices?: Uint8Array;
	counts: Record<string, number>;
};

type RowQuantizeFailure = {
	id: number;
	type: 'error';
	message: string;
};

type RowQuantizeResponse = RowQuantizeDone | RowQuantizeFailure;

const workerSelf =
	typeof self === 'undefined'
		? undefined
		: (self as unknown as {
				onmessage: ((event: MessageEvent<RowQuantizeJob>) => void) | null;
				postMessage(message: RowQuantizeResponse): void;
			});

if (workerSelf) {
	workerSelf.onmessage = (event: MessageEvent<RowQuantizeJob>) => {
		try {
			workerSelf.postMessage(runRowQuantizeJob(event.data) satisfies RowQuantizeResponse);
		} catch (error) {
			workerSelf.postMessage({
				id: event.data.id,
				type: 'error',
				message: error instanceof Error ? error.message : 'Row quantization failed.'
			} satisfies RowQuantizeResponse);
		}
	};
}

export function runRowQuantizeJob(job: RowQuantizeJob): RowQuantizeDone {
	const counts: Record<string, number> = {};
	const caches = {
		recordCount(name: string, amount = 1) {
			counts[name] = (counts[name] ?? 0) + amount;
		}
	};
	const prepared = prepareQuantize(job.palette, job.settings, caches);
	if ('indices' in prepared) {
		const rowPixels = (job.endY - job.startY) * job.width;
		const indices = job.outputBuffer
			? undefined
			: new Uint8Array(rowPixels).fill(prepared.transparentIndex);
		if (job.outputBuffer) {
			new Uint8Array(job.outputBuffer).fill(
				prepared.transparentIndex,
				job.startY * job.width,
				job.endY * job.width
			);
		}
		return { id: job.id, type: 'done', startY: job.startY, endY: job.endY, indices, counts };
	}

	const source = job.sourceBuffer
		? new Uint8ClampedArray(job.sourceBuffer)
		: (job.sourceRows ?? new Uint8ClampedArray());
	const output = job.outputBuffer
		? new Uint8Array(job.outputBuffer)
		: new Uint8Array((job.endY - job.startY) * job.width);
	const cancel = job.cancelBuffer ? new Int32Array(job.cancelBuffer) : undefined;
	const bayerSize = bayerSizeForAlgorithm(job.settings.dither.algorithm);
	const bayer = bayerSize ? normalizedBayerMatrix(bayerSize) : undefined;
	const bayerMask = bayerSize ? bayerSize - 1 : 0;
	const bayerShift =
		bayerSize === 2 ? 1 : bayerSize === 4 ? 2 : bayerSize === 8 ? 3 : bayerSize === 16 ? 4 : 0;
	const strength = prepared.strength;
	const useBayer = Boolean(bayer && bayerSize && strength > 0);
	const useRandom = job.settings.dither.algorithm === 'random' && strength > 0;
	const useVectorDither = (useBayer || useRandom) && supportsVectorDither(job.settings);
	const vectorSpace = paletteVectorSpace(
		prepared.matcher,
		job.settings,
		prepared.paletteCacheKey,
		caches
	);
	const vectorMatcher = createPaletteVectorMatcher(vectorSpace, job.settings, caches);
	const thresholdVectorMatcher =
		useBayer && bayer && useVectorDither
			? (createThresholdByteVectorMatcher(
					vectorSpace,
					job.settings.colorSpace,
					bayer,
					strength,
					job.width * job.height,
					caches
				) ??
				createThresholdRgbVectorMatcher(
					vectorSpace,
					job.settings,
					bayer,
					strength,
					job.width * job.height,
					caches
				))
			: undefined;
	const alphaMode = job.settings.output.alphaMode;
	const alphaThreshold = job.settings.output.alphaThreshold;
	const noiseScale = RGB_DITHER_NOISE_SCALE * strength;
	let randomStep = job.randomStepStart ?? 0;
	let nearestRgbCount = 0;

	for (let y = job.startY; y < job.endY; y++) {
		if (cancel && Atomics.load(cancel, 0)) throw new Error('Processing was canceled.');
		const localRow = y - job.startY;
		const sourceRow = job.sourceBuffer ? y * job.width : localRow * job.width;
		const outputRow = job.outputBuffer ? y * job.width : localRow * job.width;
		const bayerRow = bayerSize ? (y & bayerMask) << bayerShift : 0;
		for (let x = 0; x < job.width; x++) {
			const sourceIndex = sourceRow + x;
			const sourceOffset = sourceIndex * 4;
			const outputIndex = outputRow + x;
			const alpha = source[sourceOffset + 3]!;
			if (alphaMode === 'preserve' && alpha <= alphaThreshold) {
				output[outputIndex] =
					prepared.transparentIndexValue !== -1
						? prepared.transparentIndexValue
						: prepared.fallbackTransparentIndex;
				continue;
			}

			let r = source[sourceOffset]!;
			let g = source[sourceOffset + 1]!;
			let b = source[sourceOffset + 2]!;
			if (alphaMode !== 'preserve' && alpha !== 255) {
				const rgb = compositedRgb({ r, g, b }, alpha, alphaMode, prepared.matte);
				r = rgb.r;
				g = rgb.g;
				b = rgb.b;
			}

			if (useBayer && bayer && bayerSize) {
				const thresholdIndex = bayerRow + (x & bayerMask);
				if (useVectorDither && thresholdVectorMatcher) {
					output[outputIndex] = thresholdVectorMatcher.nearestIndexRgb(r, g, b, thresholdIndex);
					continue;
				}
				const threshold = bayer[thresholdIndex]!;
				if (useVectorDither) {
					const match = colorSpaceThresholdIndexRgb(
						r,
						g,
						b,
						threshold,
						vectorSpace,
						vectorMatcher,
						job.settings,
						strength
					);
					output[outputIndex] =
						match === -1 ? prepared.matcher.nearestIndexByteRgb(r, g, b) : match;
					continue;
				}
				r = clampByte(r + threshold * noiseScale);
				g = clampByte(g + threshold * noiseScale);
				b = clampByte(b + threshold * noiseScale);
			} else if (useRandom) {
				randomStep++;
				const noise = randomNoise(job.settings.dither.seed, randomStep);
				if (useVectorDither) {
					const match = colorSpaceThresholdIndexRgb(
						r,
						g,
						b,
						noise,
						vectorSpace,
						vectorMatcher,
						job.settings,
						strength
					);
					output[outputIndex] =
						match === -1 ? prepared.matcher.nearestIndexByteRgb(r, g, b) : match;
					continue;
				}
				r = clampByte(r + noise * noiseScale);
				g = clampByte(g + noise * noiseScale);
				b = clampByte(b + noise * noiseScale);
			} else if (supportsVectorDither(job.settings)) {
				const vector = vectorForRgb(r, g, b, job.settings.colorSpace);
				output[outputIndex] = vectorMatcher.nearestIndex(vector[0], vector[1], vector[2]);
				continue;
			}

			nearestRgbCount++;
			output[outputIndex] = prepared.matcher.nearestIndexByteRgb(r, g, b);
		}
	}

	if (nearestRgbCount) counts['nearest rgb'] = (counts['nearest rgb'] ?? 0) + nearestRgbCount;
	vectorMatcher.flushCounts();
	thresholdVectorMatcher?.flushCounts();
	recordMatcherMemoStats(prepared.matcher, caches);
	return {
		id: job.id,
		type: 'done',
		startY: job.startY,
		endY: job.endY,
		indices: job.outputBuffer ? undefined : output,
		counts
	};
}

function randomNoise(seed: number, step: number) {
	let randomValue = (seed + Math.imul(step, 0x6d2b79f5)) >>> 0;
	randomValue = Math.imul(randomValue ^ (randomValue >>> 15), randomValue | 1);
	randomValue ^= randomValue + Math.imul(randomValue ^ (randomValue >>> 7), randomValue | 61);
	return ((randomValue ^ (randomValue >>> 14)) >>> 0) / 4294967296 - 0.5;
}
