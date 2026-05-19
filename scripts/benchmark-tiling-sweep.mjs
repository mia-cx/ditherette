#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = stripOptionalLeadingSeparator(process.argv.slice(2));

if (args.includes('--help') || args.includes('-h')) {
	console.log(helpText());
	process.exit(0);
}

const result = spawnSync(
	'cargo',
	[
		'run',
		'--manifest-path',
		'crates/ditherette-wasm/Cargo.toml',
		'--release',
		'--features',
		'tiling',
		'--example',
		'tiling_sweep',
		'--',
		...args
	],
	{ cwd: root, stdio: 'inherit' }
);

process.exit(result.status ?? 1);

function stripOptionalLeadingSeparator(argv) {
	return argv[0] === '--' ? argv.slice(1) : argv;
}

function helpText() {
	return `Usage: pnpm bench:tiling-sweep --target resize:nearest [options]

Runs a generic row-band tiling policy sweep. Resize is the first supported
namespace; future targets can use the same <stage>:<kernel> shape.

Examples:
  pnpm bench:tiling-sweep --target resize:nearest
  pnpm bench:tiling-sweep --target resize:area --iterations 20
  pnpm bench:tiling-sweep --target resize:box --output benchmark-results/box-sweep.json

Options are forwarded to the Rust sweep example:
  --target TARGET             e.g. resize:nearest, resize:area, resize:box
  --image FILE                PNG fixture to decode before sweeping
  --output FILE               JSON result path
  --iterations N              Timed iterations per case
  --warmup-iterations N       Warmup iterations per case`;
}
