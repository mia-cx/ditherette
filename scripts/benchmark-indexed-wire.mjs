// Benchmark-only indexed transport. Public results and frozen serializers keep their existing shapes.
const ENCODING = 'indexed8-hex-v1';
const MAX_PIXELS = 8192 * 8192;
const HEX = Array.from({ length: 256 }, (_, byte) => byte.toString(16).padStart(2, '0'));
const WARNING_CODES = new Set(['palette-truncated', 'transparent-only', 'transparent-fallback']);

/** Only a declared capped case may replace its byte arrays with the compact wire envelope. */
export function usesIndexedWire(trial) {
	return trial.case.browser?.retained_output_limit_bytes !== undefined;
}

function keys(value, expected) {
	if (!value || Object.keys(value).sort().join(',') !== [...expected].sort().join(','))
		throw new Error('Invalid indexed wire metadata fields.');
}

function validate(metadata, dimensions) {
	keys(metadata, ['dimensions', 'pixels', 'warnings']);
	keys(metadata.dimensions, ['width', 'height']);
	const { width, height } = metadata.dimensions;
	const pixels = width * height;
	if (
		!Number.isSafeInteger(width) ||
		width < 1 ||
		!Number.isSafeInteger(height) ||
		height < 1 ||
		!Number.isSafeInteger(pixels) ||
		pixels > MAX_PIXELS ||
		(dimensions && (width !== dimensions.width || height !== dimensions.height))
	)
		throw new Error('Invalid indexed wire dimensions.');
	keys(metadata.pixels, ['format', 'indices', 'palette_rgba', 'transparent_index']);
	const { format, palette_rgba: palette, transparent_index: transparent } = metadata.pixels;
	if (
		format !== 'indexed8' ||
		!Array.isArray(palette) ||
		palette.length < 4 ||
		palette.length > 1024 ||
		palette.length % 4 !== 0 ||
		palette.some((byte) => !Number.isInteger(byte) || byte < 0 || byte > 255) ||
		(transparent !== null &&
			(!Number.isInteger(transparent) || transparent < 0 || transparent >= palette.length / 4))
	)
		throw new Error('Invalid indexed wire palette.');
	if (!Array.isArray(metadata.warnings) || metadata.warnings.length > 3)
		throw new Error('Invalid indexed wire warnings.');
	for (const warning of metadata.warnings) {
		keys(warning, ['code', 'message']);
		if (
			!WARNING_CODES.has(warning.code) ||
			typeof warning.message !== 'string' ||
			warning.message.length > 88
		)
			throw new Error('Invalid indexed wire warning.');
	}
	return pixels;
}

/** Encode outside timers, without creating a full-image JavaScript number array. */
export function encodeOutput(output) {
	const indices = output.pixels.indices;
	const metadata = {
		dimensions: { ...output.dimensions },
		pixels: { ...output.pixels, indices: [], palette_rgba: Array.from(output.pixels.palette_rgba) },
		warnings: output.warnings.map((warning) => ({ ...warning }))
	};
	const length = validate(metadata);
	if ((!Array.isArray(indices) && !(indices instanceof Uint8Array)) || indices.length !== length)
		throw new Error('Invalid indexed wire byte length.');
	const chunks = [];
	const chunkBytes = 16 * 1024;
	for (let start = 0; start < length; start += chunkBytes) {
		const chunk = [];
		for (let index = start; index < Math.min(length, start + chunkBytes); index++) {
			const byte = indices[index];
			if (!Number.isInteger(byte) || byte < 0 || byte >= metadata.pixels.palette_rgba.length / 4)
				throw new Error('Invalid indexed wire palette index.');
			chunk.push(HEX[byte]);
		}
		chunks.push(chunk.join(''));
	}
	return { wire_encoding: ENCODING, metadata, indices_hex: chunks.join('') };
}

/** Validate the envelope before allocating its exact typed index storage. */
export function decodeOutput(envelope, dimensions) {
	keys(envelope, ['wire_encoding', 'metadata', 'indices_hex']);
	if (envelope.wire_encoding !== ENCODING) throw new Error('Unknown indexed wire encoding.');
	const { metadata, indices_hex: hex } = envelope;
	const length = validate(metadata, dimensions);
	if (
		!Array.isArray(metadata.pixels.indices) ||
		metadata.pixels.indices.length !== 0 ||
		typeof hex !== 'string' ||
		hex.length !== length * 2
	)
		throw new Error('Invalid indexed wire byte length.');
	// Check all characters before allocation. Lowercase hex is the single canonical representation.
	for (let index = 0; index < hex.length; index++) {
		const code = hex.charCodeAt(index);
		if (!(code >= 48 && code <= 57) && !(code >= 97 && code <= 102))
			throw new Error('Invalid indexed wire hex.');
	}
	const indices = new Uint8Array(length);
	const nibble = (code) => (code <= 57 ? code - 48 : code - 87);
	for (let index = 0; index < length; index++) {
		const byte = nibble(hex.charCodeAt(index * 2)) * 16 + nibble(hex.charCodeAt(index * 2 + 1));
		if (byte >= metadata.pixels.palette_rgba.length / 4)
			throw new Error('Invalid indexed wire palette index.');
		indices[index] = byte;
	}
	return { ...metadata, pixels: { ...metadata.pixels, indices } };
}
