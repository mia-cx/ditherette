import { cp, rm } from 'node:fs/promises';

for (const variant of ['scalar', 'threads']) {
	const destination = new URL(`../dist/wasm/${variant}/`, import.meta.url);
	await rm(destination, { recursive: true, force: true });
	await cp(
		new URL(`../../../crates/ditherette-wasm/dist/${variant}/`, import.meta.url),
		destination,
		{ recursive: true }
	);
}
await cp(new URL('../../../LICENSE', import.meta.url), new URL('../LICENSE', import.meta.url));
