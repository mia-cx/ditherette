// Stateless private imports. Rust declares every import with wasm_bindgen(catch).
const typedArrayPrototype = Object.getPrototypeOf(Uint8Array.prototype);
const length = Object.getOwnPropertyDescriptor(typedArrayPrototype, 'length').get;
const tag = Object.getOwnPropertyDescriptor(typedArrayPrototype, Symbol.toStringTag).get;

export function inputLength(source) {
	if (tag.call(source) !== 'Uint8Array') throw new TypeError('Expected Uint8Array.');
	return length.call(source);
}

export function copyInput(destination, source) {
	if (inputLength(source) !== length.call(destination)) {
		throw new TypeError('Input storage changed during the call.');
	}
	Uint8Array.prototype.set.call(destination, source);
}

export function completeResult(source, width, height, sink) {
	const data = new Uint8Array(length.call(source));
	Uint8Array.prototype.set.call(data, source);
	sink.value = { width, height, data };
}
