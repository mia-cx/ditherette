import type { CropRect, FitMode, ResizeId } from './types';
import { unpremultiplySample } from './compositing';
import { assertOutputDimensions, assertSourceDimensions } from './schemas';

type Rect = { x: number; y: number; width: number; height: number };
type ResizePlan = { source: Rect; target: Rect };

function finiteRectValue(value: number, label: string) {
	if (!Number.isFinite(value)) throw new Error(`${label} must be finite.`);
	return value;
}

function clampCrop(sourceWidth: number, sourceHeight: number, crop?: CropRect): Rect {
	if (!crop) return { x: 0, y: 0, width: sourceWidth, height: sourceHeight };
	finiteRectValue(crop.x, 'Crop x');
	finiteRectValue(crop.y, 'Crop y');
	finiteRectValue(crop.width, 'Crop width');
	finiteRectValue(crop.height, 'Crop height');
	const x = Math.min(sourceWidth - 1, Math.max(0, crop.x));
	const y = Math.min(sourceHeight - 1, Math.max(0, crop.y));
	return {
		x,
		y,
		width: Math.max(1, Math.min(sourceWidth - x, crop.width)),
		height: Math.max(1, Math.min(sourceHeight - y, crop.height))
	};
}

function resizePlan(
	sourceWidth: number,
	sourceHeight: number,
	outWidth: number,
	outHeight: number,
	fit: FitMode,
	crop?: CropRect
): ResizePlan {
	assertOutputDimensions(outWidth, outHeight, 'Resize output');
	assertSourceDimensions(sourceWidth, sourceHeight, 'Resize source');
	const base = clampCrop(sourceWidth, sourceHeight, crop);
	const fullTarget = { x: 0, y: 0, width: outWidth, height: outHeight };
	if (fit === 'stretch') return { source: base, target: fullTarget };

	const sourceAspect = base.width / base.height;
	const outputAspect = outWidth / outHeight;

	if (fit === 'cover') {
		if (sourceAspect < outputAspect) {
			const height = base.width / outputAspect;
			return {
				source: { x: base.x, y: base.y + (base.height - height) / 2, width: base.width, height },
				target: fullTarget
			};
		}
		const width = base.height * outputAspect;
		return {
			source: { x: base.x + (base.width - width) / 2, y: base.y, width, height: base.height },
			target: fullTarget
		};
	}

	if (sourceAspect > outputAspect) {
		const height = outWidth / sourceAspect;
		return {
			source: base,
			target: { x: 0, y: (outHeight - height) / 2, width: outWidth, height }
		};
	}

	const width = outHeight * sourceAspect;
	return {
		source: base,
		target: { x: (outWidth - width) / 2, y: 0, width, height: outHeight }
	};
}

function getPixel(data: Uint8ClampedArray, width: number, height: number, x: number, y: number) {
	const xx = Math.min(width - 1, Math.max(0, x));
	const yy = Math.min(height - 1, Math.max(0, y));
	const offset = (yy * width + xx) * 4;
	return [data[offset], data[offset + 1], data[offset + 2], data[offset + 3]] as const;
}

function sinc(value: number) {
	if (value === 0) return 1;
	const x = Math.PI * value;
	return Math.sin(x) / x;
}

function lanczos(value: number, radius = 3) {
	const absolute = Math.abs(value);
	if (absolute >= radius) return 0;
	return sinc(absolute) * sinc(absolute / radius);
}

export function resizeImageData(
	source: ImageData,
	outWidth: number,
	outHeight: number,
	fit: FitMode,
	mode: ResizeId,
	crop?: CropRect
): ImageData {
	const { width, height } = assertOutputDimensions(outWidth, outHeight, 'Resize output');
	if (fit !== 'stretch' && fit !== 'contain' && fit !== 'cover') {
		throw new Error(`Unsupported fit mode: ${fit satisfies never}`);
	}
	const plan = resizePlan(source.width, source.height, width, height, fit, crop);
	const output = new ImageData(width, height);
	const targetLeft = Math.max(0, Math.round(plan.target.x));
	const targetTop = Math.max(0, Math.round(plan.target.y));
	const targetRight = Math.min(width, Math.round(plan.target.x + plan.target.width));
	const targetBottom = Math.min(height, Math.round(plan.target.y + plan.target.height));
	const targetWidth = Math.max(1, targetRight - targetLeft);
	const targetHeight = Math.max(1, targetBottom - targetTop);
	const scaleX = plan.source.width / targetWidth;
	const scaleY = plan.source.height / targetHeight;

	if (mode === 'lanczos3') {
		resizeLanczos3(source, output, plan.source, {
			left: targetLeft,
			top: targetTop,
			right: targetRight,
			bottom: targetBottom,
			width: targetWidth,
			height: targetHeight
		});
		return output;
	}

	for (let y = targetTop; y < targetBottom; y++) {
		for (let x = targetLeft; x < targetRight; x++) {
			const targetOffset = (y * width + x) * 4;
			const sourceX = plan.source.x + (x - targetLeft + 0.5) * scaleX - 0.5;
			const sourceY = plan.source.y + (y - targetTop + 0.5) * scaleY - 0.5;
			const pixel = sample(source, sourceX, sourceY, scaleX, scaleY, mode);
			output.data[targetOffset] = pixel[0];
			output.data[targetOffset + 1] = pixel[1];
			output.data[targetOffset + 2] = pixel[2];
			output.data[targetOffset + 3] = pixel[3];
		}
	}

	return output;
}

function sample(
	source: ImageData,
	x: number,
	y: number,
	scaleX: number,
	scaleY: number,
	mode: ResizeId
) {
	switch (mode) {
		case 'nearest':
			return getPixel(source.data, source.width, source.height, Math.round(x), Math.round(y));
		case 'bilinear':
			return sampleBilinear(source, x, y);
		case 'area':
			return sampleArea(source, x, y, scaleX, scaleY);
		case 'lanczos3':
			return sampleLanczos(source, x, y);
		default:
			throw new Error(`Unsupported resize mode: ${mode satisfies never}`);
	}
}

function addWeightedSample(
	accumulator: { r: number; g: number; b: number; a: number; total: number },
	pixel: readonly [number, number, number, number],
	weight: number
) {
	const alpha = pixel[3] / 255;
	accumulator.r += pixel[0] * alpha * weight;
	accumulator.g += pixel[1] * alpha * weight;
	accumulator.b += pixel[2] * alpha * weight;
	accumulator.a += pixel[3] * weight;
	accumulator.total += weight;
}

function finishWeightedSample(accumulator: {
	r: number;
	g: number;
	b: number;
	a: number;
	total: number;
}) {
	if (accumulator.total === 0) return [0, 0, 0, 0] as const;
	return unpremultiplySample(
		accumulator.r / accumulator.total,
		accumulator.g / accumulator.total,
		accumulator.b / accumulator.total,
		accumulator.a / accumulator.total
	);
}

function sampleBilinear(source: ImageData, x: number, y: number) {
	const x0 = Math.floor(x);
	const y0 = Math.floor(y);
	const tx = x - x0;
	const ty = y - y0;
	const accumulator = { r: 0, g: 0, b: 0, a: 0, total: 0 };
	addWeightedSample(
		accumulator,
		getPixel(source.data, source.width, source.height, x0, y0),
		(1 - tx) * (1 - ty)
	);
	addWeightedSample(
		accumulator,
		getPixel(source.data, source.width, source.height, x0 + 1, y0),
		tx * (1 - ty)
	);
	addWeightedSample(
		accumulator,
		getPixel(source.data, source.width, source.height, x0, y0 + 1),
		(1 - tx) * ty
	);
	addWeightedSample(
		accumulator,
		getPixel(source.data, source.width, source.height, x0 + 1, y0 + 1),
		tx * ty
	);
	return finishWeightedSample(accumulator);
}

function sampleLanczos(source: ImageData, x: number, y: number) {
	const accumulator = { r: 0, g: 0, b: 0, a: 0, total: 0 };
	const radius = 3;
	const floorX = Math.floor(x);
	const floorY = Math.floor(y);
	const startX = floorX - radius + 1;
	const startY = floorY - radius + 1;
	for (let yy = startY; yy <= floorY + radius; yy++) {
		const wy = lanczos(y - yy, radius);
		for (let xx = startX; xx <= floorX + radius; xx++) {
			const weight = lanczos(x - xx, radius) * wy;
			if (weight === 0) continue;
			addWeightedSample(
				accumulator,
				getPixel(source.data, source.width, source.height, xx, yy),
				weight
			);
		}
	}
	return finishWeightedSample(accumulator);
}

type TargetBounds = {
	left: number;
	top: number;
	right: number;
	bottom: number;
	width: number;
	height: number;
};

type ContributionTable = {
	indices: Int32Array;
	weights: Float64Array;
};

const LANCZOS_RADIUS = 3;
const LANCZOS_TAPS = LANCZOS_RADIUS * 2;

function resizeLanczos3(
	source: ImageData,
	output: ImageData,
	sourceRect: Rect,
	target: TargetBounds
) {
	const xTable = contributionTable(
		target.width,
		sourceRect.x,
		sourceRect.width / target.width,
		source.width
	);
	const yTable = contributionTable(
		target.height,
		sourceRect.y,
		sourceRect.height / target.height,
		source.height
	);
	const sourceData = source.data;
	const outputData = output.data;

	for (let y = target.top; y < target.bottom; y++) {
		const targetY = y - target.top;
		const yBase = targetY * LANCZOS_TAPS;
		for (let x = target.left; x < target.right; x++) {
			const targetX = x - target.left;
			const xBase = targetX * LANCZOS_TAPS;
			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;
			let total = 0;

			for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
				const wy = yTable.weights[yBase + yTap]!;
				if (wy === 0) continue;
				const rowOffset = yTable.indices[yBase + yTap]! * source.width * 4;
				for (let xTap = 0; xTap < LANCZOS_TAPS; xTap++) {
					const weight = wy * xTable.weights[xBase + xTap]!;
					if (weight === 0) continue;
					const offset = rowOffset + xTable.indices[xBase + xTap]! * 4;
					const alpha = sourceData[offset + 3]! / 255;
					r += sourceData[offset]! * alpha * weight;
					g += sourceData[offset + 1]! * alpha * weight;
					b += sourceData[offset + 2]! * alpha * weight;
					a += sourceData[offset + 3]! * weight;
					total += weight;
				}
			}

			const targetOffset = (y * output.width + x) * 4;
			if (total === 0) {
				outputData[targetOffset] = 0;
				outputData[targetOffset + 1] = 0;
				outputData[targetOffset + 2] = 0;
				outputData[targetOffset + 3] = 0;
				continue;
			}
			const [outR, outG, outB, outA] = unpremultiplySample(
				r / total,
				g / total,
				b / total,
				a / total
			);
			outputData[targetOffset] = outR;
			outputData[targetOffset + 1] = outG;
			outputData[targetOffset + 2] = outB;
			outputData[targetOffset + 3] = outA;
		}
	}
}

function contributionTable(count: number, sourceStart: number, scale: number, sourceLimit: number) {
	const indices = new Int32Array(count * LANCZOS_TAPS);
	const weights = new Float64Array(count * LANCZOS_TAPS);
	for (let target = 0; target < count; target++) {
		const sourcePosition = sourceStart + (target + 0.5) * scale - 0.5;
		const floor = Math.floor(sourcePosition);
		const first = floor - LANCZOS_RADIUS + 1;
		const base = target * LANCZOS_TAPS;
		for (let tap = 0; tap < LANCZOS_TAPS; tap++) {
			const sourceIndex = first + tap;
			indices[base + tap] = Math.min(sourceLimit - 1, Math.max(0, sourceIndex));
			weights[base + tap] = lanczos(sourcePosition - sourceIndex, LANCZOS_RADIUS);
		}
	}
	return { indices, weights } satisfies ContributionTable;
}

function sampleArea(source: ImageData, x: number, y: number, scaleX: number, scaleY: number) {
	if (scaleX < 1 && scaleY < 1) return sampleBilinear(source, x, y);
	const left = Math.floor(x - scaleX / 2);
	const right = Math.ceil(x + scaleX / 2);
	const top = Math.floor(y - scaleY / 2);
	const bottom = Math.ceil(y + scaleY / 2);
	const accumulator = { r: 0, g: 0, b: 0, a: 0, total: 0 };
	for (let yy = top; yy <= bottom; yy++) {
		for (let xx = left; xx <= right; xx++) {
			addWeightedSample(accumulator, getPixel(source.data, source.width, source.height, xx, yy), 1);
		}
	}
	return finishWeightedSample(accumulator);
}
