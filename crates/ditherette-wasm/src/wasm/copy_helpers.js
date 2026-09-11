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

// Rust supplies the exact byte offsets. This import only gathers their RGBA8 bytes.
export function gatherInput(destination, offsets, source, sourceLength) {
	if (inputLength(source) !== sourceLength) {
		throw new TypeError('Input storage changed during the call.');
	}
	const indices = new DataView(buffer.call(offsets), offset.call(offsets), length.call(offsets));
	const size = length.call(destination);
	const sourceOffset = offset.call(source);
	const destinationOffset = offset.call(destination);
	if ((sourceOffset | destinationOffset) % 4 === 0) {
		const input = new Uint32Array(buffer.call(source), sourceOffset, sourceLength / 4);
		const output = new Uint32Array(buffer.call(destination), destinationOffset, size / 4);
		for (let index = 0; index < output.length; index++) {
			output[index] = input[indices.getUint32(index * 4, true) / 4];
		}
		return;
	}
	for (let index = 0; index < size; index += 4) {
		const sourceIndex = indices.getUint32(index, true);
		destination[index] = source[sourceIndex];
		destination[index + 1] = source[sourceIndex + 1];
		destination[index + 2] = source[sourceIndex + 2];
		destination[index + 3] = source[sourceIndex + 3];
	}
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
