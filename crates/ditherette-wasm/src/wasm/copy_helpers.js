// Stateless private imports. Rust declares every import with wasm_bindgen(catch).
const typedArrayPrototype = Object.getPrototypeOf(Uint8Array.prototype);
const length = Object.getOwnPropertyDescriptor(typedArrayPrototype, 'length').get;
const tag = Object.getOwnPropertyDescriptor(typedArrayPrototype, Symbol.toStringTag).get;
const buffer = Object.getOwnPropertyDescriptor(typedArrayPrototype, 'buffer').get;
const offset = Object.getOwnPropertyDescriptor(typedArrayPrototype, 'byteOffset').get;

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

// Compare the current view's bytes, including its offset, before trusting the owned snapshot.
export function snapshotInput(destination, source, compare) {
	const size = inputLength(source);
	if (size !== length.call(destination)) {
		throw new TypeError('Input storage changed during the call.');
	}
	if (compare) {
		let index = 0;
		const destinationOffset = offset.call(destination);
		const sourceOffset = offset.call(source);
		// Aligned views compare four bytes per iteration without changing equality semantics.
		if ((destinationOffset | sourceOffset) % 4 === 0) {
			const words = Math.floor(size / 4);
			const left = new Uint32Array(buffer.call(destination), destinationOffset, words);
			const right = new Uint32Array(buffer.call(source), sourceOffset, words);
			while (index < words && left[index] === right[index]) index++;
			if (index < words) {
				Uint8Array.prototype.set.call(destination, source);
				return false;
			}
			index *= 4;
		}
		while (index < size && destination[index] === source[index]) index++;
		if (index === size) return true;
	}
	Uint8Array.prototype.set.call(destination, source);
	return false;
}

export function completeResult(source, width, height, sink) {
	const data = new Uint8Array(length.call(source));
	Uint8Array.prototype.set.call(data, source);
	sink.value = { width, height, data };
}
