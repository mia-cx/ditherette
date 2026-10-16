import { validateProcessedImage } from './schemas';
import type { ProcessedImage } from './types';

const LITTLE_ENDIAN = new Uint8Array(new Uint32Array([0x11223344]).buffer)[0] === 0x44;
// Preview construction stages at most 1 MiB of RGBA, reused across strips. Output sides are
// capped at 16,384, so even one complete row fits. The full indexed output stays untouched.
const MAX_RGBA_STRIP_BYTES = 1024 * 1024;

/** Expand exact indexed pixels for callers that need a full RGBA image. */
export function processedToImageData(image: ProcessedImage): ImageData {
	const safeImage = validateProcessedImage(image);
	const output = new ImageData(safeImage.width, safeImage.height);
	writePixels(safeImage, output, rgbaPalette(safeImage), 0);
	return output;
}

/** Draw exact indexed pixels without allocating a second full-size RGBA frame. */
export function drawProcessedImage(
	context: Pick<CanvasRenderingContext2D, 'putImageData'>,
	image: ProcessedImage
) {
	const safeImage = validateProcessedImage(image);
	const rows = Math.min(safeImage.height, Math.floor(MAX_RGBA_STRIP_BYTES / (safeImage.width * 4)));
	const strip = new ImageData(safeImage.width, rows);
	const palette = rgbaPalette(safeImage);
	for (let y = 0; y < safeImage.height; y += rows) {
		writePixels(safeImage, strip, palette, y * safeImage.width);
		context.putImageData(strip, 0, y, 0, 0, safeImage.width, Math.min(rows, safeImage.height - y));
	}
}

function rgbaPalette(image: ProcessedImage) {
	const palette = new Uint8ClampedArray(image.palette.length * 4);
	for (let i = 0; i < image.palette.length; i++) {
		const color = image.palette[i];
		if (!color || color.kind === 'transparent') continue;
		palette.set([color.rgb?.r ?? 0, color.rgb?.g ?? 0, color.rgb?.b ?? 0, 255], i * 4);
	}
	return palette;
}

function writePixels(
	image: ProcessedImage,
	output: ImageData,
	palette: Uint8ClampedArray,
	start: number
) {
	const count = Math.min(output.data.length / 4, image.indices.length - start);
	if (LITTLE_ENDIAN) {
		const rgba = new Uint32Array(output.data.buffer);
		const packedPalette = new Uint32Array(palette.buffer);
		for (let index = 0; index < count; index++)
			rgba[index] = packedPalette[image.indices[start + index]];
		return;
	}

	for (let index = 0; index < count; index++) {
		const offset = image.indices[start + index] * 4;
		output.data.set(palette.subarray(offset, offset + 4), index * 4);
	}
}
