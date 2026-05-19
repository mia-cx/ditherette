#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const resizeFilters = new Set([
	'nearest',
	'nearest_aa',
	'bilinear',
	'trilinear',
	'bicubic',
	'bicubic_2',
	'lanczos2',
	'lanczos2_scale_aware',
	'lanczos3',
	'lanczos3_2',
	'lanczos3_scale_aware',
	'area',
	'box',
	'antialias'
]);
const implementations = new Set(['reference', 'scalar', 'scalar_2', 'tiling']);
const defaultFixtureNames = ['Celeste_Insta_selfie.png', 'Celeste_box_art.png'];

const args = parseArgs(stripOptionalLeadingSeparator(process.argv.slice(2)));
const left = parseTriple(args.compare, '--compare');
const right = parseTriple(args.to, '--to');
validateComparison(left, right, args.criterionArgs);

const compareId = [sanitizeTriple(left), sanitizeTriple(right), args.scaleGroup]
	.filter(Boolean)
	.join('__');

console.log(`Comparing ${formatTriple(left)} -> ${formatTriple(right)}`);
console.log(`Criterion baseline/id: ${compareId}\n`);

runBenchmark({
	label: 'baseline',
	triple: left,
	criterionMode: ['--save-baseline', compareId],
	criterionArgs: args.criterionArgs,
	scaleGroup: args.scaleGroup,
	compareId
});

runBenchmark({
	label: 'comparison',
	triple: right,
	criterionMode: ['--baseline', compareId],
	criterionArgs: args.criterionArgs,
	scaleGroup: args.scaleGroup,
	compareId
});

console.log(`\nCompared ${formatTriple(left)} -> ${formatTriple(right)} with baseline ${compareId}.`);

function runBenchmark({ label, triple, criterionMode, criterionArgs, scaleGroup, compareId }) {
	const cargoArgs = [
		'bench',
		'--manifest-path',
		'crates/ditherette-wasm/Cargo.toml',
		'--bench',
		'crit_resize_filters'
	];

	const features = cargoFeaturesFor(triple);
	if (features.length > 0) {
		cargoArgs.push('--features', features.join(','));
	}

	cargoArgs.push('--', '--noplot', ...criterionMode, ...criterionArgsWithDefaults(criterionArgs));

	console.log(`\n[${label}] ${formatTriple(triple)}`);
	console.log(`cargo ${cargoArgs.join(' ')}`);

	const result = spawnSync('cargo', cargoArgs, {
		cwd: root,
		stdio: 'inherit',
		env: {
			...process.env,
			RESIZE_FILTER: triple.filter,
			RESIZE_FILTER_IMPLEMENTATION: triple.implementation,
			RESIZE_FILTER_COMPARE_ID: compareId,
			...(scaleGroup ? { RESIZE_SCALE_GROUP: scaleGroup } : {}),
			...(!args.allFixtures ? { RESIZE_FIXTURES: defaultFixtureNames.join(',') } : {})
		}
	});

	if (result.status !== 0) {
		process.exit(result.status ?? 1);
	}
}

function stripOptionalLeadingSeparator(argv) {
	return argv[0] === '--' ? argv.slice(1) : argv;
}

function parseArgs(argv) {
	const separatorIndex = argv.indexOf('--');
	const ownArgs = separatorIndex === -1 ? argv : argv.slice(0, separatorIndex);
	const criterionArgs = separatorIndex === -1 ? [] : argv.slice(separatorIndex + 1);
	let compare;
	let to;
	let scaleGroup;
	let allFixtures = false;

	for (let index = 0; index < ownArgs.length; index += 1) {
		const arg = ownArgs[index];

		if (arg === '--compare') {
			compare = ownArgs[index + 1];
			index += 1;
			continue;
		}

		if (arg.startsWith('--compare=')) {
			compare = arg.slice('--compare='.length);
			continue;
		}

		if (arg === '--to') {
			to = ownArgs[index + 1];
			index += 1;
			continue;
		}

		if (arg.startsWith('--to=')) {
			to = arg.slice('--to='.length);
			continue;
		}

		if (arg === '--all-fixtures') {
			allFixtures = true;
			continue;
		}

		if (arg === '--scale-group') {
			scaleGroup = ownArgs[index + 1];
			index += 1;
			continue;
		}

		if (arg.startsWith('--scale-group=')) {
			scaleGroup = arg.slice('--scale-group='.length);
			continue;
		}

		if (arg === '--help' || arg === '-h') {
			console.log(helpText());
			process.exit(0);
		}

		throw new Error(`Unknown argument: ${arg}`);
	}

	if (!compare || !to) {
		throw new Error('Expected both --compare <category:filter:implementation> and --to <category:filter:implementation>.');
	}

	if (scaleGroup && !['upscale', 'fractional-downscale', 'exact-downscale'].includes(scaleGroup)) {
		throw new Error('Expected --scale-group to be one of: upscale, fractional-downscale, exact-downscale.');
	}

	return { compare, to, scaleGroup, allFixtures, criterionArgs };
}

function parseTriple(value, flagName) {
	const parts = value.split(':');
	if (parts.length !== 3 || parts.some((part) => part.length === 0)) {
		throw new Error(`${flagName} must use <category>:<filter>:<implementation>, got "${value}".`);
	}

	return {
		category: parts[0],
		filter: parts[1],
		implementation: parts[2]
	};
}

function validateComparison(left, right, criterionArgs) {
	for (const triple of [left, right]) {
		if (triple.category !== 'resize') {
			throw new Error(`Unsupported category "${triple.category}". Only "resize" is available.`);
		}

		if (!resizeFilters.has(triple.filter)) {
			throw new Error(`Unknown resize filter "${triple.filter}". Expected one of: ${[...resizeFilters].join(', ')}.`);
		}

		if (!implementations.has(triple.implementation)) {
			throw new Error(`Unknown implementation "${triple.implementation}". Expected one of: ${[...implementations].join(', ')}.`);
		}

	}

	if (left.category !== right.category) {
		throw new Error('Comparisons across categories are not supported yet.');
	}

	for (const criterionArg of criterionArgs) {
		if (criterionArg === '--baseline' || criterionArg.startsWith('--baseline=')) {
			throw new Error('Do not pass --baseline to crit:cmp; it is generated from --compare/--to.');
		}
		if (criterionArg === '--save-baseline' || criterionArg.startsWith('--save-baseline=')) {
			throw new Error('Do not pass --save-baseline to crit:cmp; it is generated from --compare/--to.');
		}
	}
}

function criterionArgsWithDefaults(criterionArgs) {
	const args = [...criterionArgs];
	if (!hasCriterionOption(args, '--warm-up-time')) {
		args.push('--warm-up-time', '1');
	}
	if (!hasCriterionOption(args, '--measurement-time')) {
		args.push('--measurement-time', '5');
	}
	if (!hasCriterionOption(args, '--sample-size')) {
		args.push('--sample-size', '50');
	}
	return args;
}

function hasCriterionOption(args, option) {
	return args.some((arg, index) => arg === option || arg.startsWith(`${option}=`) || args[index - 1] === option);
}

function cargoFeaturesFor(triple) {
	return triple.implementation === 'tiling' ? ['tiling'] : [];
}

function sanitizeTriple(triple) {
	return formatTriple(triple).replaceAll(':', '_').replaceAll(/[^A-Za-z0-9_\-]/g, '_');
}

function formatTriple(triple) {
	return `${triple.category}:${triple.filter}:${triple.implementation}`;
}

function helpText() {
	return `Usage: pnpm crit:cmp --compare resize:area:reference --to resize:area:scalar [-- criterion options]

Runs two Criterion passes with the same synthetic benchmark ID:
  1. --compare implementation with --save-baseline <id>
  2. --to implementation with --baseline <id>

Defaults to --warm-up-time 1 --measurement-time 5 --sample-size 50 unless overridden after --.
By default, only Celeste_Insta_selfie.png and Celeste_box_art.png run; pass --all-fixtures to use every supported image in benchmark-fixtures.

Supported resize implementations: reference, scalar, scalar_2, tiling.
Examples:
  pnpm crit:cmp --compare resize:area:reference --to resize:area:scalar
  pnpm crit:cmp --compare resize:area:scalar --to resize:area:tiling
  pnpm crit:cmp --compare resize:bicubic:scalar --to resize:area:scalar --scale-group fractional-downscale
  pnpm crit:cmp --compare resize:bilinear:scalar --to resize:bicubic:scalar -- --measurement-time 30
  pnpm crit:cmp --compare resize:bicubic:scalar --to resize:bicubic:scalar_2 --all-fixtures`;
}
