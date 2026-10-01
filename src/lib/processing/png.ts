import { validatePngExportImage } from './schemas';
import { validateSourceImageSize, type ProcessedImage } from './types';

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

/** Channels per pixel and the bit depths the PNG spec allows, by colour type. */
const PNG_COLOR_TYPES: Partial<Record<number, { channels: number; depths: readonly number[] }>> = {
	0: { channels: 1, depths: [1, 2, 4, 8, 16] }, // greyscale
	2: { channels: 3, depths: [8, 16] }, // truecolour
	3: { channels: 1, depths: [1, 2, 4, 8] }, // indexed
	4: { channels: 2, depths: [8, 16] }, // greyscale with alpha
	6: { channels: 4, depths: [8, 16] } // truecolour with alpha
};
const OPAQUE = 255;

type Predictor = (left: number, up: number, upLeft: number) => number;

const PREDICTORS: readonly Predictor[] = [
	() => 0,
	(left) => left,
	(_left, up) => up,
	(left, up) => (left + up) >> 1,
	(left, up, upLeft) => {
		const estimate = left + up - upLeft;
		const toLeft = Math.abs(estimate - left);
		const toUp = Math.abs(estimate - up);
		const toUpLeft = Math.abs(estimate - upLeft);
		if (toLeft <= toUp && toLeft <= toUpLeft) return left;
		return toUp <= toUpLeft ? up : upLeft;
	}
];

export type DecodedPixels = { data: Uint8ClampedArray<ArrayBuffer>; width: number; height: number };

function readU32(bytes: Uint8Array, offset: number) {
	return (
		((bytes[offset]! << 24) |
			(bytes[offset + 1]! << 16) |
			(bytes[offset + 2]! << 8) |
			bytes[offset + 3]!) >>>
		0
	);
}

async function inflate(chunks: Uint8Array<ArrayBuffer>[]) {
	const stream = new Blob(chunks).stream().pipeThrough(new DecompressionStream('deflate'));
	return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** Reverses the per-row PNG filters and returns the rows without their filter bytes. */
function unfilter(raw: Uint8Array, stride: number, height: number, distance: number) {
	if (raw.length < (stride + 1) * height) throw new Error('PNG image data is truncated.');
	const rows = new Uint8Array(stride * height);
	for (let y = 0; y < height; y++) {
		const predict = PREDICTORS[raw[y * (stride + 1)]!];
		if (!predict) throw new Error('PNG image data uses an unknown filter.');
		const input = y * (stride + 1) + 1;
		const row = y * stride;
		const above = row - stride;
		for (let x = 0; x < stride; x++) {
			const left = x >= distance ? rows[row + x - distance]! : 0;
			const up = y > 0 ? rows[above + x]! : 0;
			const upLeft = y > 0 && x >= distance ? rows[above + x - distance]! : 0;
			rows[row + x] = raw[input + x]! + predict(left, up, upLeft);
		}
	}
	return rows;
}

/** Reads sample `index` of the row starting at byte `row`, at its stored bit depth. */
function sampleReader(rows: Uint8Array, bitDepth: number) {
	if (bitDepth === 8) return (row: number, index: number) => rows[row + index]!;
	if (bitDepth === 16) {
		return (row: number, index: number) =>
			(rows[row + index * 2]! << 8) | rows[row + index * 2 + 1]!;
	}
	const mask = (1 << bitDepth) - 1;
	return (row: number, index: number) => {
		const bit = index * bitDepth;
		return (rows[row + (bit >> 3)]! >> (8 - bitDepth - (bit & 7))) & mask;
	};
}

/**
 * Decodes a PNG to straight RGBA without a canvas, so the bytes match the file exactly.
 *
 * Returns undefined for interlaced PNGs and for PNGs a browser would colour-manage (an ICC
 * profile, or gamma or chromaticities without an sRGB chunk). The caller then lets the
 * browser decode them, so their colours still match the `<img>` preview.
 */
export async function decodePng(
	bytes: Uint8Array<ArrayBuffer>
): Promise<DecodedPixels | undefined> {
	if (!PNG_SIGNATURE.every((byte, index) => bytes[index] === byte)) {
		throw new Error('PNG signature is invalid.');
	}
	const chunks = new Map<string, Uint8Array<ArrayBuffer>>();
	const imageData: Uint8Array<ArrayBuffer>[] = [];
	for (let offset = PNG_SIGNATURE.length; offset + 8 <= bytes.length; ) {
		const length = readU32(bytes, offset);
		const type = String.fromCharCode(...bytes.subarray(offset + 4, offset + 8));
		const data = bytes.subarray(offset + 8, offset + 8 + length);
		offset += length + 12;
		if (type === 'IEND') break;
		if (type === 'IDAT') imageData.push(data);
		else if (!chunks.has(type)) chunks.set(type, data);
	}

	const header = chunks.get('IHDR');
	if (!header || header.length < 13) throw new Error('PNG header is missing.');
	const width = readU32(header, 0);
	const height = readU32(header, 4);
	const bitDepth = header[8]!;
	const colorType = header[9]!;
	const format = PNG_COLOR_TYPES[colorType];
	if (!format?.depths.includes(bitDepth)) throw new Error('PNG colour type is invalid.');
	validateSourceImageSize(width, height);
	const interlaced = header[12] !== 0;
	const colorManaged =
		chunks.has('iCCP') || (!chunks.has('sRGB') && (chunks.has('gAMA') || chunks.has('cHRM')));
	if (interlaced || colorManaged) return undefined;

	const palette = chunks.get('PLTE');
	if (colorType === 3 && !palette) throw new Error('PNG palette is missing.');
	const transparency = chunks.get('tRNS');
	const { channels } = format;
	const stride = Math.ceil((width * channels * bitDepth) / 8);
	const filterDistance = Math.max(1, (channels * bitDepth) >> 3);
	const rows = unfilter(await inflate(imageData), stride, height, filterDistance);
	const read = sampleReader(rows, bitDepth);
	const maxSample = (1 << bitDepth) - 1;
	const toByte = (sample: number) => (bitDepth === 16 ? sample >> 8 : (sample * 255) / maxSample);
	// Greyscale and truecolour tRNS name one colour, as 16-bit samples, that is fully transparent.
	const key =
		transparency && colorType !== 3
			? Array.from(
					{ length: channels },
					(_, c) => (transparency[c * 2]! << 8) | transparency[c * 2 + 1]!
				)
			: undefined;
	const isKey = (row: number, first: number) =>
		key?.every((sample, c) => read(row, first + c) === sample) ?? false;

	const data = new Uint8ClampedArray(width * height * 4);
	for (let y = 0; y < height; y++) {
		const row = y * stride;
		for (let x = 0; x < width; x++) {
			const out = (y * width + x) * 4;
			const first = x * channels;
			if (colorType === 3) {
				// Indices past the palette decode as opaque black, as browsers do.
				const index = read(row, first);
				data[out] = palette![index * 3] ?? 0;
				data[out + 1] = palette![index * 3 + 1] ?? 0;
				data[out + 2] = palette![index * 3 + 2] ?? 0;
				data[out + 3] = transparency?.[index] ?? OPAQUE;
			} else if (channels <= 2) {
				const grey = toByte(read(row, first));
				data[out] = data[out + 1] = data[out + 2] = grey;
				data[out + 3] =
					channels === 2 ? toByte(read(row, first + 1)) : isKey(row, first) ? 0 : OPAQUE;
			} else {
				data[out] = toByte(read(row, first));
				data[out + 1] = toByte(read(row, first + 1));
				data[out + 2] = toByte(read(row, first + 2));
				data[out + 3] =
					channels === 4 ? toByte(read(row, first + 3)) : isKey(row, first) ? 0 : OPAQUE;
			}
		}
	}
	return { data, width, height };
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
