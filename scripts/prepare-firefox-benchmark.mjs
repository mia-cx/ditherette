import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';

const runtimeEntry = 'chrome/juggler/content/content/Runtime.js';
const allowWasm = 'this._debugger.allowUnobservedWasm = true;';

function debuggerLayout(source) {
	const sourceFile = ts.createSourceFile('Runtime.js', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
	if (sourceFile.parseDiagnostics.length) return null;
	const isDebugger = (node) => ts.isPropertyAccessExpression(node) && node.name.text === '_debugger';
	const isThisDebugger = (node) => isDebugger(node) && node.expression.kind === ts.SyntaxKind.ThisKeyword;
	const isAssignment = (node) => ts.isExpressionStatement(node) && ts.isBinaryExpression(node.expression) &&
		node.expression.operatorToken.kind === ts.SyntaxKind.EqualsToken;
	const isConstructor = (statement) => {
		if (!isAssignment(statement) || !isThisDebugger(statement.expression.left)) return false;
		const right = statement.expression.right;
		return ts.isNewExpression(right) && ts.isIdentifier(right.expression) && right.expression.text === 'Debugger' &&
			(!right.arguments || right.arguments.length === 0);
	};
	const isAllowAssignment = (statement) => {
		if (!isAssignment(statement)) return false;
		const left = statement.expression.left;
		return ts.isPropertyAccessExpression(left) && left.name.text === 'allowUnobservedWasm' &&
			isThisDebugger(left.expression) && statement.expression.right.kind === ts.SyntaxKind.TrueKeyword;
	};
	const isAllowTarget = (statement) => isAssignment(statement) && ts.isPropertyAccessExpression(statement.expression.left) &&
		statement.expression.left.name.text === 'allowUnobservedWasm' && isThisDebugger(statement.expression.left.expression);
	const isAddDebuggee = (node) => ts.isCallExpression(node) && ts.isPropertyAccessExpression(node.expression) &&
		node.expression.name.text === 'addDebuggee' && isDebugger(node.expression.expression);
	const constructors = [];
	const debuggerAssignments = [];
	const allowAssignments = [];
	let invalidAllowAssignment = false;
	const blocks = [];
	const debuggeeCalls = [];
	function visit(node) {
		if (isAssignment(node)) {
			if (isDebugger(node.expression.left)) {
				debuggerAssignments.push(node);
				if (isConstructor(node)) constructors.push(node);
			}
			if (isAllowTarget(node)) {
				allowAssignments.push(node);
				if (!isAllowAssignment(node)) invalidAllowAssignment = true;
			}
		}
		if (isAddDebuggee(node)) debuggeeCalls.push(node);
		if (ts.isBlock(node)) blocks.push(node);
		node.forEachChild(visit);
	}
	visit(sourceFile);
	if (debuggerAssignments.length !== 1 || constructors.length !== 1 || allowAssignments.length > 1 || invalidAllowAssignment) return null;
	const constructor = constructors[0];
	const allow = allowAssignments[0];
	const matchingBlock = blocks.find((block) => {
		const statements = [...block.statements];
		const constructorIndex = statements.indexOf(constructor);
		if (constructorIndex < 0) return false;
		return allow ? statements[constructorIndex + 1] === allow : true;
	});
	if (!matchingBlock || (allow && debuggeeCalls.some((call) => call.pos < allow.pos)) ||
		(!allow && debuggeeCalls.some((call) => call.pos < constructor.pos))) return null;
	return { constructorEnd: constructor.end, patched: Boolean(allow) };
}

/** Keep Juggler automation while allowing Firefox to optimize the measured Wasm. */
export function enableOptimizedWasm(source) {
	const layout = debuggerLayout(source);
	if (!layout) {
		throw new Error('Unrecognized Juggler runtime; inspect its Debugger setup before benchmarking.');
	}
	if (layout.patched) return source;
	return `${source.slice(0, layout.constructorEnd)}\n    ${allowWasm}${source.slice(layout.constructorEnd)}`;
}

/** Prepare a separate Firefox tree before snapshotting it for a paired trial. Requires unzip/zip. */
export async function prepareFirefoxBenchmark(executable, destination) {
	if (process.env.DITHERETTE_BENCH_QUIET === '1') {
		throw new Error('Prepare the Firefox runtime before the benchmark quiet phase.');
	}
	const original = path.dirname(path.resolve(executable));
	const target = path.resolve(destination);
	if (target === original || target.startsWith(original + path.sep)) {
		throw new Error('Use a separate new directory, outside the installed Firefox tree.');
	}
	await mkdir(path.dirname(target), { recursive: true });
	await mkdir(target);
	await cp(original, target, { recursive: true, verbatimSymlinks: true });
	const archive = path.join(target, 'omni.ja');
	const hash = async () => createHash('sha256').update(await readFile(archive)).digest('hex');
	const before = await hash();
	const source = execFileSync('unzip', ['-p', archive, runtimeEntry], { encoding: 'utf8' });
	const patched = enableOptimizedWasm(source);
	const scratch = path.join(target, '.benchmark-patch');
	if (patched !== source) {
		const filename = path.join(scratch, runtimeEntry);
		await mkdir(path.dirname(filename), { recursive: true });
		await writeFile(filename, patched);
		execFileSync('zip', ['-q', archive, runtimeEntry], { cwd: scratch });
		await rm(scratch, { recursive: true });
	}
	const actual = execFileSync('unzip', ['-p', archive, runtimeEntry], { encoding: 'utf8' });
	if (actual !== patched) throw new Error('Firefox runtime patch verification failed.');
	const policyPath = path.join(target, 'distribution/policies.json');
	let policy = {};
	try { policy = JSON.parse(await readFile(policyPath, 'utf8')); }
	catch (error) { if (error.code !== 'ENOENT') throw error; }
	await mkdir(path.dirname(policyPath), { recursive: true });
	await writeFile(policyPath, JSON.stringify({ ...policy, policies: { ...policy.policies, DisableAppUpdate: true } }));
	// Juggler's test configuration overrides normal policy discovery. Resolve from
	// the executable directory so paired snapshots can relocate the prepared tree.
	const configPath = path.join(target, 'playwright.cfg');
	const config = await readFile(configPath, 'utf8');
	await writeFile(configPath, `${config}\n// Pin the benchmark runtime even after relocation.\n` +
		`var benchmarkPolicy = Components.classes["@mozilla.org/file/directory_service;1"]\n` +
		`  .getService(Components.interfaces.nsIProperties).get("GreD", Components.interfaces.nsIFile);\n` +
		`benchmarkPolicy.append("distribution");\nbenchmarkPolicy.append("policies.json");\n` +
		`lockPref("browser.policies.alternatePath", benchmarkPolicy.path);\n`);
	const result = { executable: path.join(target, path.basename(executable)), source: path.resolve(executable),
		archiveBefore: before, archiveAfter: await hash(), allowUnobservedWasm: true };
	await writeFile(path.join(target, 'benchmark-runtime.json'), JSON.stringify(result, null, 2));
	return result;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const [executable, destination, ...extra] = process.argv.slice(2);
	if (!executable || !destination || extra.length) throw new Error('Usage: prepare-firefox-benchmark.mjs FIREFOX_EXECUTABLE NEW_DIRECTORY');
	console.log(JSON.stringify(await prepareFirefoxBenchmark(executable, destination)));
}
