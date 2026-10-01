import { validateSourceBlob } from './image-metadata';
import { decodePng, type DecodedPixels } from './png';
import { validateSourceImageSize } from './types';

declare global {
	// Chromium reports EXIF orientation on the frame instead of applying it to the pixels.
	// Not yet in TypeScript's DOM types.
	interface VideoFrame {
		readonly rotation?: number;
		readonly flip?: boolean;
	}
}

export type DecodedImage = {
	imageData: ImageData;
	width: number;
	height: number;
	/** Set when only canvas readback could decode the image and this browser alters its pixels. */
	warning?: string;
};

export const NOISY_READBACK_WARNING =
	"Your browser's fingerprinting protection changes image pixels, so source colours may be slightly off.";

const RGBA_BYTES = 4;
const READBACK_CHECK_SIZE = 16;

export function browserCanProcessImages() {
	return typeof indexedDB !== 'undefined' && typeof createImageBitmap !== 'undefined';
}

/**
 * Decodes a source image to straight sRGB RGBA, with EXIF orientation applied.
 *
 * Tries decoders that never read pixels back from a canvas first, because privacy browsers add
 * noise to canvas readback: WebCodecs `ImageDecoder`, then the built-in PNG decoder. Only when
 * neither can decode the image exactly does it draw to a canvas, and then it warns if readback
 * is noisy.
 */
export async function decodeBlob(
	blob: Blob,
	options: { validate?: boolean } = {}
): Promise<DecodedImage> {
	if (options.validate !== false) await validateSourceBlob(blob);
	const pixels = await decodeExactly(new Uint8Array(await blob.arrayBuffer()), blob.type);
	if (!pixels) return decodeWithCanvas(blob);
	const { data, width, height } = pixels;
	return { imageData: new ImageData(data, width, height), width, height };
}

async function decodeExactly(bytes: Uint8Array<ArrayBuffer>, type: string) {
	const decoded = await decodeWithImageDecoder(bytes, type);
	if (decoded && (!hasPartialAlpha(decoded.data) || (await imageDecoderKeepsAlpha()))) {
		return decoded;
	}
	return type === 'image/png' ? decodePng(bytes) : undefined;
}

function hasPartialAlpha(data: Uint8ClampedArray) {
	for (let alpha = 3; alpha < data.length; alpha += RGBA_BYTES) {
		if (data[alpha] !== 0 && data[alpha] !== 255) return true;
	}
	return false;
}

/** A 1x1 RGBA PNG of rgba(5, 6, 7, 128). */
const PARTIAL_ALPHA_PNG =
	'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNgZWNvAAAAuQCTmr2QCQAAAABJRU5ErkJggg==';
let imageDecoderAlphaCheck: Promise<boolean> | undefined;

/**
 * Firefox's ImageDecoder hands back premultiplied pixels, which darkens partial alpha.
 * Checks once per session whether a known semi-transparent pixel survives unchanged.
 */
function imageDecoderKeepsAlpha() {
	imageDecoderAlphaCheck ??= decodeWithImageDecoder(
		Uint8Array.from(atob(PARTIAL_ALPHA_PNG), (char) => char.charCodeAt(0)),
		'image/png'
	).then((pixels) => pixels?.data.join() === '5,6,7,128');
	return imageDecoderAlphaCheck;
}

async function decodeWithImageDecoder(bytes: Uint8Array<ArrayBuffer>, type: string) {
	if (typeof ImageDecoder === 'undefined' || !(await ImageDecoder.isTypeSupported(type))) {
		return undefined;
	}
	// 'default' colour conversion matches createImageBitmap and the <img> preview.
	// It leaves untagged and sRGB images unchanged.
	const decoder = new ImageDecoder({
		data: bytes,
		type,
		colorSpaceConversion: 'default',
		preferAnimation: false
	});
	try {
		const { image } = await decoder.decode();
		try {
			return await copyFrame(image);
		} finally {
			image.close();
		}
	} finally {
		decoder.close();
	}
}

/**
 * Copies a frame to RGBA. Returns undefined when the frame is not packed sRGB and this browser
 * cannot convert it, as older builds cannot for YUV frames such as JPEG.
 */
async function copyFrame(frame: VideoFrame): Promise<DecodedPixels | undefined> {
	const { width, height } = frame.visibleRect!;
	validateSourceImageSize(width, height);
	const data = new Uint8ClampedArray(width * height * RGBA_BYTES);
	const layout = [{ offset: 0, stride: width * RGBA_BYTES }];
	// Copy packed sRGB frames untouched: Chromium's RGBA conversion round-trips through
	// premultiplied alpha, which loses colour under low alpha.
	if (isPackedRgb(frame.format) && isSrgb(frame.colorSpace)) {
		await frame.copyTo(data, { layout });
		toRgba(data, frame.format);
	} else if (copyToConvertsFormat(frame)) {
		await frame.copyTo(data, { format: 'RGBA', colorSpace: 'srgb', layout });
	} else return undefined;
	return orient(data, width, height, frame.rotation ?? 0, frame.flip ?? false);
}

function isPackedRgb(format: VideoPixelFormat | null): format is 'RGBA' | 'RGBX' | 'BGRA' | 'BGRX' {
	return format === 'RGBA' || format === 'RGBX' || format === 'BGRA' || format === 'BGRX';
}

/** Unset fields count as sRGB: Firefox reports none, and its canvas draws those frames as sRGB. */
function isSrgb({ primaries, transfer }: VideoColorSpace) {
	return (primaries ?? 'bt709') === 'bt709' && (transfer ?? 'iec61966-2-1') === 'iec61966-2-1';
}

function toRgba(data: Uint8ClampedArray, format: VideoPixelFormat) {
	const swapRedBlue = format === 'BGRA' || format === 'BGRX';
	const opaque = format === 'RGBX' || format === 'BGRX';
	if (!swapRedBlue && !opaque) return;
	for (let offset = 0; offset < data.length; offset += RGBA_BYTES) {
		if (swapRedBlue) {
			const blue = data[offset]!;
			data[offset] = data[offset + 2]!;
			data[offset + 2] = blue;
		}
		if (opaque) data[offset + 3] = 255;
	}
}

/**
 * Older WebCodecs builds ignore the copyTo `format` option and copy the native layout, which
 * would swap channels silently. WebIDL reads every member a browser knows, so a getter shows
 * whether the option is understood.
 */
function copyToConvertsFormat(frame: VideoFrame) {
	let understood = false;
	frame.allocationSize({
		get format() {
			understood = true;
			return 'RGBA' as const;
		}
	});
	return understood;
}

/**
 * Applies a frame's clockwise rotation, then its horizontal flip, as the WebCodecs spec orders
 * them. Firefox applies EXIF orientation while decoding and reports neither.
 */
function orient(
	data: Uint8ClampedArray<ArrayBuffer>,
	width: number,
	height: number,
	rotation: number,
	flip: boolean
): DecodedPixels {
	if (rotation === 0 && !flip) return { data, width, height };
	const turned = rotation === 90 || rotation === 270;
	const outWidth = turned ? height : width;
	const outHeight = turned ? width : height;
	const source = new Uint32Array(data.buffer);
	const output = new Uint32Array(source.length);
	for (let y = 0; y < outHeight; y++) {
		for (let x = 0; x < outWidth; x++) {
			const unflipped = flip ? outWidth - 1 - x : x;
			const [sourceX, sourceY] =
				rotation === 90
					? [y, height - 1 - unflipped]
					: rotation === 180
						? [width - 1 - unflipped, height - 1 - y]
						: rotation === 270
							? [width - 1 - y, unflipped]
							: [unflipped, y];
			output[y * outWidth + x] = source[sourceY * width + sourceX]!;
		}
	}
	return { data: new Uint8ClampedArray(output.buffer), width: outWidth, height: outHeight };
}

async function decodeWithCanvas(blob: Blob): Promise<DecodedImage> {
	const bitmap = await createImageBitmap(blob);
	try {
		const { width, height } = bitmap;
		validateSourceImageSize(width, height);
		const imageData = readBitmap(bitmap);
		const warning = (await canvasReadbackIsExact()) ? undefined : NOISY_READBACK_WARNING;
		return { imageData, width, height, warning };
	} finally {
		bitmap.close();
	}
}

function readBitmap(bitmap: ImageBitmap) {
	const context = createDecodeContext(bitmap.width, bitmap.height);
	if (!context) throw new Error('Canvas 2D is not available');
	context.drawImage(bitmap, 0, 0);
	return context.getImageData(0, 0, bitmap.width, bitmap.height);
}

// Typed, so the HTML canvas picks its '2d' overload rather than the catch-all string one.
const decodeContextSettings: CanvasRenderingContext2DSettings = { willReadFrequently: true };

function createDecodeContext(width: number, height: number) {
	if (typeof OffscreenCanvas !== 'undefined')
		return new OffscreenCanvas(width, height).getContext('2d', decodeContextSettings);
	if (typeof document !== 'undefined') {
		const canvas = Object.assign(document.createElement('canvas'), { width, height });
		return canvas.getContext('2d', decodeContextSettings);
	}
	throw new Error('No canvas implementation is available for image decoding.');
}

/** Draws a known opaque pattern through the decode canvas and compares every byte read back. */
async function canvasReadbackIsExact() {
	const expected = new Uint8ClampedArray(READBACK_CHECK_SIZE * READBACK_CHECK_SIZE * RGBA_BYTES);
	for (let offset = 0; offset < expected.length; offset += RGBA_BYTES) {
		expected[offset] = (offset * 37) & 0xff;
		expected[offset + 1] = (offset * 91 + 50) & 0xff;
		expected[offset + 2] = (offset * 13 + 120) & 0xff;
		expected[offset + 3] = 255;
	}
	const bitmap = await createImageBitmap(
		new ImageData(expected, READBACK_CHECK_SIZE, READBACK_CHECK_SIZE)
	);
	try {
		const actual = readBitmap(bitmap).data;
		return actual.every((value, index) => value === expected[index]);
	} finally {
		bitmap.close();
	}
}
