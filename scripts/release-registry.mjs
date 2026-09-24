import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { appendFileSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** A rerun may skip publication only when npm already serves the exact tested archive. */
export async function alreadyPublished(version, tarball, request = fetch) {
	const response = await request(
		`https://registry.npmjs.org/ditherette/${encodeURIComponent(version)}`
	);
	if (response.status === 404) return false;
	assert.ok(response.ok, `Registry lookup failed with HTTP ${response.status}.`);
	const manifest = await response.json();
	assert.equal(manifest.name, 'ditherette');
	assert.equal(manifest.version, version);
	const integrity = `sha512-${createHash('sha512').update(tarball).digest('base64')}`;
	assert.equal(
		manifest.dist?.integrity,
		integrity,
		'Published version differs from the tested tarball.'
	);
	return true;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const { version } = JSON.parse(readFileSync('packages/ditherette/package.json', 'utf8'));
	const exists = await alreadyPublished(version, readFileSync(process.argv[2]));
	console.log(`exists=${exists}`);
	if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `exists=${exists}\n`);
}
