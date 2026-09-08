import assert from 'node:assert/strict';
import {
	cpSync,
	existsSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	rmSync,
	symlinkSync,
	writeFileSync
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { CRATE, git, verifyContent } from './content.mjs';
import {
	dependencySnapshot,
	isolatedCheck,
	run,
	syntaxBinary,
	verifyBuildConfiguration,
	verifyDependencies,
	verifySyntax
} from './build.mjs';
import { POLICY, WORKFLOW, verifyPolicy } from './guard.mjs';

const ROOT = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
const checkpoint = JSON.parse(readFileSync(join(ROOT, POLICY, 'checkpoint.json')));
const dependencies = JSON.parse(readFileSync(join(ROOT, POLICY, 'dependencies.json')));
const binary = syntaxBinary();

function fixture(operation) {
	const root = mkdtempSync(join(tmpdir(), 'ditherette-freeze-test-'));
	try {
		for (const path of [
			CRATE,
			'crates/ditherette-bench',
			'crates/ditherette-bench-api',
			'crates/ditherette-bench-oracle',
			POLICY,
			'.github/workflows'
		]) {
			mkdirSync(dirname(join(root, path)), { recursive: true });
			cpSync(join(ROOT, path), join(root, path), {
				recursive: true,
				filter: (path) => basename(path) !== 'target'
			});
		}
		operation(root);
	} finally {
		rmSync(root, { recursive: true, force: true });
		assert.equal(existsSync(root), false, 'temporary mutations are removed');
		verifyContent(ROOT, checkpoint);
	}
}

function mutation(root, path, changed, check) {
	const full = join(root, path);
	const original = existsSync(full) ? readFileSync(full) : undefined;
	try {
		writeFileSync(full, changed);
		check();
	} finally {
		if (original) writeFileSync(full, original);
		else rmSync(full, { force: true });
	}
}

test('controlled frozen edits, additions, deletion, and symlinks fail, then restore cleanly', () =>
	fixture((root) => {
		for (const path of [`${CRATE}/src/spec/color/common.rs`, `${CRATE}/src/image/view.rs`]) {
			mutation(root, path, `${readFileSync(join(root, path))}\n// controlled mutation\n`, () =>
				assert.throws(() => verifyContent(root, checkpoint), /Frozen content changed/)
			);
			verifyContent(root, checkpoint);
		}
		mutation(root, `${CRATE}/src/spec/unexpected.rs`, '// unexpected\n', () =>
			assert.throws(() => verifyContent(root, checkpoint), /Frozen content changed/)
		);
		const path = `${CRATE}/src/spec/color/common.rs`;
		const original = readFileSync(join(root, path));
		rmSync(join(root, path));
		assert.throws(() => verifyContent(root, checkpoint), /Frozen content changed/);
		symlinkSync('/dev/null', join(root, path));
		assert.throws(() => verifyContent(root, checkpoint), /Symlink/);
		rmSync(join(root, path));
		writeFileSync(join(root, path), original);
		verifyContent(root, checkpoint);
	}));

test('trusted base rejects checkpoint, checker, workflow, and added helper replacement', () =>
	fixture((root) => {
		verifyPolicy(root, ROOT);
		for (const path of [
			`${POLICY}/checkpoint.json`,
			`${POLICY}/content.mjs`,
			WORKFLOW,
			`${POLICY}/replacement.mjs`
		]) {
			mutation(root, path, 'replacement\n', () =>
				assert.throws(() => verifyPolicy(root, ROOT), /trusted freeze policy/)
			);
			verifyPolicy(root, ROOT);
		}
	}));

test('content identity survives unrelated history, but a new parent cannot bless edited bytes', () =>
	fixture((root) => {
		git(root, 'init', '--quiet');
		git(root, 'config', 'user.name', 'Freeze fixture');
		git(root, 'config', 'user.email', 'fixture@example.invalid');
		git(root, 'add', '.');
		git(root, 'commit', '--quiet', '-m', 'unrelated root');
		assert.notEqual(git(root, 'rev-parse', 'HEAD'), checkpoint.revision);
		assert.deepEqual(verifyContent(root, checkpoint), checkpoint.identity);
		mutation(root, `${CRATE}/src/spec/mod.rs`, '// changed parent\n', () => {
			git(root, 'add', '.');
			git(root, 'commit', '--quiet', '-m', 'different parent contents');
			assert.throws(() => verifyContent(root, checkpoint), /Frozen content changed/);
		});
	}));

test('syntax rejects both directions, aliases, shared/adapter bridges, and source injection', () =>
	fixture((root) => {
		const cases = [
			['prod', 'pub use crate::{spec as oracle};'],
			['spec', 'pub use crate::{prod as implementation};'],
			['image', 'pub use crate::spec::pipeline as shared;'],
			['prod', '#[cfg(any())] pub use crate::wasm as bridge;'],
			['prod', '#[path = "../spec/color/common.rs"] mod stolen;'],
			['prod', '#[r#path = "../spec/color/common.rs"] mod stolen;'],
			['prod', 'std::include!("../spec/color/common.rs");'],
			['prod', 'use std::include as load; load!("../spec/color/common.rs");'],
			['prod', 'macro_rules! local { () => { #[path = "../spec/color/common.rs"] mod stolen; } }'],
			['prod', 'macro_rules! local { () => { crate::spec::pipeline::process } }'],
			['prod', '#[macro_export] macro_rules! vec { () => {} }']
		];
		for (const [role, addition] of cases) {
			const path = `${CRATE}/src/${role}/mod.rs`;
			mutation(root, path, `${readFileSync(join(root, path))}\n${addition}\n`, () =>
				assert.throws(() => verifySyntax(root, binary))
			);
		}
		// Ordinary implementation syntax remains available without policy edits.
		const path = `${CRATE}/src/prod/mod.rs`;
		mutation(
			root,
			path,
			`${readFileSync(join(root, path))}\n#[repr(u8)] enum Example { A }\nmacro_rules! local { ($value:expr) => { $value + 1 }; }\n`,
			() => verifySyntax(root, binary)
		);
	}));

test('independent Rust compilation rejects indirect helper imports in both directions', () =>
	fixture((root) => {
		for (const [role, opposite] of [
			['prod', 'spec'],
			['spec', 'prod']
		]) {
			const path = `${CRATE}/src/image/mod.rs`;
			mutation(
				root,
				path,
				`${readFileSync(join(root, path))}\npub use crate::${opposite}::color as bridge;\n`,
				() => assert.throws(() => isolatedCheck(root, role, 'x86_64-unknown-linux-gnu'))
			);
		}
	}));

test('real profile changes and crate-root module redirection cannot hide behind isolation', () =>
	fixture((root) => {
		const manifest = `${CRATE}/Cargo.toml`;
		mutation(
			root,
			manifest,
			`${readFileSync(join(root, manifest))}\noverflow-checks = true\n`,
			() => assert.throws(() => verifySyntax(root, binary))
		);
		const lib = `${CRATE}/src/lib.rs`;
		mutation(root, lib, `${readFileSync(join(root, lib))}\npub use spec as bridge;\n`, () =>
			assert.throws(() => verifySyntax(root, binary))
		);
		for (const name of ['serde_json', 'r#serde_json', 'r#vec']) {
			mutation(root, lib, `${readFileSync(join(root, lib))}\npub use wasm::${name};\n`, () =>
				assert.throws(() => verifySyntax(root, binary))
			);
		}
		const adapter = `${CRATE}/src/wasm.rs`;
		mutation(
			root,
			adapter,
			`${readFileSync(join(root, adapter))}\n#[macro_export] macro_rules! vec { () => {} }\n`,
			() => assert.throws(() => verifySyntax(root, binary))
		);
		mutation(root, `${CRATE}/build.rs`, 'fn main() {}\n', () =>
			assert.throws(() => verifyBuildConfiguration(root), /build script/)
		);
		mkdirSync(join(root, CRATE, 'src/wasm'), { recursive: true });
		mutation(
			root,
			`${CRATE}/src/wasm/nested.rs`,
			'#[macro_export] macro_rules! vec { () => {} }\n',
			() => assert.throws(() => verifySyntax(root, binary))
		);
	}));

test('foreign symbol bridges are rejected in semantic roles, not the Wasm adapter boundary', () =>
	fixture((root) => {
		const prod = `${CRATE}/src/prod/mod.rs`;
		const adapter = `${CRATE}/src/wasm.rs`;
		const bridge =
			'extern "Rust" { fn oracle_bridge() -> f32; }\npub fn bridged() -> f32 { unsafe { oracle_bridge() } }';
		mutation(
			root,
			adapter,
			`${readFileSync(join(root, adapter))}\n#[no_mangle] pub extern "Rust" fn oracle_bridge() -> f32 { crate::spec::color::srgb8_to_linear(0) }\n`,
			() => {
				mutation(root, prod, `${readFileSync(join(root, prod))}\n${bridge}\n`, () => {
					// Type-checking succeeds without resolving the adapter-owned symbol.
					isolatedCheck(root, 'prod', 'x86_64-unknown-linux-gnu');
					assert.throws(
						() => verifySyntax(root, binary),
						(error) => error.stderr.includes('semantic foreign blocks')
					);
				});
			}
		);
		for (const role of ['spec', 'prod', 'image']) {
			for (const addition of [
				bridge,
				'#[no_mangle] pub fn bridge() {}',
				'#[unsafe(export_name = "oracle_bridge")] pub fn bridge() {}',
				'#[link_name = "oracle_bridge"] pub fn bridge() {}',
				'macro_rules! foreign { () => { extern "Rust" { fn oracle_bridge(); } }; }'
			]) {
				const path = `${CRATE}/src/${role}/mod.rs`;
				mutation(root, path, `${readFileSync(join(root, path))}\n${addition}\n`, () =>
					assert.throws(() => verifySyntax(root, binary))
				);
			}
		}
		mutation(
			root,
			adapter,
			`${readFileSync(join(root, adapter))}\n#[wasm_bindgen] extern "C" { #[wasm_bindgen(catch, js_name = now)] fn host_now() -> Result<f64, JsValue>; }\n`,
			() => verifySyntax(root, binary)
		);
	}));

test('root-use procedural attributes cannot inject code omitted from isolated roots', () =>
	fixture((root) => {
		const path = `${CRATE}/src/lib.rs`;
		const original = readFileSync(join(root, path), 'utf8');
		for (const attribute of [
			'#[new_proc_macro::inject]',
			'#[cfg(any())]',
			'#[doc = include_str!("arbitrary")]'
		]) {
			mutation(
				root,
				path,
				original.replace('pub use wasm::{', `${attribute}\npub use wasm::{`),
				() =>
					assert.throws(
						() => verifySyntax(root, binary),
						(error) => /root use (attributes|docs)/.test(error.stderr)
					)
			);
		}
		mutation(
			root,
			path,
			original.replace('pub use wasm::{', '#[doc = "Explicit adapter exports."]\npub use wasm::{'),
			() => verifySyntax(root, binary)
		);
	}));

test('adapter procedural expansion, symbol interposition, and Wasm oracle routes require review', () =>
	fixture((root) => {
		const path = `${CRATE}/src/wasm.rs`;
		for (const addition of [
			'#[new_proc_macro::inject] fn injected() {}',
			'#[derive(new_proc_macro::Inject)] struct Injected;',
			'#[no_mangle] pub extern "C" fn powf(_: f32, _: f32) -> f32 { 0.0 }',
			'#[unsafe(export_name = "powf")] pub extern "C" fn changed(_: f32, _: f32) -> f32 { 0.0 }',
			'#[unsafe(r#no_mangle)] pub extern "C" fn powf(_: f32, _: f32) -> f32 { 0.0 }',
			'#[r#export_name = "powf"] pub extern "C" fn changed(_: f32, _: f32) -> f32 { 0.0 }',
			'#[r#link_name = "powf"] pub fn changed() {}',
			'use crate::spec as oracle;',
			'use crate::bench_subjects as bridge;'
		]) {
			mutation(root, path, `${readFileSync(join(root, path))}\n${addition}\n`, () =>
				assert.throws(() => verifySyntax(root, binary))
			);
		}
		mkdirSync(join(root, CRATE, 'src/wasm'), { recursive: true });
		mutation(root, `${CRATE}/src/wasm/nested.rs`, 'use crate::spec as oracle;\n', () =>
			assert.throws(() => verifySyntax(root, binary))
		);
	}));

test('a new procedural macro dependency cannot hide behind an audited attribute alias', () =>
	fixture((root) => {
		const crate = join(root, 'crates/freeze-inject');
		mkdirSync(join(crate, 'src'), { recursive: true });
		writeFileSync(
			join(crate, 'Cargo.toml'),
			'[package]\nname = "freeze-inject"\nversion = "0.1.0"\nedition = "2021"\n[lib]\nproc-macro = true\n'
		);
		writeFileSync(
			join(crate, 'src/lib.rs'),
			'extern crate proc_macro;\n#[proc_macro_attribute] pub fn inject(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream { item }\n'
		);
		const path = `${CRATE}/Cargo.toml`;
		const changed = readFileSync(join(root, path), 'utf8').replace(
			'[dependencies]',
			'[dependencies]\nfreeze-inject = { path = "../freeze-inject" }'
		);
		mutation(root, path, changed, () => {
			// Clean runners may need dependencies outside the guard's platform-filtered cache.
			for (const manifest of [path, 'crates/ditherette-bench/Cargo.toml'])
				run(
					'cargo',
					[
						'+1.97.0',
						'metadata',
						'--all-features',
						'--format-version',
						'1',
						'--manifest-path',
						join(root, manifest)
					],
					tmpdir()
				);
			assert.throws(
				() => verifyDependencies(root, dependencies),
				/Unpinned reference dependency.*freeze-inject/
			);
		});
	}));

test('resolved JSON feature changes fail even with unchanged frozen source', () =>
	fixture((root) => {
		const path = `${CRATE}/Cargo.toml`;
		const changed = readFileSync(join(root, path), 'utf8').replace(
			'serde_json = "1"',
			'serde_json = { version = "1", features = ["preserve_order"] }'
		);
		mutation(root, path, changed, () => {
			// A valid updated lock demonstrates a feature failure, not a stale-lock failure.
			for (const manifest of [path, 'crates/ditherette-bench/Cargo.toml']) {
				run(
					'cargo',
					[
						'+1.97.0',
						'update',
						'--manifest-path',
						join(root, manifest),
						'-p',
						'serde_json',
						'--precise',
						'1.0.149'
					],
					tmpdir()
				);
			}
			const snapshot = dependencySnapshot(root);
			assert(
				snapshot[`${path}:x86_64-unknown-linux-gnu`]
					.find((p) => p.name === 'serde_json')
					.features.includes('preserve_order')
			);
			assert.throws(
				() => verifyDependencies(root, dependencies),
				/Reference dependency\/features changed/
			);
		});
	}));
