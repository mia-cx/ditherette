import { readFile } from 'node:fs/promises';

const { version } = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));
const crate = await readFile(
	new URL('../../../crates/ditherette-wasm/Cargo.toml', import.meta.url),
	'utf8'
);
const crateVersion = crate.match(/^version = "([^"]+)"$/m)?.[1];
if (version !== crateVersion) {
	throw new Error(`Package version ${version} differs from Rust crate version ${crateVersion}.`);
}
