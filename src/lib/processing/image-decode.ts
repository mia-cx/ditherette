import { validateSourceBlob } from './image-metadata';
import { validateSourceImageSize } from './types';

export function browserCanProcessImages() {
	return typeof indexedDB !== 'undefined' && typeof createImageBitmap !== 'undefined';
}

function createDecodeCanvas(width: number, height: number) {
	if (typeof OffscreenCanvas !== 'undefined') return new OffscreenCanvas(width, height);
	if (typeof document !== 'undefined') {
		return Object.assign(document.createElement('canvas'), { width, height });
	}
	throw new Error('No canvas implementation is available for image decoding.');
}

export async function decodeBlob(
	blob: Blob,
	options: { validate?: boolean } = {}
): Promise<{ imageData: ImageData; width: number; height: number }> {
	if (options.validate !== false) await validateSourceBlob(blob);
	const bitmap = await createImageBitmap(blob);
	try {
		validateSourceImageSize(bitmap.width, bitmap.height);
		const canvas = createDecodeCanvas(bitmap.width, bitmap.height);
		const context = canvas.getContext('2d', { willReadFrequently: true });
		if (!context) throw new Error('Canvas 2D is not available');
		context.drawImage(bitmap, 0, 0);
		return {
			imageData: context.getImageData(0, 0, bitmap.width, bitmap.height),
			width: bitmap.width,
			height: bitmap.height
		};
	} finally {
		bitmap.close();
	}
}
