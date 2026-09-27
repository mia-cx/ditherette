import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	contentDigest,
	expectedFiles,
	FROZEN_ROOTS,
	GENERATOR,
	inventory,
	v1Files,
	verifyExtensionRoot
} from './content.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '../..');
const CHECKPOINT = join(ROOT, 'tools/spec-freeze/checkpoint.json');

/**
 * Record the working tree's reference additions as a new named extension.
 * Run once per reviewed reference domain: `node tools/spec-freeze/extend.mjs <name> <summary>`.
 * The trusted-base guard still requires maintainer approval for the resulting checkpoint edit.
 */
export function extend(root, checkpoint, name, summary) {
	if ((checkpoint.extensions ?? []).some((extension) => extension.name === name))
		throw new Error(`Extension already recorded: ${name}`);
	const recorded = new Map(expectedFiles(checkpoint).map((entry) => [entry.path, entry]));
	const actual = inventory(root, [...FROZEN_ROOTS, GENERATOR]);
	const found = new Map(actual.map((entry) => [entry.path, entry]));
	const changes = [...new Set([...recorded.keys(), ...found.keys()])]
		.sort()
		.filter((path) => JSON.stringify(recorded.get(path)) !== JSON.stringify(found.get(path)))
		.map((path) => ({ path, before: recorded.get(path) ?? null, after: found.get(path) ?? null }));
	if (changes.length === 0) throw new Error('No reference changes to record');
	verifyExtensionRoot(root, v1Files(checkpoint));
	const extensions = [
		...(checkpoint.extensions ?? []),
		{ name, summary, changes, contentSha256: contentDigest(actual) }
	];
	const next = { ...checkpoint, extensions };
	expectedFiles(next);
	return next;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
	const [name, summary] = process.argv.slice(2);
	if (!name || !summary) {
		console.error('Usage: node tools/spec-freeze/extend.mjs <name> <summary>');
		process.exit(1);
	}
	const next = extend(ROOT, JSON.parse(readFileSync(CHECKPOINT, 'utf8')), name, summary);
	writeFileSync(CHECKPOINT, `${JSON.stringify(next, null, '\t')}\n`);
	console.log(`Recorded ${name}: ${next.extensions.at(-1).changes.length} changes`);
}
