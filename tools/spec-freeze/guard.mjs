import { existsSync, lstatSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { inventory, sha256, verifyContent } from './content.mjs';
import {
	isolatedCheck,
	TARGETS,
	verifyBuildConfiguration,
	verifyCompiler,
	verifyDependencies,
	verifySyntax
} from './build.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
export const POLICY = 'tools/spec-freeze';
export const WORKFLOW = '.github/workflows/spec-freeze.yml';

function policyPaths(root, relative = POLICY) {
	if (relative === `${POLICY}/syntax/target` || relative === `${POLICY}/README.md`) return [];
	const stat = lstatSync(join(root, relative));
	if (stat.isSymbolicLink()) throw new Error(`Symlink in guard policy: ${relative}`);
	if (stat.isFile()) return [relative];
	return readdirSync(join(root, relative))
		.sort()
		.flatMap((name) => policyPaths(root, `${relative}/${name}`));
}

function policyInventory(directory) {
	return inventory(directory, [
		...policyPaths(directory),
		...(existsSync(join(directory, WORKFLOW)) ? [WORKFLOW] : [])
	]);
}

/** Identity of a checkout's whole freeze policy: checker, checkpoint, dependency record, workflow. */
export function policyDigest(root) {
	return sha256(JSON.stringify(policyInventory(root)));
}

/**
 * Execute this function from trusted base code, not a candidate-supplied implementation.
 * Returns the checkout whose policy data applies: the trusted base, or the candidate when a
 * maintainer approved exactly its policy digest. Checking code always stays the trusted base's.
 */
export function verifyPolicy(root, trustedRoot, approved = []) {
	if (JSON.stringify(policyInventory(root)) === JSON.stringify(policyInventory(trustedRoot)))
		return trustedRoot;
	const digest = policyDigest(root);
	if (approved.includes(digest)) return root;
	throw new Error(
		`Candidate changed trusted freeze policy/checkpoint/workflow; explicit maintainer policy approval is required.\nTo approve exactly this policy, a maintainer comments on the pull request:\n/approve-freeze sha256:${digest}`
	);
}

export function verify(root, trustedRoot, approved = []) {
	const policyRoot = verifyPolicy(root, trustedRoot, approved);
	const checkpoint = JSON.parse(readFileSync(join(policyRoot, POLICY, 'checkpoint.json')));
	const dependencies = JSON.parse(readFileSync(join(policyRoot, POLICY, 'dependencies.json')));
	const identity = verifyContent(root, checkpoint);
	verifyCompiler();
	verifyBuildConfiguration(root);
	verifySyntax(root);
	verifyDependencies(root, dependencies);
	for (const target of TARGETS) {
		for (const role of ['spec', 'prod']) isolatedCheck(root, role, target);
	}
	isolatedCheck(root, 'prod', TARGETS[0], true);
	return identity;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	try {
		const { values } = parseArgs({
			options: {
				root: { type: 'string' },
				'trusted-root': { type: 'string' },
				approved: { type: 'string', multiple: true }
			}
		});
		const trustedRoot = resolve(values['trusted-root'] ?? join(HERE, '../..'));
		const root = resolve(values.root ?? trustedRoot);
		const approved = (values.approved ?? []).map((value) => value.replace(/^sha256:/, ''));
		console.log(JSON.stringify(verify(root, trustedRoot, approved), null, 2));
	} catch (error) {
		console.error(error.stderr?.toString().trim() || error.message);
		process.exitCode = 1;
	}
}
