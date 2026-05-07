import type { ProcessedImage } from './types';

const LITTLE_ENDIAN = new Uint8Array(new Uint32Array([0x11223344]).buffer)[0] === 0x44;

export function processedToImageData(image: ProcessedImage): ImageData {
	const output = new ImageData(image.width, image.height);
	if (LITTLE_ENDIAN) {
		const rgba = new Uint32Array(output.data.buffer);
		const palette = new Uint32Array(image.palette.length);
		for (let i = 0; i < image.palette.length; i++) {
			const color = image.palette[i];
			palette[i] =
				!color || color.kind === 'transparent'
					? 0
					: (255 << 24) |
						((color.rgb?.b ?? 0) << 16) |
						((color.rgb?.g ?? 0) << 8) |
						(color.rgb?.r ?? 0);
		}
		for (let index = 0; index < image.indices.length; index++)
			rgba[index] = palette[image.indices[index]]!;
		return output;
	}

	for (let index = 0; index < image.indices.length; index++) {
		const color = image.palette[image.indices[index]];
		const offset = index * 4;
		if (!color || color.kind === 'transparent') {
			output.data[offset] = 0;
			output.data[offset + 1] = 0;
			output.data[offset + 2] = 0;
			output.data[offset + 3] = 0;
			continue;
		}
		output.data[offset] = color.rgb?.r ?? 0;
		output.data[offset + 1] = color.rgb?.g ?? 0;
		output.data[offset + 2] = color.rgb?.b ?? 0;
		output.data[offset + 3] = 255;
	}
	return output;
}

export function drawImageDataToCanvas(canvas: HTMLCanvasElement, image: ImageData) {
	canvas.width = image.width;
	canvas.height = image.height;
	const context = canvas.getContext('2d');
	if (!context) throw new Error('Canvas 2D is not available');
	context.putImageData(image, 0, 0);
}
