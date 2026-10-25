// Builds the effect for this OS into `dist/`.
//
// macOS: `Ditherette.plugin`, universal (Apple silicon and Intel), ad-hoc signed.
// Windows: `Ditherette.aex`.
//
// Run from anywhere: `node crates/ditherette-ae/scripts/package.mjs`.

import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const NAME = 'Ditherette';
const PACKAGE = 'ditherette-ae';
const LIBRARY = 'ditherette_ae';
const BUNDLE_ID = 'cx.mia.ditherette';

const crate = fileURLToPath(new URL('..', import.meta.url));
const dist = join(crate, 'dist');
const run = (command, args) => execFileSync(command, args, { cwd: crate, stdio: 'inherit' });
const release = (target = '') => join(crate, 'target', target, 'release');

rmSync(dist, { recursive: true, force: true });
mkdirSync(dist);

if (process.platform === 'darwin') {
	const targets = ['aarch64-apple-darwin', 'x86_64-apple-darwin'];
	for (const target of targets)
		run('cargo', ['build', '--locked', '--release', '--target', target]);
	const contents = join(dist, `${NAME}.plugin`, 'Contents');
	mkdirSync(join(contents, 'Resources'), { recursive: true });
	mkdirSync(join(contents, 'MacOS'));
	// build.rs writes the same resources for both architectures.
	const resources = release(targets[0]);
	copyFileSync(join(resources, `${PACKAGE}.rsrc`), join(contents, 'Resources', `${NAME}.rsrc`));
	copyFileSync(join(resources, `${PACKAGE}_PkgInfo`), join(contents, 'PkgInfo'));
	copyFileSync(join(resources, `${PACKAGE}_Info.plist`), join(contents, 'Info.plist'));
	run('lipo', [
		...targets.map((target) => join(release(target), `lib${LIBRARY}.dylib`)),
		'-create',
		'-output',
		join(contents, 'MacOS', NAME)
	]);
	run('plutil', [
		'-replace',
		'CFBundleIdentifier',
		'-string',
		BUNDLE_ID,
		join(contents, 'Info.plist')
	]);
	// After Effects 25.2 and later load only signed plugins; ad-hoc signing is enough locally.
	run('codesign', ['--force', '--options', 'runtime', '--sign', '-', join(dist, `${NAME}.plugin`)]);
} else if (process.platform === 'win32') {
	run('cargo', ['build', '--locked', '--release']);
	copyFileSync(join(release(), `${LIBRARY}.dll`), join(dist, `${NAME}.aex`));
} else {
	throw new Error('After Effects plugins build on macOS or Windows.');
}

console.log(`Packaged ${NAME} in ${dist}`);
