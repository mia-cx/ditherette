#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

/** Offline compilation of the actual adapter dependency closure, preserving module boundaries. */
export async function prepareTypeScript(destination) {
	await mkdir(destination); // Refuse stale output. The caller chooses a new directory before the lease.
	const pending = ['scripts/benchmark-typescript.ts'];
	const inputs = new Map();
	const options = { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext };
	while (pending.length) {
		const relative = pending.pop();
		if (inputs.has(relative)) continue;
		const source = await readFile(path.join(root, relative), 'utf8');
		inputs.set(relative, digest(source));
		const emitted = ts.transpileModule(source, {
			fileName: relative,
			compilerOptions: options,
			reportDiagnostics: true
		});
		if (emitted.diagnostics?.length)
			throw new Error(
				ts.formatDiagnosticsWithColorAndContext(emitted.diagnostics, {
					getCanonicalFileName: (name) => name,
					getCurrentDirectory: () => root,
					getNewLine: () => '\n'
				})
			);
		const parsed = ts.createSourceFile(
			relative.replace(/\.ts$/, '.js'),
			emitted.outputText,
			ts.ScriptTarget.Latest,
			true,
			ts.ScriptKind.JS
		);
		const rewrites = [];
		for (const statement of parsed.statements) {
			if (!ts.isImportDeclaration(statement) && !ts.isExportDeclaration(statement)) continue;
			const specifier = statement.moduleSpecifier;
			if (!specifier) continue;
			if (!ts.isStringLiteral(specifier) || !specifier.text.startsWith('.'))
				throw new Error(`Unsupported browser dependency in ${relative}`);
			const dependency = path.posix.normalize(
				path.posix.join(path.posix.dirname(relative), specifier.text)
			);
			if (dependency.startsWith('../')) throw new Error('Dependency escapes repository.');
			if (path.posix.extname(dependency))
				throw new Error(`Expected extensionless TypeScript dependency: ${dependency}`);
			pending.push(`${dependency}.ts`);
			rewrites.push({
				start: specifier.getStart(parsed),
				end: specifier.end,
				text: JSON.stringify(`${specifier.text}.js`)
			});
		}
		let output = emitted.outputText;
		for (const rewrite of rewrites.reverse())
			output = output.slice(0, rewrite.start) + rewrite.text + output.slice(rewrite.end);
		const target = path.join(destination, relative.replace(/\.ts$/, '.js'));
		await mkdir(path.dirname(target), { recursive: true });
		await writeFile(target, output);
	}
	const compiler = fileURLToPath(import.meta.resolve('typescript'));
	const manifest = {
		entry: 'scripts/benchmark-typescript.js',
		compiler: { version: ts.version, source: compiler, sha256: digest(await readFile(compiler)) },
		options,
		inputs: [...inputs]
			.sort(([a], [b]) => a.localeCompare(b))
			.map(([path, sha256]) => ({ path, sha256 }))
	};
	await writeFile(
		path.join(destination, 'compiler-inputs.json'),
		`${JSON.stringify(manifest, null, 2)}\n`
	);
	return manifest;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	if (process.argv.length !== 3)
		throw new Error('Usage: prepare-benchmark-typescript.mjs NEW_OUTPUT_DIRECTORY');
	console.log(JSON.stringify(await prepareTypeScript(path.resolve(process.argv[2]))));
}
