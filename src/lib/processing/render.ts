import { validateProcessedImage } from './schemas';
import type { ProcessedImage } from './types';

const LITTLE_ENDIAN = new Uint8Array(new Uint32Array([0x11223344]).buffer)[0] === 0x44;

export function processedToImageData(image: ProcessedImage): ImageData {
	const safeImage = validateProcessedImage(image);
	const output = new ImageData(safeImage.width, safeImage.height);
	if (LITTLE_ENDIAN) {
		const rgba = new Uint32Array(output.data.buffer);
		const palette = new Uint32Array(safeImage.palette.length);
		for (let i = 0; i < safeImage.palette.length; i++) {
			const color = safeImage.palette[i];
			palette[i] =
				!color || color.kind === 'transparent'
					? 0
					: (255 << 24) |
						((color.rgb?.b ?? 0) << 16) |
						((color.rgb?.g ?? 0) << 8) |
						(color.rgb?.r ?? 0);
		}
		for (let index = 0; index < safeImage.indices.length; index++)
			rgba[index] = palette[safeImage.indices[index]]!;
		return output;
	}

	for (let index = 0; index < safeImage.indices.length; index++) {
		const color = safeImage.palette[safeImage.indices[index]];
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
