use ditherette_bench::{browser_assets::*, paired::browser::*, verification::content_digest};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ditherette-browser-assets-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn file(&self, path: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }
    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.0.join("checkout"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }
    fn sources(&self) -> BrowserSources {
        self.file("checkout/source.rs", b"source");
        self.git(&["init", "-q"]);
        self.git(&["add", "source.rs"]);
        self.git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ]);
        for (path, bytes) in [
            (
                "package/package.json",
                br#"{"name":"ditherette","exports":{".":{"import":"./dist/index.js"}}}"#.as_slice(),
            ),
            ("package/dist/index.js", b"import './helper.js';"),
            ("package/dist/helper.js", b"export const helper = 1;"),
            ("package/dist/core.wasm", b"wasm"),
            ("typescript/main.js", b"export const resize = 1;"),
            ("scripts/transport.mjs", b"transport"),
            ("scripts/page.mjs", b"page"),
            (
                "playwright/package.json",
                br#"{"name":"playwright","version":"1.0"}"#,
            ),
            ("playwright/index.mjs", b"playwright"),
            (
                "core/package.json",
                br#"{"name":"playwright-core","version":"1.0"}"#,
            ),
            ("browser/launcher", b"binary"),
            ("browser/private.so", b"private dependency"),
            ("node", b"node binary"),
        ] {
            self.file(path, bytes);
        }
        let provenance = BuildProvenance {
            schema: 1,
            source_revision: self.git(&["rev-parse", "HEAD"]),
            tools: vec![BuildTool {
                name: "fixture compiler".into(),
                version: "1".into(),
                digest: content_digest(b"compiler"),
            }],
            inputs: vec![BuildFile {
                path: "source.rs".into(),
                bytes: 6,
                digest: content_digest(b"source"),
            }],
            package: manifest(&self.0.join("package")),
            typescript: manifest(&self.0.join("typescript")),
            scripts: manifest(&self.0.join("scripts")),
        };
        let provenance = self.file(
            "build-provenance.json",
            &serde_json::to_vec(&provenance).unwrap(),
        );
        let accepted = BundleSource {
            source_checkout: self.0.join("checkout"),
            provenance,
            package: self.0.join("package"),
            typescript: self.0.join("typescript"),
            scripts: self.0.join("scripts"),
            entries: AssetEntries {
                package: "package/dist/index.js".into(),
                typescript: "typescript/main.js".into(),
                transport: "scripts/transport.mjs".into(),
                page: "scripts/page.mjs".into(),
                wasm: "package/dist/core.wasm".into(),
            },
        };
        BrowserSources {
            candidate: accepted.clone(),
            accepted,
            runtime: RuntimeSource {
                engine: BrowserEngine::Webkit,
                node: self.0.join("node"),
                node_version: "v24".into(),
                browser: self.0.join("browser/launcher"),
                browser_assets: self.0.join("browser"),
                browser_version: "1".into(),
                playwright: self.0.join("playwright"),
                playwright_core: self.0.join("core"),
                launch_args: vec![],
                headless: true,
                cross_origin_isolated: true,
            },
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn manifest(root: &Path) -> Vec<BuildFile> {
    fn visit(root: &Path, path: &Path, files: &mut Vec<BuildFile>) {
        if path.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                visit(root, &entry.unwrap().path(), files);
            }
            return;
        }
        let bytes = fs::read(path).unwrap();
        files.push(BuildFile {
            path: path.strip_prefix(root).unwrap().to_str().unwrap().into(),
            bytes: bytes.len() as u64,
            digest: content_digest(&bytes),
        });
    }
    let mut files = vec![];
    visit(root, root, &mut files);
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}
#[cfg(unix)]
fn mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn complete_closures_are_portable_and_include_helpers_native_libraries_and_provenance() {
    let f = Fixture::new();
    let source = f.sources();
    let first = prepare_assets(&source, &f.0.join("first")).unwrap();
    let second = prepare_assets(&source, &f.0.join("second")).unwrap();
    assert_eq!(first.accepted.tree.digest, second.accepted.tree.digest);
    assert_eq!(
        runtime_digest(&first.runtime).unwrap(),
        runtime_digest(&second.runtime).unwrap()
    );
    for path in [
        "package/dist/helper.js",
        "package/dist/core.wasm",
        "provenance/build-provenance.json",
    ] {
        assert!(first
            .accepted
            .tree
            .files
            .iter()
            .any(|file| file.path == path));
    }
    assert!(first
        .runtime
        .browser_assets
        .files
        .iter()
        .any(|file| file.path.ends_with("private.so")));
    assert!(first
        .runtime
        .browser
        .path
        .starts_with(&first.runtime.browser_assets.root));
    validate_trial_assets(&first.trial(ditherette_bench::paired::Role::Accepted)).unwrap();
    assert!(prepare_assets(&source, &f.0.join("first")).is_err());
    assert_eq!(
        fs::read(source.accepted.package.join("dist/helper.js")).unwrap(),
        b"export const helper = 1;"
    );
}

#[cfg(unix)]
#[test]
fn tampering_additions_removals_permissions_and_runtime_changes_are_rejected() {
    let f = Fixture::new();
    let source = f.sources();
    let prepared = prepare_assets(&source, &f.0.join("snapshot")).unwrap();
    let tree = &prepared.accepted.tree;
    let helper = tree.root.join("package/dist/helper.js");
    let bytes = fs::read(&helper).unwrap();
    mode(&helper, 0o644);
    assert!(validate_tree(tree).is_err());
    fs::write(&helper, b"changed").unwrap();
    mode(&helper, 0o444);
    assert!(validate_tree(tree).is_err());
    mode(&helper, 0o644);
    fs::write(&helper, bytes).unwrap();
    mode(&helper, 0o444);
    validate_tree(tree).unwrap();
    let extra = tree.root.join("extra.js");
    fs::write(&extra, b"extra").unwrap();
    mode(&extra, 0o444);
    assert!(validate_tree(tree).is_err());
    fs::remove_file(extra).unwrap();
    fs::remove_file(&helper).unwrap();
    assert!(validate_tree(tree).is_err());
    fs::write(&source.runtime.node, b"other node").unwrap();
    assert!(
        validate_trial_assets(&prepared.trial(ditherette_bench::paired::Role::Candidate)).is_err()
    );
}

#[cfg(unix)]
#[test]
fn links_are_materialized_only_within_source_and_rejected_in_snapshots() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    f.file("source/a", b"a");
    symlink("a", f.0.join("source/b")).unwrap();
    let tree = snapshot_tree(&[("source", &f.0.join("source"))], &f.0.join("snapshot")).unwrap();
    assert!(fs::symlink_metadata(tree.root.join("source/b"))
        .unwrap()
        .is_file());
    f.file("outside", b"outside");
    symlink("../outside", f.0.join("source/escape")).unwrap();
    assert!(snapshot_tree(&[("source", &f.0.join("source"))], &f.0.join("rejected")).is_err());
    fs::remove_file(tree.root.join("source/b")).unwrap();
    symlink("a", tree.root.join("source/b")).unwrap();
    assert!(validate_tree(&tree).is_err());
    assert!(snapshot_tree(
        &[("../escape", &f.0.join("source"))],
        &f.0.join("traversal")
    )
    .is_err());
}

#[test]
fn provenance_rejects_wrong_revision_dirty_source_and_stale_built_bytes() {
    let f = Fixture::new();
    let source = f.sources();
    let revision = f.git(&["rev-parse", "HEAD"]);
    validate_source_revision(&source.accepted, &revision).unwrap();
    assert!(validate_source_revision(&source.accepted, &"a".repeat(40)).is_err());
    f.file("checkout/source.rs", b"changed");
    assert!(prepare_assets(&source, &f.0.join("dirty")).is_err());
    f.file("checkout/source.rs", b"source");
    f.file("package/dist/helper.js", b"stale");
    assert!(prepare_assets(&source, &f.0.join("stale")).is_err());
}

#[test]
fn index_flags_cannot_hide_source_bytes_under_an_unchanged_revision() {
    for flag in ["--assume-unchanged", "--skip-worktree"] {
        let f = Fixture::new();
        let source = f.sources();
        f.git(&["update-index", flag, "source.rs"]);
        f.file("checkout/source.rs", b"tampered");
        assert!(f.git(&["status", "--porcelain"]).is_empty());
        let mut provenance: BuildProvenance =
            serde_json::from_slice(&fs::read(&source.accepted.provenance).unwrap()).unwrap();
        provenance.inputs[0].bytes = 8;
        provenance.inputs[0].digest = content_digest(b"tampered");
        fs::write(
            &source.accepted.provenance,
            serde_json::to_vec(&provenance).unwrap(),
        )
        .unwrap();
        let error =
            validate_source_revision(&source.accepted, &provenance.source_revision).unwrap_err();
        assert!(error.to_string().contains("committed HEAD blob"));
    }
}

#[test]
fn webkit_launcher_uses_only_its_copied_install_and_private_libraries() {
    let f = Fixture::new();
    f.file("install/minibrowser-wpe/bin/MiniBrowser", b"browser");
    f.file("libs/private.so", b"lib");
    let launcher = prepare_webkit(
        &f.0.join("install"),
        &f.0.join("libs"),
        &f.0.join("relocatable"),
    )
    .unwrap();
    let text = fs::read_to_string(launcher).unwrap();
    assert!(text.contains("$root/engine/minibrowser-wpe/bin/MiniBrowser"));
    assert!(text.contains("$root/libraries:"));
    assert!(!text.contains(f.0.to_str().unwrap()));
    let copied = snapshot_tree(
        &[("browser", &f.0.join("relocatable"))],
        &f.0.join("copied"),
    )
    .unwrap();
    validate_tree(&copied).unwrap();
}

#[test]
fn copying_an_absolute_external_launcher_does_not_bind_its_dependencies() {
    let f = Fixture::new();
    let source = f.sources();
    fs::write(
        &source.runtime.browser,
        b"#!/bin/sh\nexec /external/unbound/MiniBrowser \"$@\"\n",
    )
    .unwrap();
    let error = prepare_assets(&source, &f.0.join("rejected-launcher")).unwrap_err();
    assert!(error.to_string().contains("reviewed relocatable"));
}
