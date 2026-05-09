import type { CropRect, ResizeId } from './types';
import { unpremultiplySample } from './compositing';
import { clampByte } from './color';
import { assertOutputDimensions, assertSourceDimensions } from './schemas';

type Rect = { x: number; y: number; width: number; height: number };

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
	mode: ResizeId,
	crop?: CropRect
): ImageData {
	const { width, height } = assertOutputDimensions(outWidth, outHeight, 'Resize output');
	assertSourceDimensions(source.width, source.height, 'Resize source');
	const sourceRect = clampCrop(source.width, source.height, crop);
	const output = new ImageData(width, height);
	const targetLeft = 0;
	const targetTop = 0;
	const targetRight = width;
	const targetBottom = height;
	const targetWidth = width;
	const targetHeight = height;
	const scaleX = sourceRect.width / targetWidth;
	const scaleY = sourceRect.height / targetHeight;

	if (mode === 'lanczos2') {
		resizeLanczos2(source, output, sourceRect, {
			left: targetLeft,
			top: targetTop,
			right: targetRight,
			bottom: targetBottom,
			width: targetWidth,
			height: targetHeight
		});
		return output;
	}

	if (mode === 'lanczos2-scale-aware') {
		resizeScaleAwareLanczos(
			source,
			output,
			sourceRect,
			{
				left: targetLeft,
				top: targetTop,
				right: targetRight,
				bottom: targetBottom,
				width: targetWidth,
				height: targetHeight
			},
			LANCZOS2_RADIUS
		);
		return output;
	}

	if (mode === 'lanczos3-scale-aware') {
		resizeScaleAwareLanczos(
			source,
			output,
			sourceRect,
			{
				left: targetLeft,
				top: targetTop,
				right: targetRight,
				bottom: targetBottom,
				width: targetWidth,
				height: targetHeight
			},
			LANCZOS_RADIUS
		);
		return output;
	}

	if (mode === 'lanczos3') {
		resizeLanczos3(source, output, sourceRect, {
			left: targetLeft,
			top: targetTop,
			right: targetRight,
			bottom: targetBottom,
			width: targetWidth,
			height: targetHeight
		});
		return output;
	}

	if (mode === 'bilinear') {
		resizeBilinearDirect(source, output, sourceRect);
		return output;
	}

	for (let y = targetTop; y < targetBottom; y++) {
		for (let x = targetLeft; x < targetRight; x++) {
			const targetOffset = (y * width + x) * 4;
			const sourceX = sourceRect.x + (x - targetLeft + 0.5) * scaleX - 0.5;
			const sourceY = sourceRect.y + (y - targetTop + 0.5) * scaleY - 0.5;
			const pixel = sample(source, sourceX, sourceY, scaleX, scaleY, mode);
			output.data[targetOffset] = pixel[0];
			output.data[targetOffset + 1] = pixel[1];
			output.data[targetOffset + 2] = pixel[2];
			output.data[targetOffset + 3] = pixel[3];
		}
	}

	return output;
}

function resizeBilinearDirect(source: ImageData, output: ImageData, sourceRect: Rect) {
	const outputData = output.data;
	const data = source.data;
	const outputWidth = output.width;
	const outputHeight = output.height;
	const sourceWidth = source.width;
	const sourceHeight = source.height;
	const scaleX = sourceRect.width / outputWidth;
	const scaleY = sourceRect.height / outputHeight;
	for (let y = 0; y < outputHeight; y++) {
		const sourceY = sourceRect.y + (y + 0.5) * scaleY - 0.5;
		const y0 = Math.floor(sourceY);
		const y1 = y0 + 1;
		const clampedY0 = Math.min(sourceHeight - 1, Math.max(0, y0));
		const clampedY1 = Math.min(sourceHeight - 1, Math.max(0, y1));
		const ty = sourceY - y0;
		const rowOffset0 = clampedY0 * sourceWidth * 4;
		const rowOffset1 = clampedY1 * sourceWidth * 4;
		let targetOffset = y * outputWidth * 4;
		for (let x = 0; x < outputWidth; x++) {
			const sourceX = sourceRect.x + (x + 0.5) * scaleX - 0.5;
			const x0 = Math.floor(sourceX);
			const x1 = x0 + 1;
			const clampedX0 = Math.min(sourceWidth - 1, Math.max(0, x0));
			const clampedX1 = Math.min(sourceWidth - 1, Math.max(0, x1));
			const tx = sourceX - x0;
			const weight00 = (1 - tx) * (1 - ty);
			const weight10 = tx * (1 - ty);
			const weight01 = (1 - tx) * ty;
			const weight11 = tx * ty;
			const offset00 = rowOffset0 + clampedX0 * 4;
			const offset10 = rowOffset0 + clampedX1 * 4;
			const offset01 = rowOffset1 + clampedX0 * 4;
			const offset11 = rowOffset1 + clampedX1 * 4;
			const total = weight00 + weight10 + weight01 + weight11;

			if (
				data[offset00 + 3] === 255 &&
				data[offset10 + 3] === 255 &&
				data[offset01 + 3] === 255 &&
				data[offset11 + 3] === 255
			) {
				outputData[targetOffset] = clampByte(
					(data[offset00]! * weight00 +
						data[offset10]! * weight10 +
						data[offset01]! * weight01 +
						data[offset11]! * weight11) /
						total
				);
				outputData[targetOffset + 1] = clampByte(
					(data[offset00 + 1]! * weight00 +
						data[offset10 + 1]! * weight10 +
						data[offset01 + 1]! * weight01 +
						data[offset11 + 1]! * weight11) /
						total
				);
				outputData[targetOffset + 2] = clampByte(
					(data[offset00 + 2]! * weight00 +
						data[offset10 + 2]! * weight10 +
						data[offset01 + 2]! * weight01 +
						data[offset11 + 2]! * weight11) /
						total
				);
				outputData[targetOffset + 3] = 255;
				targetOffset += 4;
				continue;
			}

			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;
			let alpha = data[offset00 + 3]! / 255;
			r += data[offset00]! * alpha * weight00;
			g += data[offset00 + 1]! * alpha * weight00;
			b += data[offset00 + 2]! * alpha * weight00;
			a += data[offset00 + 3]! * weight00;

			alpha = data[offset10 + 3]! / 255;
			r += data[offset10]! * alpha * weight10;
			g += data[offset10 + 1]! * alpha * weight10;
			b += data[offset10 + 2]! * alpha * weight10;
			a += data[offset10 + 3]! * weight10;

			alpha = data[offset01 + 3]! / 255;
			r += data[offset01]! * alpha * weight01;
			g += data[offset01 + 1]! * alpha * weight01;
			b += data[offset01 + 2]! * alpha * weight01;
			a += data[offset01 + 3]! * weight01;

			alpha = data[offset11 + 3]! / 255;
			r += data[offset11]! * alpha * weight11;
			g += data[offset11 + 1]! * alpha * weight11;
			b += data[offset11 + 2]! * alpha * weight11;
			a += data[offset11 + 3]! * weight11;

			if (total === 0) {
				outputData[targetOffset] = 0;
				outputData[targetOffset + 1] = 0;
				outputData[targetOffset + 2] = 0;
				outputData[targetOffset + 3] = 0;
				targetOffset += 4;
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
			targetOffset += 4;
		}
	}
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
		case 'lanczos2':
			return sampleLanczos(source, x, y, 2);
		case 'lanczos2-scale-aware':
			return sampleScaleAwareLanczos(source, x, y, scaleX, scaleY, 2);
		case 'lanczos3':
			return sampleLanczos(source, x, y, 3);
		case 'lanczos3-scale-aware':
			return sampleScaleAwareLanczos(source, x, y, scaleX, scaleY, 3);
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
	const data = source.data;
	const x0 = Math.floor(x);
	const y0 = Math.floor(y);
	const x1 = x0 + 1;
	const y1 = y0 + 1;
	const clampedX0 = Math.min(source.width - 1, Math.max(0, x0));
	const clampedX1 = Math.min(source.width - 1, Math.max(0, x1));
	const clampedY0 = Math.min(source.height - 1, Math.max(0, y0));
	const clampedY1 = Math.min(source.height - 1, Math.max(0, y1));
	const tx = x - x0;
	const ty = y - y0;
	const weight00 = (1 - tx) * (1 - ty);
	const weight10 = tx * (1 - ty);
	const weight01 = (1 - tx) * ty;
	const weight11 = tx * ty;
	const offset00 = (clampedY0 * source.width + clampedX0) * 4;
	const offset10 = (clampedY0 * source.width + clampedX1) * 4;
	const offset01 = (clampedY1 * source.width + clampedX0) * 4;
	const offset11 = (clampedY1 * source.width + clampedX1) * 4;
	const total = weight00 + weight10 + weight01 + weight11;

	if (
		data[offset00 + 3] === 255 &&
		data[offset10 + 3] === 255 &&
		data[offset01 + 3] === 255 &&
		data[offset11 + 3] === 255
	) {
		return [
			clampByte(
				(data[offset00]! * weight00 +
					data[offset10]! * weight10 +
					data[offset01]! * weight01 +
					data[offset11]! * weight11) /
					total
			),
			clampByte(
				(data[offset00 + 1]! * weight00 +
					data[offset10 + 1]! * weight10 +
					data[offset01 + 1]! * weight01 +
					data[offset11 + 1]! * weight11) /
					total
			),
			clampByte(
				(data[offset00 + 2]! * weight00 +
					data[offset10 + 2]! * weight10 +
					data[offset01 + 2]! * weight01 +
					data[offset11 + 2]! * weight11) /
					total
			),
			255
		] as const;
	}

	let r = 0;
	let g = 0;
	let b = 0;
	let a = 0;
	let alpha = data[offset00 + 3]! / 255;
	r += data[offset00]! * alpha * weight00;
	g += data[offset00 + 1]! * alpha * weight00;
	b += data[offset00 + 2]! * alpha * weight00;
	a += data[offset00 + 3]! * weight00;

	alpha = data[offset10 + 3]! / 255;
	r += data[offset10]! * alpha * weight10;
	g += data[offset10 + 1]! * alpha * weight10;
	b += data[offset10 + 2]! * alpha * weight10;
	a += data[offset10 + 3]! * weight10;

	alpha = data[offset01 + 3]! / 255;
	r += data[offset01]! * alpha * weight01;
	g += data[offset01 + 1]! * alpha * weight01;
	b += data[offset01 + 2]! * alpha * weight01;
	a += data[offset01 + 3]! * weight01;

	alpha = data[offset11 + 3]! / 255;
	r += data[offset11]! * alpha * weight11;
	g += data[offset11 + 1]! * alpha * weight11;
	b += data[offset11 + 2]! * alpha * weight11;
	a += data[offset11 + 3]! * weight11;

	if (total === 0) return [0, 0, 0, 0] as const;
	return unpremultiplySample(r / total, g / total, b / total, a / total);
}

function sampleLanczos(source: ImageData, x: number, y: number, radius: number) {
	const accumulator = { r: 0, g: 0, b: 0, a: 0, total: 0 };
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

function sampleScaleAwareLanczos(
	source: ImageData,
	x: number,
	y: number,
	scaleX: number,
	scaleY: number,
	radius: number
) {
	const accumulator = { r: 0, g: 0, b: 0, a: 0, total: 0 };
	const filterScaleX = Math.max(1, scaleX);
	const filterScaleY = Math.max(1, scaleY);
	const tapsX = scaleAwareTaps(radius, scaleX);
	const tapsY = scaleAwareTaps(radius, scaleY);
	const floorX = Math.floor(x);
	const floorY = Math.floor(y);
	const startX = floorX - tapsX / 2 + 1;
	const startY = floorY - tapsY / 2 + 1;
	for (let yTap = 0; yTap < tapsY; yTap++) {
		const yy = startY + yTap;
		const wy = lanczos((y - yy) / filterScaleY, radius);
		for (let xTap = 0; xTap < tapsX; xTap++) {
			const xx = startX + xTap;
			const weight = lanczos((x - xx) / filterScaleX, radius) * wy;
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
	byteOffsets: Int32Array;
	weights: Float64Array;
	totals: Float64Array;
};

const LANCZOS_RADIUS = 3;
const LANCZOS_TAPS = LANCZOS_RADIUS * 2;
const LANCZOS2_RADIUS = 2;
const LANCZOS2_TAPS = LANCZOS2_RADIUS * 2;

function resizeLanczos2(
	source: ImageData,
	output: ImageData,
	sourceRect: Rect,
	target: TargetBounds
) {
	const xTable = contributionTable(
		target.width,
		sourceRect.x,
		sourceRect.width / target.width,
		source.width,
		LANCZOS2_RADIUS,
		LANCZOS2_TAPS
	);
	const yTable = contributionTable(
		target.height,
		sourceRect.y,
		sourceRect.height / target.height,
		source.height,
		LANCZOS2_RADIUS,
		LANCZOS2_TAPS
	);

	if (hasTransparentPixels(source.data)) {
		resizeLanczos2WithAlpha(source, output, target, xTable, yTable);
		return;
	}

	resizeOpaqueLanczos2(source, output, target, xTable, yTable);
}

function resizeOpaqueLanczos2(
	source: ImageData,
	output: ImageData,
	target: TargetBounds,
	xTable: ContributionTable,
	yTable: ContributionTable
) {
	const outputData = output.data;
	const rowCache = new Map<number, Float64Array>();
	const lastUse = lastUseTable(source.height, target.height, yTable, LANCZOS2_TAPS);

	for (let y = target.top; y < target.bottom; y++) {
		const targetY = y - target.top;
		const yBase = targetY * LANCZOS2_TAPS;
		const yTotal = yTable.totals[targetY]!;
		const rows = lanczos2RowsForTargetY(
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

			const rowOffset = targetX * 3;
			let r = 0;
			let g = 0;
			let b = 0;

			const row0 = rows[0];
			const weight0 = yTable.weights[yBase]!;
			if (weight0 !== 0 && row0) {
				r += row0[rowOffset]! * weight0;
				g += row0[rowOffset + 1]! * weight0;
				b += row0[rowOffset + 2]! * weight0;
			}

			const row1 = rows[1];
			const weight1 = yTable.weights[yBase + 1]!;
			if (weight1 !== 0 && row1) {
				r += row1[rowOffset]! * weight1;
				g += row1[rowOffset + 1]! * weight1;
				b += row1[rowOffset + 2]! * weight1;
			}

			const row2 = rows[2];
			const weight2 = yTable.weights[yBase + 2]!;
			if (weight2 !== 0 && row2) {
				r += row2[rowOffset]! * weight2;
				g += row2[rowOffset + 1]! * weight2;
				b += row2[rowOffset + 2]! * weight2;
			}

			const row3 = rows[3];
			const weight3 = yTable.weights[yBase + 3]!;
			if (weight3 !== 0 && row3) {
				r += row3[rowOffset]! * weight3;
				g += row3[rowOffset + 1]! * weight3;
				b += row3[rowOffset + 2]! * weight3;
			}

			outputData[targetOffset] = clampByte(r / total);
			outputData[targetOffset + 1] = clampByte(g / total);
			outputData[targetOffset + 2] = clampByte(b / total);
			outputData[targetOffset + 3] = 255;
		}

		evictLanczosRows(rowCache, lastUse, targetY, yTable, yBase, LANCZOS2_TAPS);
	}
}

function resizeLanczos2WithAlpha(
	source: ImageData,
	output: ImageData,
	target: TargetBounds,
	xTable: ContributionTable,
	yTable: ContributionTable
) {
	const outputData = output.data;
	const rowCache = new Map<number, Float64Array>();
	const lastUse = lastUseTable(source.height, target.height, yTable, LANCZOS2_TAPS);

	for (let y = target.top; y < target.bottom; y++) {
		const targetY = y - target.top;
		const yBase = targetY * LANCZOS2_TAPS;
		const yTotal = yTable.totals[targetY]!;
		const rows = lanczos2RowsForTargetY(
			source,
			target.width,
			xTable,
			yTable,
			yBase,
			rowCache,
			true
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

			const rowOffset = targetX * 4;
			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;

			const row0 = rows[0];
			const weight0 = yTable.weights[yBase]!;
			if (weight0 !== 0 && row0) {
				r += row0[rowOffset]! * weight0;
				g += row0[rowOffset + 1]! * weight0;
				b += row0[rowOffset + 2]! * weight0;
				a += row0[rowOffset + 3]! * weight0;
			}

			const row1 = rows[1];
			const weight1 = yTable.weights[yBase + 1]!;
			if (weight1 !== 0 && row1) {
				r += row1[rowOffset]! * weight1;
				g += row1[rowOffset + 1]! * weight1;
				b += row1[rowOffset + 2]! * weight1;
				a += row1[rowOffset + 3]! * weight1;
			}

			const row2 = rows[2];
			const weight2 = yTable.weights[yBase + 2]!;
			if (weight2 !== 0 && row2) {
				r += row2[rowOffset]! * weight2;
				g += row2[rowOffset + 1]! * weight2;
				b += row2[rowOffset + 2]! * weight2;
				a += row2[rowOffset + 3]! * weight2;
			}

			const row3 = rows[3];
			const weight3 = yTable.weights[yBase + 3]!;
			if (weight3 !== 0 && row3) {
				r += row3[rowOffset]! * weight3;
				g += row3[rowOffset + 1]! * weight3;
				b += row3[rowOffset + 2]! * weight3;
				a += row3[rowOffset + 3]! * weight3;
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

		evictLanczosRows(rowCache, lastUse, targetY, yTable, yBase, LANCZOS2_TAPS);
	}
}

function lanczos2RowsForTargetY(
	source: ImageData,
	targetWidth: number,
	xTable: ContributionTable,
	yTable: ContributionTable,
	yBase: number,
	rowCache: Map<number, Float64Array>,
	withAlpha: boolean
) {
	const rows: Array<Float64Array | undefined> = [];
	for (let yTap = 0; yTap < LANCZOS2_TAPS; yTap++) {
		if (yTable.weights[yBase + yTap] === 0) {
			rows[yTap] = undefined;
			continue;
		}
		const sourceY = yTable.indices[yBase + yTap]!;
		let row = rowCache.get(sourceY);
		if (!row) {
			row = withAlpha
				? lanczosAlphaRow2(source, sourceY, targetWidth, xTable)
				: lanczosOpaqueRow2(source, sourceY, targetWidth, xTable);
			rowCache.set(sourceY, row);
		}
		rows[yTap] = row;
	}
	return rows;
}

function lanczosOpaqueRow2(
	source: ImageData,
	sourceY: number,
	targetWidth: number,
	xTable: ContributionTable
) {
	const sourceData = source.data;
	const row = new Float64Array(targetWidth * 3);
	const sourceRowOffset = sourceY * source.width * 4;
	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const xBase = targetX * LANCZOS2_TAPS;
		let offset = sourceRowOffset + xTable.byteOffsets[xBase]!;
		const weight0 = xTable.weights[xBase]!;
		let r = sourceData[offset]! * weight0;
		let g = sourceData[offset + 1]! * weight0;
		let b = sourceData[offset + 2]! * weight0;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 1]!;
		const weight1 = xTable.weights[xBase + 1]!;
		r += sourceData[offset]! * weight1;
		g += sourceData[offset + 1]! * weight1;
		b += sourceData[offset + 2]! * weight1;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 2]!;
		const weight2 = xTable.weights[xBase + 2]!;
		r += sourceData[offset]! * weight2;
		g += sourceData[offset + 1]! * weight2;
		b += sourceData[offset + 2]! * weight2;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 3]!;
		const weight3 = xTable.weights[xBase + 3]!;
		r += sourceData[offset]! * weight3;
		g += sourceData[offset + 1]! * weight3;
		b += sourceData[offset + 2]! * weight3;

		const rowOffset = targetX * 3;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
	}
	return row;
}

function lanczosAlphaRow2(
	source: ImageData,
	sourceY: number,
	targetWidth: number,
	xTable: ContributionTable
) {
	const sourceData = source.data;
	const row = new Float64Array(targetWidth * 4);
	const sourceRowOffset = sourceY * source.width * 4;
	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const xBase = targetX * LANCZOS2_TAPS;
		let offset = sourceRowOffset + xTable.byteOffsets[xBase]!;
		let alpha = sourceData[offset + 3]! / 255;
		const weight0 = xTable.weights[xBase]!;
		let r = sourceData[offset]! * alpha * weight0;
		let g = sourceData[offset + 1]! * alpha * weight0;
		let b = sourceData[offset + 2]! * alpha * weight0;
		let a = sourceData[offset + 3]! * weight0;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 1]!;
		alpha = sourceData[offset + 3]! / 255;
		const weight1 = xTable.weights[xBase + 1]!;
		r += sourceData[offset]! * alpha * weight1;
		g += sourceData[offset + 1]! * alpha * weight1;
		b += sourceData[offset + 2]! * alpha * weight1;
		a += sourceData[offset + 3]! * weight1;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 2]!;
		alpha = sourceData[offset + 3]! / 255;
		const weight2 = xTable.weights[xBase + 2]!;
		r += sourceData[offset]! * alpha * weight2;
		g += sourceData[offset + 1]! * alpha * weight2;
		b += sourceData[offset + 2]! * alpha * weight2;
		a += sourceData[offset + 3]! * weight2;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 3]!;
		alpha = sourceData[offset + 3]! / 255;
		const weight3 = xTable.weights[xBase + 3]!;
		r += sourceData[offset]! * alpha * weight3;
		g += sourceData[offset + 1]! * alpha * weight3;
		b += sourceData[offset + 2]! * alpha * weight3;
		a += sourceData[offset + 3]! * weight3;

		const rowOffset = targetX * 4;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
		row[rowOffset + 3] = a;
	}
	return row;
}

function resizeScaleAwareLanczos(
	source: ImageData,
	output: ImageData,
	sourceRect: Rect,
	target: TargetBounds,
	radius: number
) {
	const scaleX = sourceRect.width / target.width;
	const scaleY = sourceRect.height / target.height;
	const xTaps = scaleAwareTaps(radius, scaleX);
	const yTaps = scaleAwareTaps(radius, scaleY);
	const xTable = contributionTable(
		target.width,
		sourceRect.x,
		scaleX,
		source.width,
		radius,
		xTaps,
		Math.max(1, scaleX)
	);
	const yTable = contributionTable(
		target.height,
		sourceRect.y,
		scaleY,
		source.height,
		radius,
		yTaps,
		Math.max(1, scaleY)
	);

	if (hasTransparentPixels(source.data)) {
		resizeLanczosWithAlphaDynamic(source, output, target, xTable, yTable, xTaps, yTaps);
		return;
	}

	resizeOpaqueLanczosDynamic(source, output, target, xTable, yTable, xTaps, yTaps);
}

function scaleAwareTaps(radius: number, scale: number) {
	return Math.ceil(radius * Math.max(1, scale)) * 2;
}

function resizeOpaqueLanczosDynamic(
	source: ImageData,
	output: ImageData,
	target: TargetBounds,
	xTable: ContributionTable,
	yTable: ContributionTable,
	xTaps: number,
	yTaps: number
) {
	const outputData = output.data;
	const rowCache = new Map<number, Float64Array>();
	const lastUse = lastUseTable(source.height, target.height, yTable, yTaps);

	for (let y = target.top; y < target.bottom; y++) {
		const targetY = y - target.top;
		const yBase = targetY * yTaps;
		const yTotal = yTable.totals[targetY]!;
		const rows = lanczosRowsForTargetYDynamic(
			source,
			target.width,
			xTable,
			yTable,
			yBase,
			rowCache,
			false,
			xTaps,
			yTaps
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

			const rowOffset = targetX * 3;
			let r = 0;
			let g = 0;
			let b = 0;
			for (let yTap = 0; yTap < yTaps; yTap++) {
				const weight = yTable.weights[yBase + yTap]!;
				const row = rows[yTap];
				if (weight === 0 || !row) continue;
				r += row[rowOffset]! * weight;
				g += row[rowOffset + 1]! * weight;
				b += row[rowOffset + 2]! * weight;
			}

			outputData[targetOffset] = clampByte(r / total);
			outputData[targetOffset + 1] = clampByte(g / total);
			outputData[targetOffset + 2] = clampByte(b / total);
			outputData[targetOffset + 3] = 255;
		}

		evictLanczosRows(rowCache, lastUse, targetY, yTable, yBase, yTaps);
	}
}

function resizeLanczosWithAlphaDynamic(
	source: ImageData,
	output: ImageData,
	target: TargetBounds,
	xTable: ContributionTable,
	yTable: ContributionTable,
	xTaps: number,
	yTaps: number
) {
	const outputData = output.data;
	const rowCache = new Map<number, Float64Array>();
	const lastUse = lastUseTable(source.height, target.height, yTable, yTaps);

	for (let y = target.top; y < target.bottom; y++) {
		const targetY = y - target.top;
		const yBase = targetY * yTaps;
		const yTotal = yTable.totals[targetY]!;
		const rows = lanczosRowsForTargetYDynamic(
			source,
			target.width,
			xTable,
			yTable,
			yBase,
			rowCache,
			true,
			xTaps,
			yTaps
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

			const rowOffset = targetX * 4;
			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;
			for (let yTap = 0; yTap < yTaps; yTap++) {
				const weight = yTable.weights[yBase + yTap]!;
				const row = rows[yTap];
				if (weight === 0 || !row) continue;
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

		evictLanczosRows(rowCache, lastUse, targetY, yTable, yBase, yTaps);
	}
}

function lanczosRowsForTargetYDynamic(
	source: ImageData,
	targetWidth: number,
	xTable: ContributionTable,
	yTable: ContributionTable,
	yBase: number,
	rowCache: Map<number, Float64Array>,
	withAlpha: boolean,
	xTaps: number,
	yTaps: number
) {
	const rows: Array<Float64Array | undefined> = [];
	for (let yTap = 0; yTap < yTaps; yTap++) {
		if (yTable.weights[yBase + yTap] === 0) {
			rows[yTap] = undefined;
			continue;
		}
		const sourceY = yTable.indices[yBase + yTap]!;
		let row = rowCache.get(sourceY);
		if (!row) {
			row = withAlpha
				? lanczosAlphaRowDynamic(source, sourceY, targetWidth, xTable, xTaps)
				: lanczosOpaqueRowDynamic(source, sourceY, targetWidth, xTable, xTaps);
			rowCache.set(sourceY, row);
		}
		rows[yTap] = row;
	}
	return rows;
}

function lanczosOpaqueRowDynamic(
	source: ImageData,
	sourceY: number,
	targetWidth: number,
	xTable: ContributionTable,
	xTaps: number
) {
	const sourceData = source.data;
	const row = new Float64Array(targetWidth * 3);
	const sourceRowOffset = sourceY * source.width * 4;
	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const xBase = targetX * xTaps;
		let r = 0;
		let g = 0;
		let b = 0;
		for (let xTap = 0; xTap < xTaps; xTap++) {
			const offset = sourceRowOffset + xTable.byteOffsets[xBase + xTap]!;
			const weight = xTable.weights[xBase + xTap]!;
			r += sourceData[offset]! * weight;
			g += sourceData[offset + 1]! * weight;
			b += sourceData[offset + 2]! * weight;
		}

		const rowOffset = targetX * 3;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
	}
	return row;
}

function lanczosAlphaRowDynamic(
	source: ImageData,
	sourceY: number,
	targetWidth: number,
	xTable: ContributionTable,
	xTaps: number
) {
	const sourceData = source.data;
	const row = new Float64Array(targetWidth * 4);
	const sourceRowOffset = sourceY * source.width * 4;
	for (let targetX = 0; targetX < targetWidth; targetX++) {
		const xBase = targetX * xTaps;
		let r = 0;
		let g = 0;
		let b = 0;
		let a = 0;
		for (let xTap = 0; xTap < xTaps; xTap++) {
			const offset = sourceRowOffset + xTable.byteOffsets[xBase + xTap]!;
			const weight = xTable.weights[xBase + xTap]!;
			const alpha = sourceData[offset + 3]! / 255;
			r += sourceData[offset]! * alpha * weight;
			g += sourceData[offset + 1]! * alpha * weight;
			b += sourceData[offset + 2]! * alpha * weight;
			a += sourceData[offset + 3]! * weight;
		}

		const rowOffset = targetX * 4;
		row[rowOffset] = r;
		row[rowOffset + 1] = g;
		row[rowOffset + 2] = b;
		row[rowOffset + 3] = a;
	}
	return row;
}

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

			const rowOffset = targetX * 3;
			let r = 0;
			let g = 0;
			let b = 0;

			const row0 = rows[0];
			const weight0 = yTable.weights[yBase]!;
			if (weight0 !== 0 && row0) {
				r += row0[rowOffset]! * weight0;
				g += row0[rowOffset + 1]! * weight0;
				b += row0[rowOffset + 2]! * weight0;
			}

			const row1 = rows[1];
			const weight1 = yTable.weights[yBase + 1]!;
			if (weight1 !== 0 && row1) {
				r += row1[rowOffset]! * weight1;
				g += row1[rowOffset + 1]! * weight1;
				b += row1[rowOffset + 2]! * weight1;
			}

			const row2 = rows[2];
			const weight2 = yTable.weights[yBase + 2]!;
			if (weight2 !== 0 && row2) {
				r += row2[rowOffset]! * weight2;
				g += row2[rowOffset + 1]! * weight2;
				b += row2[rowOffset + 2]! * weight2;
			}

			const row3 = rows[3];
			const weight3 = yTable.weights[yBase + 3]!;
			if (weight3 !== 0 && row3) {
				r += row3[rowOffset]! * weight3;
				g += row3[rowOffset + 1]! * weight3;
				b += row3[rowOffset + 2]! * weight3;
			}

			const row4 = rows[4];
			const weight4 = yTable.weights[yBase + 4]!;
			if (weight4 !== 0 && row4) {
				r += row4[rowOffset]! * weight4;
				g += row4[rowOffset + 1]! * weight4;
				b += row4[rowOffset + 2]! * weight4;
			}

			const row5 = rows[5];
			const weight5 = yTable.weights[yBase + 5]!;
			if (weight5 !== 0 && row5) {
				r += row5[rowOffset]! * weight5;
				g += row5[rowOffset + 1]! * weight5;
				b += row5[rowOffset + 2]! * weight5;
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

			const rowOffset = targetX * 4;
			let r = 0;
			let g = 0;
			let b = 0;
			let a = 0;

			const row0 = rows[0];
			const weight0 = yTable.weights[yBase]!;
			if (weight0 !== 0 && row0) {
				r += row0[rowOffset]! * weight0;
				g += row0[rowOffset + 1]! * weight0;
				b += row0[rowOffset + 2]! * weight0;
				a += row0[rowOffset + 3]! * weight0;
			}

			const row1 = rows[1];
			const weight1 = yTable.weights[yBase + 1]!;
			if (weight1 !== 0 && row1) {
				r += row1[rowOffset]! * weight1;
				g += row1[rowOffset + 1]! * weight1;
				b += row1[rowOffset + 2]! * weight1;
				a += row1[rowOffset + 3]! * weight1;
			}

			const row2 = rows[2];
			const weight2 = yTable.weights[yBase + 2]!;
			if (weight2 !== 0 && row2) {
				r += row2[rowOffset]! * weight2;
				g += row2[rowOffset + 1]! * weight2;
				b += row2[rowOffset + 2]! * weight2;
				a += row2[rowOffset + 3]! * weight2;
			}

			const row3 = rows[3];
			const weight3 = yTable.weights[yBase + 3]!;
			if (weight3 !== 0 && row3) {
				r += row3[rowOffset]! * weight3;
				g += row3[rowOffset + 1]! * weight3;
				b += row3[rowOffset + 2]! * weight3;
				a += row3[rowOffset + 3]! * weight3;
			}

			const row4 = rows[4];
			const weight4 = yTable.weights[yBase + 4]!;
			if (weight4 !== 0 && row4) {
				r += row4[rowOffset]! * weight4;
				g += row4[rowOffset + 1]! * weight4;
				b += row4[rowOffset + 2]! * weight4;
				a += row4[rowOffset + 3]! * weight4;
			}

			const row5 = rows[5];
			const weight5 = yTable.weights[yBase + 5]!;
			if (weight5 !== 0 && row5) {
				r += row5[rowOffset]! * weight5;
				g += row5[rowOffset + 1]! * weight5;
				b += row5[rowOffset + 2]! * weight5;
				a += row5[rowOffset + 3]! * weight5;
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

function lastUseTable(
	sourceHeight: number,
	targetHeight: number,
	yTable: ContributionTable,
	taps = LANCZOS_TAPS
) {
	const lastUse = new Int32Array(sourceHeight);
	lastUse.fill(-1);
	for (let targetY = 0; targetY < targetHeight; targetY++) {
		const yBase = targetY * taps;
		for (let yTap = 0; yTap < taps; yTap++) {
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
	yBase: number,
	taps = LANCZOS_TAPS
) {
	for (let yTap = 0; yTap < taps; yTap++) {
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
		let offset = sourceRowOffset + xTable.byteOffsets[xBase]!;
		const weight0 = xTable.weights[xBase]!;
		let r = sourceData[offset]! * weight0;
		let g = sourceData[offset + 1]! * weight0;
		let b = sourceData[offset + 2]! * weight0;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 1]!;
		const weight1 = xTable.weights[xBase + 1]!;
		r += sourceData[offset]! * weight1;
		g += sourceData[offset + 1]! * weight1;
		b += sourceData[offset + 2]! * weight1;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 2]!;
		const weight2 = xTable.weights[xBase + 2]!;
		r += sourceData[offset]! * weight2;
		g += sourceData[offset + 1]! * weight2;
		b += sourceData[offset + 2]! * weight2;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 3]!;
		const weight3 = xTable.weights[xBase + 3]!;
		r += sourceData[offset]! * weight3;
		g += sourceData[offset + 1]! * weight3;
		b += sourceData[offset + 2]! * weight3;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 4]!;
		const weight4 = xTable.weights[xBase + 4]!;
		r += sourceData[offset]! * weight4;
		g += sourceData[offset + 1]! * weight4;
		b += sourceData[offset + 2]! * weight4;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 5]!;
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
		let offset = sourceRowOffset + xTable.byteOffsets[xBase]!;
		let alpha = sourceData[offset + 3]! / 255;
		const weight0 = xTable.weights[xBase]!;
		let r = sourceData[offset]! * alpha * weight0;
		let g = sourceData[offset + 1]! * alpha * weight0;
		let b = sourceData[offset + 2]! * alpha * weight0;
		let a = sourceData[offset + 3]! * weight0;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 1]!;
		alpha = sourceData[offset + 3]! / 255;
		const weight1 = xTable.weights[xBase + 1]!;
		r += sourceData[offset]! * alpha * weight1;
		g += sourceData[offset + 1]! * alpha * weight1;
		b += sourceData[offset + 2]! * alpha * weight1;
		a += sourceData[offset + 3]! * weight1;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 2]!;
		alpha = sourceData[offset + 3]! / 255;
		const weight2 = xTable.weights[xBase + 2]!;
		r += sourceData[offset]! * alpha * weight2;
		g += sourceData[offset + 1]! * alpha * weight2;
		b += sourceData[offset + 2]! * alpha * weight2;
		a += sourceData[offset + 3]! * weight2;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 3]!;
		alpha = sourceData[offset + 3]! / 255;
		const weight3 = xTable.weights[xBase + 3]!;
		r += sourceData[offset]! * alpha * weight3;
		g += sourceData[offset + 1]! * alpha * weight3;
		b += sourceData[offset + 2]! * alpha * weight3;
		a += sourceData[offset + 3]! * weight3;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 4]!;
		alpha = sourceData[offset + 3]! / 255;
		const weight4 = xTable.weights[xBase + 4]!;
		r += sourceData[offset]! * alpha * weight4;
		g += sourceData[offset + 1]! * alpha * weight4;
		b += sourceData[offset + 2]! * alpha * weight4;
		a += sourceData[offset + 3]! * weight4;

		offset = sourceRowOffset + xTable.byteOffsets[xBase + 5]!;
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

function contributionTable(
	count: number,
	sourceStart: number,
	scale: number,
	sourceLimit: number,
	radius = LANCZOS_RADIUS,
	taps = LANCZOS_TAPS,
	filterScale = 1
) {
	const indices = new Int32Array(count * taps);
	const byteOffsets = new Int32Array(count * taps);
	const weights = new Float64Array(count * taps);
	const totals = new Float64Array(count);
	for (let target = 0; target < count; target++) {
		const sourcePosition = sourceStart + (target + 0.5) * scale - 0.5;
		const floor = Math.floor(sourcePosition);
		const first = floor - taps / 2 + 1;
		const base = target * taps;
		let total = 0;
		for (let tap = 0; tap < taps; tap++) {
			const sourceIndex = first + tap;
			const weight = lanczos((sourcePosition - sourceIndex) / filterScale, radius);
			const clampedSourceIndex = Math.min(sourceLimit - 1, Math.max(0, sourceIndex));
			indices[base + tap] = clampedSourceIndex;
			byteOffsets[base + tap] = clampedSourceIndex * 4;
			weights[base + tap] = weight;
			total += weight;
		}
		totals[target] = total;
	}
	return { indices, byteOffsets, weights, totals } satisfies ContributionTable;
}

function sampleArea(source: ImageData, x: number, y: number, scaleX: number, scaleY: number) {
	if (scaleX < 1 && scaleY < 1) return sampleBilinear(source, x, y);
	const data = source.data;
	const left = Math.floor(x - scaleX / 2);
	const right = Math.ceil(x + scaleX / 2);
	const top = Math.floor(y - scaleY / 2);
	const bottom = Math.ceil(y + scaleY / 2);
	let r = 0;
	let g = 0;
	let b = 0;
	let a = 0;
	let total = 0;
	for (let yy = top; yy <= bottom; yy++) {
		const clampedY = Math.min(source.height - 1, Math.max(0, yy));
		for (let xx = left; xx <= right; xx++) {
			const clampedX = Math.min(source.width - 1, Math.max(0, xx));
			const offset = (clampedY * source.width + clampedX) * 4;
			const alpha = data[offset + 3]! / 255;
			r += data[offset]! * alpha;
			g += data[offset + 1]! * alpha;
			b += data[offset + 2]! * alpha;
			a += data[offset + 3]!;
			total += 1;
		}
	}
	if (total === 0) return [0, 0, 0, 0] as const;
	return unpremultiplySample(r / total, g / total, b / total, a / total);
}
