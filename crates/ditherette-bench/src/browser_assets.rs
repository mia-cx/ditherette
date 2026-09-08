//! Immutable browser/package dependency trees and checked runtime identities.
//!
//! Preparation reads built artifacts only. It never compiles or starts a browser.

use crate::{
    paired::browser::{
        tree_digest, AssetBundle, AssetEntries, AssetFile, AssetTree, BrowserEngine,
        BrowserRuntime, BrowserTrial, PreparedBrowser, RuntimeBinary, RuntimePackage,
    },
    verification::content_digest,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Component, Path, PathBuf},
};

mod oracle;
mod provenance;
pub use oracle::validate_oracle;
pub use provenance::{validate_source_revision, BuildFile, BuildMode, BuildProvenance, BuildTool};
const WEBKIT_LAUNCHER: &[u8] = include_bytes!("browser_assets/webkit-launcher.sh");

/// Explicit built inputs for one role. Entries are relative to the combined snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleSource {
    pub source_checkout: PathBuf,
    pub provenance: PathBuf,
    pub package: PathBuf,
    pub typescript: PathBuf,
    pub scripts: PathBuf,
    pub entries: AssetEntries,
}

/// Runtime binaries and dependency packages supplied during preparation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSource {
    pub engine: BrowserEngine,
    pub node: PathBuf,
    pub node_version: String,
    pub browser: PathBuf,
    pub browser_assets: PathBuf,
    pub browser_version: String,
    pub playwright: PathBuf,
    pub playwright_core: PathBuf,
    pub launch_args: Vec<String>,
    pub headless: bool,
    pub cross_origin_isolated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserSources {
    pub accepted: BundleSource,
    pub candidate: BundleSource,
    pub runtime: RuntimeSource,
}

/// Stage a relocatable task-local WPE WebKit launcher and its private dependencies.
/// `private_libraries` is the directory containing the additional shared libraries.
pub fn prepare_webkit(
    install: &Path,
    private_libraries: &Path,
    directory: &Path,
) -> io::Result<PathBuf> {
    let tree = snapshot_tree(
        &[("engine", install), ("libraries", private_libraries)],
        directory,
    )?;
    require_entry(&tree, "engine/minibrowser-wpe/bin/MiniBrowser")?;
    let launcher = tree.root.join("webkit");
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&launcher)?
        .write_all(WEBKIT_LAUNCHER)?;
    set_mode(&launcher, 0o555)?;
    Ok(launcher)
}

/// Build a new retained asset directory. Existing destinations are never overwritten.
pub fn prepare_assets(sources: &BrowserSources, directory: &Path) -> io::Result<PreparedBrowser> {
    let node = runtime_binary(&sources.runtime.node, &sources.runtime.node_version)?;
    let mut browser = runtime_binary(&sources.runtime.browser, &sources.runtime.browser_version)?;
    validate_launcher(&browser.path)?;
    let browser_root = fs::canonicalize(&sources.runtime.browser_assets)?;
    let browser_entry = browser
        .path
        .strip_prefix(&browser_root)
        .map_err(|_| {
            invalid("browser launch entry must belong to the complete browser source tree")
        })?
        .to_owned();
    let playwright_version = package_version(&sources.runtime.playwright, "playwright")?;
    let core_version = package_version(&sources.runtime.playwright_core, "playwright-core")?;
    if playwright_version != core_version {
        return Err(invalid("Playwright and playwright-core versions differ"));
    }
    fs::create_dir(directory)?;
    let directory = fs::canonicalize(directory)?;
    let accepted = snapshot_bundle(&sources.accepted, &directory.join("accepted"))?;
    let candidate = snapshot_bundle(&sources.candidate, &directory.join("candidate"))?;
    fs::create_dir(directory.join("runtime"))?;
    let browser_assets = snapshot_tree(
        &[("browser", browser_root.as_path())],
        &directory.join("runtime/browser"),
    )?;
    browser.path = browser_assets.root.join("browser").join(browser_entry);
    let playwright = RuntimePackage {
        tree: snapshot_tree(
            &[
                ("playwright", sources.runtime.playwright.as_path()),
                ("playwright-core", sources.runtime.playwright_core.as_path()),
            ],
            &directory.join("runtime/node_modules"),
        )?,
        version: playwright_version,
        entry: "playwright/index.mjs".into(),
    };
    require_entry(&playwright.tree, &playwright.entry)?;
    let runtime = BrowserRuntime {
        engine: sources.runtime.engine,
        node,
        browser,
        browser_assets,
        playwright,
        launch_args: sources.runtime.launch_args.clone(),
        headless: sources.runtime.headless,
        cross_origin_isolated: sources.runtime.cross_origin_isolated,
    };
    let prepared = PreparedBrowser {
        accepted,
        candidate,
        runtime,
    };
    for assets in [&prepared.accepted, &prepared.candidate] {
        validate_trial_assets(&BrowserTrial {
            assets: assets.clone(),
            runtime: prepared.runtime.clone(),
        })?;
    }
    Ok(prepared)
}

fn snapshot_bundle(source: &BundleSource, directory: &Path) -> io::Result<AssetBundle> {
    provenance::validate(source)?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(source.package.join("package.json"))?)
            .map_err(io::Error::other)?;
    if metadata["name"] != "ditherette" {
        return Err(invalid(
            "package source must be the actual installed ditherette package",
        ));
    }
    let export = metadata
        .pointer("/exports/./import")
        .and_then(|value| value.as_str())
        .or_else(|| {
            metadata
                .pointer("/exports/.")
                .and_then(|value| value.as_str())
        })
        .ok_or_else(|| invalid("installed package has no supported public import export"))?;
    if source.entries.package != format!("package/{}", export.trim_start_matches("./")) {
        return Err(invalid(
            "package entry differs from its installed public export",
        ));
    }
    let tree = snapshot_tree(
        &[
            ("package", source.package.as_path()),
            ("typescript", source.typescript.as_path()),
            ("scripts", source.scripts.as_path()),
            (
                "provenance/build-provenance.json",
                source.provenance.as_path(),
            ),
        ],
        directory,
    )?;
    let bundle = AssetBundle {
        tree,
        entries: source.entries.clone(),
    };
    validate_bundle(&bundle)?;
    // Legacy snapshots remain readable. New preparation requires same-target oracle evidence.
    validate_oracle(&bundle)?;
    Ok(bundle)
}

/// Snapshot complete source trees as regular files, retaining alias groups as hardlinks.
pub fn snapshot_tree(sources: &[(&str, &Path)], destination: &Path) -> io::Result<AssetTree> {
    let mut content = Vec::new();
    for (prefix, source) in sources {
        relative_path(prefix)?;
        let source = fs::canonicalize(source)?;
        collect(&source, Path::new(""), prefix, true, &mut content)?;
    }
    content.sort_by(|a, b| a.0.path.cmp(&b.0.path));
    identify_aliases(&mut content)?;
    if content.is_empty()
        || content
            .windows(2)
            .any(|pair| pair[0].0.path == pair[1].0.path)
    {
        return Err(invalid("asset tree is empty or contains duplicate paths"));
    }
    fs::create_dir(destination)?;
    let root = fs::canonicalize(destination)?;
    for (file, source) in &content {
        let path = root.join(relative_path(&file.path)?);
        fs::create_dir_all(path.parent().ok_or_else(|| invalid("asset lacks parent"))?)?;
        if let Some(alias) = &file.alias_of {
            fs::hard_link(root.join(relative_path(alias)?), &path)?;
        } else {
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            io::copy(&mut fs::File::open(source)?, &mut output)?;
        }
        set_mode(&path, file.mode)?;
    }
    let files: Vec<_> = content.into_iter().map(|(file, _)| file).collect();
    let tree = AssetTree {
        root,
        digest: tree_digest(&files)?,
        files,
    };
    validate_tree(&tree)?;
    Ok(tree)
}

fn collect(
    root: &Path,
    relative: &Path,
    prefix: &str,
    source: bool,
    files: &mut Vec<(AssetFile, PathBuf)>,
) -> io::Result<()> {
    let path = if relative.as_os_str().is_empty() {
        root.to_owned()
    } else {
        root.join(relative)
    };
    let mut metadata = fs::symlink_metadata(&path)?;
    if metadata.is_symlink() && source {
        let target = fs::canonicalize(&path)?;
        if !target.starts_with(root) || !target.is_file() {
            return Err(invalid(
                "source links must resolve to regular files inside the same source tree",
            ));
        }
        metadata = fs::metadata(&target)?;
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            collect(
                root,
                &relative.join(entry?.file_name()),
                prefix,
                source,
                files,
            )?;
        }
        return Ok(());
    }
    if !metadata.is_file() {
        return Err(invalid(
            "asset trees may contain regular files and directories only",
        ));
    }
    let bytes = fs::read(&path)?;
    let relative = relative
        .to_str()
        .ok_or_else(|| invalid("asset paths must be UTF-8"))?;
    let path = if prefix.is_empty() {
        relative.to_owned()
    } else if relative.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}/{relative}")
    };
    relative_path(&path)?;
    files.push((
        AssetFile {
            path,
            bytes: bytes.len() as u64,
            mode: file_mode(&metadata) & !0o222,
            digest: content_digest(&bytes),
            alias_of: None,
        },
        if relative.is_empty() {
            root.to_owned()
        } else {
            root.join(relative)
        },
    ));
    Ok(())
}

/// Verify content, read-only modes, alias groups, and absence of extra files or symlinks.
pub fn validate_tree(tree: &AssetTree) -> io::Result<()> {
    if !fs::symlink_metadata(&tree.root)?.is_dir() {
        return Err(invalid("snapshot root must be a real directory"));
    }
    let mut actual = Vec::new();
    collect(&tree.root, Path::new(""), "", false, &mut actual)?;
    actual.sort_by(|a, b| a.0.path.cmp(&b.0.path));
    identify_aliases(&mut actual)?;
    let files: Vec<_> = actual.into_iter().map(|(file, _)| file).collect();
    for file in &files {
        let path = tree.root.join(relative_path(&file.path)?);
        if file_mode(&fs::symlink_metadata(path)?) & 0o222 != 0 {
            return Err(invalid("snapshot files must remain read-only"));
        }
    }
    if files != tree.files || tree_digest(&files)? != tree.digest {
        return Err(invalid("snapshot manifest/content digest differs"));
    }
    Ok(())
}

/// Preserve source symlink and hardlink identity without retaining snapshot symlinks.
/// Only canonical relative paths enter the manifest; device/inode numbers stay local.
fn identify_aliases(files: &mut [(AssetFile, PathBuf)]) -> io::Result<()> {
    let mut groups = std::collections::BTreeMap::new();
    for (file, path) in files {
        let identity = file_identity(path)?;
        file.alias_of = groups.get(&identity).cloned();
        groups.entry(identity).or_insert_with(|| file.path.clone());
    }
    Ok(())
}

#[cfg(unix)]
fn file_identity(path: &Path) -> io::Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::metadata(path)?;
    Ok((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn file_identity(_: &Path) -> io::Result<(u64, u64)> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "browser snapshots require Unix file identity",
    ))
}

pub fn validate_bundle(bundle: &AssetBundle) -> io::Result<()> {
    validate_tree(&bundle.tree)?;
    for entry in [
        &bundle.entries.package,
        &bundle.entries.typescript,
        &bundle.entries.transport,
        &bundle.entries.page,
        &bundle.entries.wasm,
    ] {
        require_entry(&bundle.tree, entry)?;
    }
    if !bundle.entries.package.starts_with("package/")
        || !bundle.entries.wasm.starts_with("package/")
        || !bundle.entries.typescript.starts_with("typescript/")
        || !bundle.entries.transport.starts_with("scripts/")
        || !bundle.entries.page.starts_with("scripts/")
    {
        return Err(invalid(
            "browser entries do not belong to their declared complete source trees",
        ));
    }
    Ok(())
}

/// Bind the retained build source to the worker revision without needing its old checkout.
pub fn validate_bundle_revision(bundle: &AssetBundle, revision: &str) -> io::Result<()> {
    let entry = "provenance/build-provenance.json";
    require_entry(&bundle.tree, entry)?;
    let provenance: BuildProvenance =
        serde_json::from_slice(&fs::read(bundle.tree.root.join(entry))?)
            .map_err(io::Error::other)?;
    if provenance.schema != 1 || provenance.source_revision != revision {
        return Err(invalid(
            "snapshotted build source revision differs from the worker",
        ));
    }
    Ok(())
}

/// Coordinator and worker both call this before and after each owned browser trial.
pub fn validate_trial_assets(trial: &BrowserTrial) -> io::Result<()> {
    crate::paired::browser::validate_trial(trial)?;
    validate_bundle(&trial.assets)?;
    validate_binary(&trial.runtime.node)?;
    validate_binary(&trial.runtime.browser)?;
    validate_launcher(&trial.runtime.browser.path)?;
    validate_tree(&trial.runtime.browser_assets)?;
    validate_tree(&trial.runtime.playwright.tree)?;
    require_entry(
        &trial.runtime.playwright.tree,
        &trial.runtime.playwright.entry,
    )?;
    Ok(())
}

fn validate_launcher(path: &Path) -> io::Result<()> {
    let bytes = fs::read(path)?;
    if bytes.starts_with(b"#!") && bytes != WEBKIT_LAUNCHER {
        return Err(invalid(
            "script launchers must use the reviewed relocatable WebKit launcher",
        ));
    }
    if bytes == WEBKIT_LAUNCHER
        && !path
            .parent()
            .ok_or_else(|| invalid("launcher lacks parent"))?
            .join("engine/minibrowser-wpe/bin/MiniBrowser")
            .is_file()
    {
        return Err(invalid(
            "relocatable launcher lacks its copied browser executable",
        ));
    }
    Ok(())
}

fn runtime_binary(path: &Path, version: &str) -> io::Result<RuntimeBinary> {
    if version.trim().is_empty() {
        return Err(invalid("runtime version is required"));
    }
    let path = fs::canonicalize(path)?;
    if !fs::symlink_metadata(&path)?.is_file() {
        return Err(invalid("runtime binary must be a regular file"));
    }
    Ok(RuntimeBinary {
        digest: content_digest(&fs::read(&path)?),
        path,
        version: version.into(),
    })
}

fn validate_binary(binary: &RuntimeBinary) -> io::Result<()> {
    if binary.version.trim().is_empty()
        || !fs::symlink_metadata(&binary.path)?.is_file()
        || content_digest(&fs::read(&binary.path)?) != binary.digest
    {
        return Err(invalid("runtime executable identity differs"));
    }
    Ok(())
}

fn package_version(path: &Path, expected_name: &str) -> io::Result<String> {
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(path.join("package.json"))?).map_err(io::Error::other)?;
    if metadata["name"] != expected_name {
        return Err(invalid("runtime package name differs"));
    }
    metadata["version"]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| invalid("runtime package version is missing"))
}

fn require_entry(tree: &AssetTree, entry: &str) -> io::Result<()> {
    relative_path(entry)?;
    if !tree.files.iter().any(|file| file.path == entry) {
        return Err(invalid("entry is absent from complete snapshot"));
    }
    Ok(())
}

fn relative_path(path: &str) -> io::Result<&Path> {
    if path.contains('\\') {
        return Err(invalid("asset path must not contain backslashes"));
    }
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || !path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(invalid(
            "asset path must be a nonempty relative path without traversal",
        ));
    }
    Ok(path)
}

#[cfg(unix)]
fn file_mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o777
}
#[cfg(not(unix))]
fn file_mode(metadata: &fs::Metadata) -> u32 {
    if metadata.permissions().readonly() {
        0o444
    } else {
        0o666
    }
}
#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}
#[cfg(not(unix))]
fn set_mode(path: &Path, _: u32) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions)
}
fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}
