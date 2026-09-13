import { cp, rm } from 'node:fs/promises';

const variant = process.argv[2];
if (variant !== 'scalar' && variant !== 'threads') {
	throw new Error('Expected scalar or threads.');
}

// Keep the existing website and benchmark URLs until their package migration.
const directory = variant === 'scalar' ? 'ditherette-wasm' : 'ditherette-wasm-threads';
const destination = new URL(`../static/wasm/${directory}/`, import.meta.url);
await rm(destination, { recursive: true, force: true });
await cp(new URL(`../crates/ditherette-wasm/dist/${variant}/`, import.meta.url), destination, {
	recursive: true
});
