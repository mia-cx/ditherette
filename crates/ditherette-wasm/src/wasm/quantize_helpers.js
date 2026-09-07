// Stateless private imports. Every invocation, including string decoding, is caught by Rust's binding.
import { inputLength } from './copy_helpers.js';
export function paletteLength(palette) {
	if (!Array.isArray(palette)) throw new TypeError('Expected normalized palette codes.');
	return palette.length;
}

export function paletteEntry(palette, index) {
	const value = palette[index];
	if (typeof value !== 'number') throw new TypeError('Expected a numeric palette code.');
	return value;
}

const codes = ['', 'palette-truncated', 'transparent-only', 'transparent-fallback'];

export function completeIndexedResult(source, width, height, rgba, transparentIndex,
	firstCode, firstMessage, secondCode, secondMessage, sink) {
	const indices = new Uint8Array(inputLength(source));
	Uint8Array.prototype.set.call(indices, source);
	const palette = new Uint8Array(inputLength(rgba));
	Uint8Array.prototype.set.call(palette, rgba);
	const warnings = [];
	if (firstCode) warnings.push({ code: codes[firstCode], message: firstMessage });
	if (secondCode) warnings.push({ code: codes[secondCode], message: secondMessage });
	sink.value = { width, height, indices,
		palette: { rgba: palette, transparentIndex: transparentIndex < 0 ? null : transparentIndex }, warnings };
}
