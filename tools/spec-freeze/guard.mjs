import { existsSync, lstatSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { inventory, verifyContent } from './content.mjs';
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

/** Execute this function from trusted base code, not a candidate-supplied implementation. */
export function verifyPolicy(root, trustedRoot) {
	const paths = (directory) => [
		...policyPaths(directory),
		...(existsSync(join(directory, WORKFLOW)) ? [WORKFLOW] : [])
	];
	if (
		JSON.stringify(inventory(root, paths(root))) !==
		JSON.stringify(inventory(trustedRoot, paths(trustedRoot)))
	) {
		throw new Error(
			'Candidate changed trusted freeze policy/checkpoint/workflow; explicit maintainer policy approval is required'
		);
	}
}

export function verify(root, trustedRoot) {
	verifyPolicy(root, trustedRoot);
	const checkpoint = JSON.parse(readFileSync(join(trustedRoot, POLICY, 'checkpoint.json')));
	const dependencies = JSON.parse(readFileSync(join(trustedRoot, POLICY, 'dependencies.json')));
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
			options: { root: { type: 'string' }, 'trusted-root': { type: 'string' } }
		});
		const trustedRoot = resolve(values['trusted-root'] ?? join(HERE, '../..'));
		const root = resolve(values.root ?? trustedRoot);
		console.log(JSON.stringify(verify(root, trustedRoot), null, 2));
	} catch (error) {
		console.error(error.stderr?.toString().trim() || error.message);
		process.exitCode = 1;
	}
}
