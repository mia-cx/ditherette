import { readFile, writeFile } from 'node:fs/promises';
import ts from 'typescript';

/**
 * Wrap generated web bindings in a fresh closure while retaining static imports.
 * Only wasm-bindgen's named functions/classes and local export lists are supported.
 * New export syntax fails generation instead of silently producing shared bindings.
 */
export function scalarFactory(source, sourceName = 'ditherette_wasm.js', threaded = false) {
	const file = ts.createSourceFile(
		sourceName,
		source,
		ts.ScriptTarget.Latest,
		true,
		ts.ScriptKind.JS
	);
	if (file.parseDiagnostics.length) {
		throw new Error(
			`Cannot parse generated scalar glue: ${ts.flattenDiagnosticMessageText(file.parseDiagnostics[0].messageText, '\n')}`
		);
	}
	const imports = [];
	const body = [];
	const exported = new Map();
	const declared = new Set();
	let workerImport;
	const unsupported = () => {
		throw new Error('Unsupported generated scalar export shape; review the factory generator.');
	};
	const addExport = (name, local) => {
		if (exported.has(name) || name === '__proto__') unsupported();
		exported.set(name, local);
	};
	for (const statement of file.statements) {
		if (ts.isImportDeclaration(statement)) {
			if (threaded && statement.moduleSpecifier.text.endsWith('/workerHelpers.no-bundler.js')) {
				const bindings = statement.importClause?.namedBindings;
				if (workerImport || !bindings || !ts.isNamedImports(bindings) ||
					bindings.elements.length !== 1 || statement.importClause.name ||
					(bindings.elements[0].propertyName ?? bindings.elements[0].name).text !== 'startWorkers')
					unsupported();
				workerImport = bindings.elements[0].name.text;
				continue;
			}
			imports.push(statement);
			continue;
		}
		if (ts.isExportDeclaration(statement)) {
			if (
				statement.moduleSpecifier ||
				!statement.exportClause ||
				!ts.isNamedExports(statement.exportClause)
			)
				unsupported();
			for (const item of statement.exportClause.elements) {
				if (
					!ts.isIdentifier(item.name) ||
					(item.propertyName && !ts.isIdentifier(item.propertyName))
				)
					unsupported();
				addExport(item.name.text, (item.propertyName ?? item.name).text);
			}
			continue;
		}
		if (ts.isExportAssignment(statement)) unsupported();
		const modifiers = ts.getModifiers(statement) ?? [];
		const isExported = modifiers.some((modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword);
		if (modifiers.some((modifier) => modifier.kind === ts.SyntaxKind.DefaultKeyword)) unsupported();
		if (ts.isFunctionDeclaration(statement) || ts.isClassDeclaration(statement)) {
			if (!statement.name) unsupported();
			declared.add(statement.name.text);
			if (isExported) {
				addExport(statement.name.text, statement.name.text);
				const localModifiers = modifiers.filter(
					(modifier) => modifier.kind !== ts.SyntaxKind.ExportKeyword
				);
				body.push(
					ts.isFunctionDeclaration(statement)
						? ts.factory.updateFunctionDeclaration(
								statement,
								localModifiers,
								statement.asteriskToken,
								statement.name,
								statement.typeParameters,
								statement.parameters,
								statement.type,
								statement.body
							)
						: ts.factory.updateClassDeclaration(
								statement,
								localModifiers,
								statement.name,
								statement.typeParameters,
								statement.heritageClauses,
								statement.members
							)
				);
				continue;
			}
		} else if (isExported) {
			unsupported();
		}
		body.push(statement);
	}
	if (exported.get('default') !== '__wbg_init' || exported.get('initSync') !== 'initSync') {
		throw new Error(
			'Generated scalar initialization exports changed; expected __wbg_init and initSync.'
		);
	}
	for (const local of exported.values()) {
		if (!declared.has(local)) unsupported();
	}
	if (threaded && !workerImport) throw new Error('Generated threaded worker import changed.');
	const properties = [...exported].map(([name, local]) =>
		ts.factory.createPropertyAssignment(
			ts.factory.createStringLiteral(name),
			ts.factory.createIdentifier(local)
		)
	);
	body.push(
		ts.factory.createReturnStatement(
			ts.factory.createCallExpression(
				ts.factory.createPropertyAccessExpression(ts.factory.createIdentifier('Object'), 'freeze'),
				undefined,
				[ts.factory.createObjectLiteralExpression(properties, true)]
			)
		)
	);
	const factory = ts.factory.createFunctionDeclaration(
		[ts.factory.createModifier(ts.SyntaxKind.ExportKeyword)],
		undefined,
		threaded ? 'createThreadedBindings' : 'createScalarBindings',
		undefined,
		workerImport ? [ts.factory.createParameterDeclaration(undefined, undefined, workerImport)] : [],
		undefined,
		ts.factory.createBlock(body, true)
	);
	const output = ts.factory.updateSourceFile(file, [...imports, factory]);
	const printer = ts.createPrinter({ newLine: ts.NewLineKind.LineFeed, removeComments: true });
	return `/* @ts-self-types="./ditherette_wasm.factory.d.ts" */\n// Generated private factory. The original web glue remains unchanged.\n${printer.printFile(output)}`;
}

/** Generate the factory beside scalar web glue so import.meta URLs and snippet imports stay relative. */
export async function writeScalarFactory(directory, threaded = false) {
	const source = await readFile(new URL('ditherette_wasm.js', directory), 'utf8');
	await writeFile(new URL('ditherette_wasm.factory.js', directory), scalarFactory(source, undefined, threaded));
	await writeFile(
		new URL('ditherette_wasm.factory.d.ts', directory),
		'/** Creates independent glue state; initialization remains explicit. */\n' +
			(threaded
				? 'export declare function createThreadedBindings(startWorkers: (module: WebAssembly.Module, memory: WebAssembly.Memory, builder: import("./ditherette_wasm.js").wbg_rayon_PoolBuilder) => Promise<void>): typeof import("./ditherette_wasm.js");\n'
				: 'export declare function createScalarBindings(): typeof import("./ditherette_wasm.js");\n')
	);
}
