// Caught by Rust's binding. The recipe crosses as JSON so JS receives a fresh, plain object.
export function completeRecipe(text, sink) {
	sink.value = JSON.parse(text);
}
