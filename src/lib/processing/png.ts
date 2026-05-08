import { validatePngExportImage } from './schemas';
import type { ProcessedImage } from './types';

const PNG_SIGNATURE = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]);

let crcTable: Uint32Array | undefined;

function makeCrcTable() {
	const table = new Uint32Array(256);
	for (let n = 0; n < 256; n++) {
		let c = n;
		for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
		table[n] = c >>> 0;
	}
	return table;
}

function crc32(bytes: Uint8Array) {
	const table = (crcTable ??= makeCrcTable());
	let crc = 0xffffffff;
	for (const byte of bytes) crc = table[(crc ^ byte) & 0xff] ^ (crc >>> 8);
	return (crc ^ 0xffffffff) >>> 0;
}

function adler32(bytes: Uint8Array) {
	let a = 1;
	let b = 0;
	for (const byte of bytes) {
		a = (a + byte) % 65521;
		b = (b + a) % 65521;
	}
	return ((b << 16) | a) >>> 0;
}

function writeU32(bytes: Uint8Array, offset: number, value: number) {
	bytes[offset] = (value >>> 24) & 0xff;
	bytes[offset + 1] = (value >>> 16) & 0xff;
	bytes[offset + 2] = (value >>> 8) & 0xff;
	bytes[offset + 3] = value & 0xff;
}

function ascii(value: string) {
	return Uint8Array.from(value, (char) => char.charCodeAt(0));
}

function chunk(type: string, data: Uint8Array) {
	const typeBytes = ascii(type);
	const output = new Uint8Array(12 + data.length);
	writeU32(output, 0, data.length);
	output.set(typeBytes, 4);
	output.set(data, 8);
	writeU32(output, output.length - 4, crc32(output.subarray(4, output.length - 4)));
	return output;
}

function zlibStore(bytes: Uint8Array) {
	const blockCount = Math.ceil(bytes.length / 65535) || 1;
	const output = new Uint8Array(2 + bytes.length + blockCount * 5 + 4);
	let offset = 0;
	output[offset++] = 0x78;
	output[offset++] = 0x01;
	let sourceOffset = 0;
	for (let block = 0; block < blockCount; block++) {
		const length = Math.min(65535, bytes.length - sourceOffset);
		const final = block === blockCount - 1 ? 1 : 0;
		output[offset++] = final;
		output[offset++] = length & 0xff;
		output[offset++] = (length >>> 8) & 0xff;
		const inverse = ~length & 0xffff;
		output[offset++] = inverse & 0xff;
		output[offset++] = (inverse >>> 8) & 0xff;
		output.set(bytes.subarray(sourceOffset, sourceOffset + length), offset);
		offset += length;
		sourceOffset += length;
	}
	writeU32(output, offset, adler32(bytes));
	return output;
}

export function encodeIndexedPng(image: ProcessedImage): Blob {
	const safeImage = validatePngExportImage(image);

	const ihdr = new Uint8Array(13);
	writeU32(ihdr, 0, safeImage.width);
	writeU32(ihdr, 4, safeImage.height);
	ihdr[8] = 8; // bit depth
	ihdr[9] = 3; // indexed color
	ihdr[10] = 0; // deflate
	ihdr[11] = 0; // adaptive filters
	ihdr[12] = 0; // no interlace

	const plte = new Uint8Array(safeImage.palette.length * 3);
	const trns = new Uint8Array(safeImage.palette.length);
	for (let i = 0; i < safeImage.palette.length; i++) {
		const color = safeImage.palette[i];
		const base = i * 3;
		plte[base] = color.rgb?.r ?? 0;
		plte[base + 1] = color.rgb?.g ?? 0;
		plte[base + 2] = color.rgb?.b ?? 0;
		trns[i] = color.kind === 'transparent' ? 0 : 255;
	}

	const rows = new Uint8Array((safeImage.width + 1) * safeImage.height);
	for (let y = 0; y < safeImage.height; y++) {
		const rowOffset = y * (safeImage.width + 1);
		rows[rowOffset] = 0;
		rows.set(
			safeImage.indices.subarray(y * safeImage.width, (y + 1) * safeImage.width),
			rowOffset + 1
		);
	}

	return new Blob(
		[
			PNG_SIGNATURE,
			chunk('IHDR', ihdr),
			chunk('PLTE', plte),
			chunk('tRNS', trns),
			chunk('IDAT', zlibStore(rows)),
			chunk('IEND', new Uint8Array())
		],
		{ type: 'image/png' }
	);
}

export function downloadIndexedPng(image: ProcessedImage, filename = 'ditherette.png') {
	const blob = encodeIndexedPng(image);
	const url = URL.createObjectURL(blob);
	const link = document.createElement('a');
	link.href = url;
	link.download = filename;
	link.click();
	setTimeout(() => URL.revokeObjectURL(url), 0);
}
