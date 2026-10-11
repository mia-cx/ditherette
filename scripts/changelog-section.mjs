import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Extract the release-notes body for `version` from a Changesets-format changelog. */
export function changelogSection(content, version) {
	const lines = content.split(/\r?\n/);
	const start = lines.indexOf(`## ${version}`);
	assert.ok(start !== -1, `CHANGELOG has no section for ${version}.`);
	const rest = lines
		.slice(start + 1)
		.join('\n')
		.trimStart();
	const end = rest.search(/^## /m);
	const section = (end === -1 ? rest : rest.slice(0, end)).trim();
	assert.notEqual(section.length, 0, `The CHANGELOG section for ${version} is empty.`);
	return section;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const [changelogPath, version] = process.argv.slice(2);
	if (!changelogPath || !version) {
		console.error('usage: changelog-section <path> <version>');
		process.exit(2);
	}
	process.stdout.write(`${changelogSection(readFileSync(changelogPath, 'utf8'), version)}\n`);
}
