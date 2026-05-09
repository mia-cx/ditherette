import type { CropRect, FitMode, ResizeId } from './types';
import { unpremultiplySample } from './compositing';
import { clampByte } from './color';
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
	totals: Float64Array;
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

	if (hasTransparentPixels(source.data)) {
		resizeLanczos3WithAlpha(source, output, target, xTable, yTable);
		return;
	}

	resizeOpaqueLanczos3(source, output, target, xTable, yTable);
}

function hasTransparentPixels(data: Uint8ClampedArray) {
	for (let offset = 3; offset < data.length; offset += 4) {
		if (data[offset] !== 255) return true;
	}
	return false;
}

function resizeOpaqueLanczos3(
	source: ImageData,
	output: ImageData,
	target: TargetBounds,
	xTable: ContributionTable,
	yTable: ContributionTable
) {
	const outputData = output.data;
	const rowCache = new Map<number, Float64Array>();
	const lastUse = lastUseTable(source.height, target.height, yTable);

	for (let y = target.top; y < target.bottom; y++) {
		const targetY = y - target.top;
		const yBase = targetY * LANCZOS_TAPS;
		const yTotal = yTable.totals[targetY]!;
		const rows = lanczosRowsForTargetY(
			source,
			target.width,
			xTable,
			yTable,
			yBase,
			rowCache,
			false
		);

		for (let x = target.left; x < target.right; x++) {
			const targetX = x - target.left;
			const total = xTable.totals[targetX]! * yTotal;
			const targetOffset = (y * output.width + x) * 4;
			if (total === 0) {
				outputData[targetOffset] = 0;
				outputData[targetOffset + 1] = 0;
				outputData[targetOffset + 2] = 0;
				outputData[targetOffset + 3] = 0;
				continue;
			}

			let r = 0;
			let g = 0;
			let b = 0;
			for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
				const weight = yTable.weights[yBase + yTap]!;
				const row = rows[yTap];
				if (weight === 0 || !row) continue;
				const rowOffset = targetX * 3;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
			}

			outputData[targetOffset] = clampByte(r / total);
			outputData[targetOffset + 1] = clampByte(g / total);
			outputData[targetOffset + 2] = clampByte(b / total);
			outputData[targetOffset + 3] = 255;
		}

		evictLanczosRows(rowCache, lastUse, targetY, yTable, yBase);
	}
}

function resizeLanczos3WithAlpha(
	source: ImageData,
	output: ImageData,
	target: TargetBounds,
	xTable: ContributionTable,
	yTable: ContributionTable
) {
	const outputData = output.data;
	const rowCache = new Map<number, Float64Array>();
	const lastUse = lastUseTable(source.height, target.height, yTable);

	for (let y = target.top; y < target.bottom; y++) {
		const targetY = y - target.top;
		const yBase = targetY * LANCZOS_TAPS;
		const yTotal = yTable.totals[targetY]!;
		const rows = lanczosRowsForTargetY(source, target.width, xTable, yTable, yBase, rowCache, true);

		for (let x = target.left; x < target.right; x++) {
			const targetX = x - target.left;
			const total = xTable.totals[targetX]! * yTotal;
			const targetOffset = (y * output.width + x) * 4;
			if (total === 0) {
				outputData[targetOffset] = 0;
				outputData[targetOffset + 1] = 0;
				outputData[targetOffset + 2] = 0;
				outputData[targetOffset + 3] = 0;
				continue;
			}

			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;
			for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
				const weight = yTable.weights[yBase + yTap]!;
				const row = rows[yTap];
				if (weight === 0 || !row) continue;
				const rowOffset = targetX * 4;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
				a += row[rowOffset + 3]! * weight;
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

		evictLanczosRows(rowCache, lastUse, targetY, yTable, yBase);
	}
}

function lastUseTable(sourceHeight: number, targetHeight: number, yTable: ContributionTable) {
	const lastUse = new Int32Array(sourceHeight);
	lastUse.fill(-1);
	for (let targetY = 0; targetY < targetHeight; targetY++) {
		const yBase = targetY * LANCZOS_TAPS;
		for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
			if (yTable.weights[yBase + yTap] === 0) continue;
			lastUse[yTable.indices[yBase + yTap]!] = targetY;
		}
	}
	return lastUse;
}

function lanczosRowsForTargetY(
	source: ImageData,
	targetWidth: number,
	xTable: ContributionTable,
	yTable: ContributionTable,
	yBase: number,
	rowCache: Map<number, Float64Array>,
	withAlpha: boolean
) {
	const rows: Array<Float64Array | undefined> = [];
	for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
		if (yTable.weights[yBase + yTap] === 0) {
			rows[yTap] = undefined;
			continue;
		}
		const sourceY = yTable.indices[yBase + yTap]!;
		let row = rowCache.get(sourceY);
		if (!row) {
			row = withAlpha
				? lanczosAlphaRow(source, sourceY, targetWidth, xTable)
				: lanczosOpaqueRow(source, sourceY, targetWidth, xTable);
			rowCache.set(sourceY, row);
		}
		rows[yTap] = row;
	}
	return rows;
}

function evictLanczosRows(
	rowCache: Map<number, Float64Array>,
	lastUse: Int32Array,
	targetY: number,
	yTable: ContributionTable,
	yBase: number
) {
	for (let yTap = 0; yTap < LANCZOS_TAPS; yTap++) {
		if (yTable.weights[yBase + yTap] === 0) continue;
		const sourceY = yTable.indices[yBase + yTap]!;
		if (lastUse[sourceY] === targetY) rowCache.delete(sourceY);
	}
}

function lanczosOpaqueRow(
	source: ImageData,
	sourceY: number,
	targetWidth: number,
	xTable: ContributionTable
) {
	const sourceData = source.data;
	const row = new Float64Array(targetWidth * 3);
	const sourceRowOffset = sourceY * source.width * 4;
	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const xBase = targetX * LANCZOS_TAPS;
		let offset = sourceRowOffset + xTable.indices[xBase]! * 4;
		const weight0 = xTable.weights[xBase]!;
		let r = sourceData[offset]! * weight0;
		let g = sourceData[offset + 1]! * weight0;
		let b = sourceData[offset + 2]! * weight0;

		offset = sourceRowOffset + xTable.indices[xBase + 1]! * 4;
		const weight1 = xTable.weights[xBase + 1]!;
		r += sourceData[offset]! * weight1;
		g += sourceData[offset + 1]! * weight1;
		b += sourceData[offset + 2]! * weight1;

		offset = sourceRowOffset + xTable.indices[xBase + 2]! * 4;
		const weight2 = xTable.weights[xBase + 2]!;
		r += sourceData[offset]! * weight2;
		g += sourceData[offset + 1]! * weight2;
		b += sourceData[offset + 2]! * weight2;

		offset = sourceRowOffset + xTable.indices[xBase + 3]! * 4;
		const weight3 = xTable.weights[xBase + 3]!;
		r += sourceData[offset]! * weight3;
		g += sourceData[offset + 1]! * weight3;
		b += sourceData[offset + 2]! * weight3;

		offset = sourceRowOffset + xTable.indices[xBase + 4]! * 4;
		const weight4 = xTable.weights[xBase + 4]!;
		r += sourceData[offset]! * weight4;
		g += sourceData[offset + 1]! * weight4;
		b += sourceData[offset + 2]! * weight4;

		offset = sourceRowOffset + xTable.indices[xBase + 5]! * 4;
		const weight5 = xTable.weights[xBase + 5]!;
		r += sourceData[offset]! * weight5;
		g += sourceData[offset + 1]! * weight5;
		b += sourceData[offset + 2]! * weight5;

		const rowOffset = targetX * 3;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
	}
	return row;
}

function lanczosAlphaRow(
	source: ImageData,
	sourceY: number,
	targetWidth: number,
	xTable: ContributionTable
) {
	const sourceData = source.data;
	const row = new Float64Array(targetWidth * 4);
	const sourceRowOffset = sourceY * source.width * 4;
	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const xBase = targetX * LANCZOS_TAPS;
		let offset = sourceRowOffset + xTable.indices[xBase]! * 4;
		let alpha = sourceData[offset + 3]! / 255;
		const weight0 = xTable.weights[xBase]!;
		let r = sourceData[offset]! * alpha * weight0;
		let g = sourceData[offset + 1]! * alpha * weight0;
		let b = sourceData[offset + 2]! * alpha * weight0;
		let a = sourceData[offset + 3]! * weight0;

		offset = sourceRowOffset + xTable.indices[xBase + 1]! * 4;
		alpha = sourceData[offset + 3]! / 255;
		const weight1 = xTable.weights[xBase + 1]!;
		r += sourceData[offset]! * alpha * weight1;
		g += sourceData[offset + 1]! * alpha * weight1;
		b += sourceData[offset + 2]! * alpha * weight1;
		a += sourceData[offset + 3]! * weight1;

		offset = sourceRowOffset + xTable.indices[xBase + 2]! * 4;
		alpha = sourceData[offset + 3]! / 255;
		const weight2 = xTable.weights[xBase + 2]!;
		r += sourceData[offset]! * alpha * weight2;
		g += sourceData[offset + 1]! * alpha * weight2;
		b += sourceData[offset + 2]! * alpha * weight2;
		a += sourceData[offset + 3]! * weight2;

		offset = sourceRowOffset + xTable.indices[xBase + 3]! * 4;
		alpha = sourceData[offset + 3]! / 255;
		const weight3 = xTable.weights[xBase + 3]!;
		r += sourceData[offset]! * alpha * weight3;
		g += sourceData[offset + 1]! * alpha * weight3;
		b += sourceData[offset + 2]! * alpha * weight3;
		a += sourceData[offset + 3]! * weight3;

		offset = sourceRowOffset + xTable.indices[xBase + 4]! * 4;
		alpha = sourceData[offset + 3]! / 255;
		const weight4 = xTable.weights[xBase + 4]!;
		r += sourceData[offset]! * alpha * weight4;
		g += sourceData[offset + 1]! * alpha * weight4;
		b += sourceData[offset + 2]! * alpha * weight4;
		a += sourceData[offset + 3]! * weight4;

		offset = sourceRowOffset + xTable.indices[xBase + 5]! * 4;
		alpha = sourceData[offset + 3]! / 255;
		const weight5 = xTable.weights[xBase + 5]!;
		r += sourceData[offset]! * alpha * weight5;
		g += sourceData[offset + 1]! * alpha * weight5;
		b += sourceData[offset + 2]! * alpha * weight5;
		a += sourceData[offset + 3]! * weight5;

		const rowOffset = targetX * 4;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
		row[rowOffset + 3] = a;
	}
	return row;
}

function contributionTable(count: number, sourceStart: number, scale: number, sourceLimit: number) {
	const indices = new Int32Array(count * LANCZOS_TAPS);
	const weights = new Float64Array(count * LANCZOS_TAPS);
	const totals = new Float64Array(count);
	for (let target = 0; target < count; target++) {
		const sourcePosition = sourceStart + (target + 0.5) * scale - 0.5;
		const floor = Math.floor(sourcePosition);
		const first = floor - LANCZOS_RADIUS + 1;
		const base = target * LANCZOS_TAPS;
		let total = 0;
		for (let tap = 0; tap < LANCZOS_TAPS; tap++) {
			const sourceIndex = first + tap;
			const weight = lanczos(sourcePosition - sourceIndex, LANCZOS_RADIUS);
			indices[base + tap] = Math.min(sourceLimit - 1, Math.max(0, sourceIndex));
			weights[base + tap] = weight;
			total += weight;
		}
		totals[target] = total;
	}
	return { indices, weights, totals } satisfies ContributionTable;
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
