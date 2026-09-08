//! Check fresh-build evidence before turning mutable build outputs into snapshots.

use super::{collect, invalid, relative_path, BundleSource};
use crate::verification::content_digest;
use ditherette_bench_api::verification::Digest256;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path, process::Command};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildFile {
    pub path: String,
    pub bytes: u64,
    pub digest: Digest256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildTool {
    pub name: String,
    pub version: String,
    pub digest: Digest256,
}

/// Explicit developer feature selection, independent of the scalar or threaded variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildMode {
    Public,
    BenchSubjects,
}

/// Paths are relative to explicit source roots, never part of content identity.
/// An absent build mode preserves compatibility with older schema-one records.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildProvenance {
    pub schema: u32,
    pub source_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_mode: Option<BuildMode>,
    pub tools: Vec<BuildTool>,
    pub inputs: Vec<BuildFile>,
    pub package: Vec<BuildFile>,
    pub typescript: Vec<BuildFile>,
    pub scripts: Vec<BuildFile>,
}

pub fn validate_source_revision(source: &BundleSource, revision: &str) -> io::Result<()> {
    let provenance = read(source)?;
    if provenance.source_revision != revision {
        return Err(invalid(
            "asset source revision differs from its role's worker revision",
        ));
    }
    validate(source)
}

fn read(source: &BundleSource) -> io::Result<BuildProvenance> {
    serde_json::from_slice(&fs::read(&source.provenance)?).map_err(io::Error::other)
}

pub(super) fn validate(source: &BundleSource) -> io::Result<()> {
    let provenance = read(source)?;
    if provenance.schema != 1
        || provenance.tools.is_empty()
        || provenance
            .tools
            .iter()
            .any(|tool| tool.name.is_empty() || tool.version.is_empty() || tool.digest.0 == [0; 32])
    {
        return Err(invalid(
            "build provenance schema or toolchain identity is incomplete",
        ));
    }
    let head = git(&source.source_checkout, &["rev-parse", "HEAD"])?;
    if String::from_utf8_lossy(&head).trim() != provenance.source_revision
        || !git(
            &source.source_checkout,
            &["status", "--porcelain", "--untracked-files=normal"],
        )?
        .is_empty()
    {
        return Err(invalid(
            "asset source checkout is dirty or differs from build revision",
        ));
    }
    let mut inputs = Vec::new();
    for entry in git(&source.source_checkout, &["ls-tree", "-r", "-z", "HEAD"])?
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        let entry = std::str::from_utf8(entry).map_err(io::Error::other)?;
        let (header, path) = entry
            .split_once('\t')
            .ok_or_else(|| invalid("invalid committed source tree entry"))?;
        let fields: Vec<_> = header.split_whitespace().collect();
        if fields.len() != 3 || !["100644", "100755"].contains(&fields[0]) || fields[1] != "blob" {
            return Err(invalid("committed build inputs must be regular files"));
        }
        let absolute = source.source_checkout.join(relative_path(path)?);
        if !fs::symlink_metadata(&absolute)?.is_file() {
            return Err(invalid("tracked build inputs must be regular files"));
        }
        let bytes = fs::read(absolute)?;
        if bytes != git(&source.source_checkout, &["cat-file", "blob", fields[2]])? {
            return Err(invalid(
                "working build input differs from its committed HEAD blob",
            ));
        }
        inputs.push(BuildFile {
            path: path.into(),
            bytes: bytes.len() as u64,
            digest: content_digest(&bytes),
        });
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    if inputs.is_empty() || inputs != provenance.inputs {
        return Err(invalid(
            "build source inputs differ from the complete tracked checkout",
        ));
    }
    for (root, expected) in [
        (&source.package, &provenance.package),
        (&source.typescript, &provenance.typescript),
        (&source.scripts, &provenance.scripts),
    ] {
        let root = fs::canonicalize(root)?;
        let mut actual = Vec::new();
        collect(&root, Path::new(""), "", true, &mut actual)?;
        let mut actual: Vec<_> = actual
            .into_iter()
            .map(|(file, _)| BuildFile {
                path: file.path,
                bytes: file.bytes,
                digest: file.digest,
            })
            .collect();
        actual.sort_by(|a, b| a.path.cmp(&b.path));
        if actual.is_empty() || actual != *expected {
            return Err(invalid(
                "built dependency closure differs from retained build provenance",
            ));
        }
    }
    Ok(())
}

fn git(directory: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(invalid("cannot inspect build source checkout"));
    }
    Ok(output.stdout)
}
