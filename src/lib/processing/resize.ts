import type { CropRect, FitMode, ResizeId } from './types';
import { clampByte } from './color';

type Rect = { x: number; y: number; width: number; height: number };

function clampCrop(sourceWidth: number, sourceHeight: number, crop?: CropRect): Rect {
	if (!crop) return { x: 0, y: 0, width: sourceWidth, height: sourceHeight };
	const x = Math.min(sourceWidth - 1, Math.max(0, crop.x));
	const y = Math.min(sourceHeight - 1, Math.max(0, crop.y));
	return {
		x,
		y,
		width: Math.max(1, Math.min(sourceWidth - x, crop.width)),
		height: Math.max(1, Math.min(sourceHeight - y, crop.height))
	};
}

function sourceRect(
	sourceWidth: number,
	sourceHeight: number,
	outWidth: number,
	outHeight: number,
	fit: FitMode,
	crop?: CropRect
): Rect {
	const base = clampCrop(sourceWidth, sourceHeight, crop);
	if (fit === 'stretch') return base;

	const sourceAspect = base.width / base.height;
	const outputAspect = outWidth / outHeight;
	if (
		(fit === 'contain' && sourceAspect > outputAspect) ||
		(fit === 'cover' && sourceAspect < outputAspect)
	) {
		const height = base.width / outputAspect;
		return { x: base.x, y: base.y + (base.height - height) / 2, width: base.width, height };
	}

	const width = base.height * outputAspect;
	return { x: base.x + (base.width - width) / 2, y: base.y, width, height: base.height };
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
	const rect = sourceRect(source.width, source.height, outWidth, outHeight, fit, crop);
	const output = new ImageData(outWidth, outHeight);
	const scaleX = rect.width / outWidth;
	const scaleY = rect.height / outHeight;

	for (let y = 0; y < outHeight; y++) {
		for (let x = 0; x < outWidth; x++) {
			const targetOffset = (y * outWidth + x) * 4;
			const sourceX = rect.x + (x + 0.5) * scaleX - 0.5;
			const sourceY = rect.y + (y + 0.5) * scaleY - 0.5;
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
		default:
			return sampleLanczos(source, x, y);
	}
}

function sampleBilinear(source: ImageData, x: number, y: number) {
	const x0 = Math.floor(x);
	const y0 = Math.floor(y);
	const tx = x - x0;
	const ty = y - y0;
	const p00 = getPixel(source.data, source.width, source.height, x0, y0);
	const p10 = getPixel(source.data, source.width, source.height, x0 + 1, y0);
	const p01 = getPixel(source.data, source.width, source.height, x0, y0 + 1);
	const p11 = getPixel(source.data, source.width, source.height, x0 + 1, y0 + 1);
	const out: [number, number, number, number] = [0, 0, 0, 0];
	for (let channel = 0; channel < 4; channel++) {
		const top = p00[channel] * (1 - tx) + p10[channel] * tx;
		const bottom = p01[channel] * (1 - tx) + p11[channel] * tx;
		out[channel] = clampByte(top * (1 - ty) + bottom * ty);
	}
	return out;
}

function sampleLanczos(source: ImageData, x: number, y: number) {
	let r = 0;
	let g = 0;
	let b = 0;
	let a = 0;
	let total = 0;
	const radius = 3;
	const startX = Math.floor(x) - radius + 1;
	const startY = Math.floor(y) - radius + 1;
	for (let yy = startY; yy <= Math.floor(y) + radius; yy++) {
		const wy = lanczos(y - yy, radius);
		for (let xx = startX; xx <= Math.floor(x) + radius; xx++) {
			const weight = lanczos(x - xx, radius) * wy;
			if (weight === 0) continue;
			const pixel = getPixel(source.data, source.width, source.height, xx, yy);
			r += pixel[0] * weight;
			g += pixel[1] * weight;
			b += pixel[2] * weight;
			a += pixel[3] * weight;
			total += weight;
		}
	}
	return [
		clampByte(r / total),
		clampByte(g / total),
		clampByte(b / total),
		clampByte(a / total)
	] as const;
}

function sampleArea(source: ImageData, x: number, y: number, scaleX: number, scaleY: number) {
	if (scaleX < 1 && scaleY < 1) return sampleBilinear(source, x, y);
	const left = Math.floor(x - scaleX / 2);
	const right = Math.ceil(x + scaleX / 2);
	const top = Math.floor(y - scaleY / 2);
	const bottom = Math.ceil(y + scaleY / 2);
	let r = 0;
	let g = 0;
	let b = 0;
	let a = 0;
	let count = 0;
	for (let yy = top; yy <= bottom; yy++) {
		for (let xx = left; xx <= right; xx++) {
			const pixel = getPixel(source.data, source.width, source.height, xx, yy);
			r += pixel[0];
			g += pixel[1];
			b += pixel[2];
			a += pixel[3];
			count++;
		}
	}
	return [
		clampByte(r / count),
		clampByte(g / count),
		clampByte(b / count),
		clampByte(a / count)
	] as const;
}
