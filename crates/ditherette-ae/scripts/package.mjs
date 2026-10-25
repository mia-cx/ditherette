// Builds the effect for this OS into `dist/`.
//
// macOS: `Ditherette.plugin`, universal (Apple silicon and Intel), ad-hoc signed.
// Windows: `Ditherette.aex`.
//
// Run from anywhere: `node crates/ditherette-ae/scripts/package.mjs`.

import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
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
	// build.rs writes the same PiPL resource for both architectures.
	copyFileSync(
		join(release(targets[0]), `${PACKAGE}.rsrc`),
		join(contents, 'Resources', `${NAME}.rsrc`)
	);
	// An After Effects effect bundle: type 'eFKT', creator 'FXTC'.
	writeFileSync(join(contents, 'PkgInfo'), 'eFKTFXTC');
	writeFileSync(
		join(contents, 'Info.plist'),
		`<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key>
	<string>${NAME}</string>
	<key>CFBundleIdentifier</key>
	<string>${BUNDLE_ID}</string>
	<key>CFBundlePackageType</key>
	<string>eFKT</string>
	<key>CFBundleSignature</key>
	<string>FXTC</string>
</dict>
</plist>
`
	);
	run('lipo', [
		...targets.map((target) => join(release(target), `lib${LIBRARY}.dylib`)),
		'-create',
		'-output',
		join(contents, 'MacOS', NAME)
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
