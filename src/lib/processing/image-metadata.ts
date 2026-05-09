import { MAX_SOURCE_BYTES, isAcceptedImageType, validateSourceImageSize } from './types';

export type ImageDimensions = { width: number; height: number };

const HEADER_BYTES = 1024 * 1024;
const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const SOF_MARKERS = new Set([
	0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf
]);
const SOF_DIMENSION_BYTES = 7;

export async function validateSourceBlob(blob: Blob) {
	if (!isAcceptedImageType(blob.type)) {
		throw new Error('Choose a PNG, JPEG, WebP, or GIF image.');
	}
	if (blob.size > MAX_SOURCE_BYTES) {
		throw new Error(
			`Image file is too large. Maximum file size is ${Math.round(MAX_SOURCE_BYTES / 1024 / 1024)} MB.`
		);
	}
	const dimensions = await readImageDimensions(blob);
	validateSourceImageSize(dimensions.width, dimensions.height);
	return dimensions;
}

export async function readImageDimensions(blob: Blob): Promise<ImageDimensions> {
	const header = new Uint8Array(await blob.slice(0, HEADER_BYTES).arrayBuffer());
	switch (blob.type) {
		case 'image/png':
			return readPngDimensions(header);
		case 'image/gif':
			return readGifDimensions(header);
		case 'image/jpeg':
			return readJpegDimensions(header);
		case 'image/webp':
			return readWebpDimensions(header);
		default:
			throw new Error('Unsupported image type.');
	}
}

function readPngDimensions(header: Uint8Array): ImageDimensions {
	if (header.length < 33 || !PNG_SIGNATURE.every((byte, index) => header[index] === byte)) {
		throw new Error('PNG header could not be read safely.');
	}
	if (readU32BE(header, 8) !== 13 || text(header, 12, 4) !== 'IHDR') {
		throw new Error('PNG IHDR chunk could not be read safely.');
	}
	return { width: readU32BE(header, 16), height: readU32BE(header, 20) };
}

function readGifDimensions(header: Uint8Array): ImageDimensions {
	if (header.length < 10) throw new Error('GIF header could not be read safely.');
	const signature = text(header, 0, 6);
	if (signature !== 'GIF87a' && signature !== 'GIF89a') {
		throw new Error('GIF header could not be read safely.');
	}
	return { width: readU16LE(header, 6), height: readU16LE(header, 8) };
}

function readJpegDimensions(header: Uint8Array): ImageDimensions {
	if (header.length < 4 || header[0] !== 0xff || header[1] !== 0xd8) {
		throw new Error('JPEG header could not be read safely.');
	}
	let offset = 2;
	while (offset + 4 < header.length) {
		while (header[offset] === 0xff) offset++;
		const marker = header[offset++];
		if (marker === 0xd9 || marker === 0xda) break;
		const length = readU16BE(header, offset);
		if (length < 2 || offset + length > header.length) break;
		if (SOF_MARKERS.has(marker)) {
			if (length < SOF_DIMENSION_BYTES || offset + SOF_DIMENSION_BYTES > header.length) break;
			return { width: readU16BE(header, offset + 5), height: readU16BE(header, offset + 3) };
		}
		offset += length;
	}
	throw new Error('JPEG dimensions could not be read safely.');
}

function readWebpDimensions(header: Uint8Array): ImageDimensions {
	if (header.length < 30 || text(header, 0, 4) !== 'RIFF' || text(header, 8, 4) !== 'WEBP') {
		throw new Error('WebP header could not be read safely.');
	}
	let offset = 12;
	while (offset + 8 <= header.length) {
		const chunk = text(header, offset, 4);
		const size = readU32LE(header, offset + 4);
		const data = offset + 8;
		if (data + size > header.length) break;
		if (chunk === 'VP8X' && size >= 10) {
			return { width: readU24LE(header, data + 4) + 1, height: readU24LE(header, data + 7) + 1 };
		}
		if (chunk === 'VP8 ' && size >= 10) {
			if (header[data + 3] !== 0x9d || header[data + 4] !== 0x01 || header[data + 5] !== 0x2a)
				break;
			return {
				width: readU16LE(header, data + 6) & 0x3fff,
				height: readU16LE(header, data + 8) & 0x3fff
			};
		}
		if (chunk === 'VP8L' && size >= 5) {
			if (header[data] !== 0x2f) break;
			const bits = readU32LE(header, data + 1);
			return { width: (bits & 0x3fff) + 1, height: ((bits >> 14) & 0x3fff) + 1 };
		}
		offset = data + size + (size % 2);
	}
	throw new Error('WebP dimensions could not be read safely.');
}

function text(bytes: Uint8Array, offset: number, length: number) {
	return String.fromCharCode(...bytes.subarray(offset, offset + length));
}

function readU16BE(bytes: Uint8Array, offset: number) {
	return (bytes[offset]! << 8) | bytes[offset + 1]!;
}

function readU16LE(bytes: Uint8Array, offset: number) {
	return bytes[offset]! | (bytes[offset + 1]! << 8);
}

function readU24LE(bytes: Uint8Array, offset: number) {
	return bytes[offset]! | (bytes[offset + 1]! << 8) | (bytes[offset + 2]! << 16);
}

function readU32BE(bytes: Uint8Array, offset: number) {
	return (
		((bytes[offset]! << 24) |
			(bytes[offset + 1]! << 16) |
			(bytes[offset + 2]! << 8) |
			bytes[offset + 3]!) >>>
		0
	);
}

function readU32LE(bytes: Uint8Array, offset: number) {
	return (
		(bytes[offset]! |
			(bytes[offset + 1]! << 8) |
			(bytes[offset + 2]! << 16) |
			(bytes[offset + 3]! << 24)) >>>
		0
	);
}
