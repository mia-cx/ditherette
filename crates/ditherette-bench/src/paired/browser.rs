//! Public-call protocol. Artifact identities bind served bytes, not their locations.

use super::*;
use crate::verification::{input_digest, settings_digest};
pub use ditherette_wasm::prod::contract::lifecycle::Threads;
pub use ditherette_wasm::prod::contract::request::{Anchor, Support};
use std::{collections::BTreeSet, io, path::Component};

/// Later method slices extend this operation registry with their concrete typed settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PublicOperation {
    Process {
        settings: super::process::ProcessSettings,
    },
    Diffusion {
        settings: super::diffusion::DiffusionSettings,
    },
    Perturb {
        settings: super::fields::PerturbPolicy,
    },
    Separable {
        settings: super::fields::SeparableSettings,
    },
    Yliluoma {
        settings: super::yliluoma::YliluomaSettings,
    },
    ResizeNearest {
        anchor: Anchor,
    },
    ResizeArea {},
    ResizeTrilinear {
        anchor: Anchor,
    },
    ResizeBilinear {
        anchor: Anchor,
    },
    ResizeBicubic {
        anchor: Anchor,
        support: Support,
    },
    ResizeLanczos2 {
        anchor: Anchor,
        support: Support,
    },
    ResizeLanczos3 {
        anchor: Anchor,
        support: Support,
    },
    Quantize {
        settings: super::quantize::QuantizeSettings,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrowserBackend {
    Package,
    /// Actual resize plus ditherAndQuantize calls from the same package.
    PackageStaged,
    #[serde(rename = "typescript")]
    TypeScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrowserPreparation {
    FreshInstance,
    PrimedInstance,
    /// A fresh instance and declared stage prime precede every single-call sample.
    PrimedSample,
    /// Time createDitherette with already-loaded bytes; package import/fetch is excluded.
    InitializationBytes,
    /// Time createDitherette with an already-compiled module.
    InitializationCompiled,
}

/// Historical fixtures use `none`; cache comparisons declare each artifact's capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheCapability {
    None,
    Roles {
        accepted: PreparationCapability,
        candidate: PreparationCapability,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sample_prime: Option<super::preparation::SamplePrime>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreparationCapability {
    Uncached,
    Preparation,
    ImageStages,
}

impl CacheCapability {
    pub fn sample_prime(self) -> Option<super::preparation::SamplePrime> {
        match self {
            Self::Roles { sample_prime, .. } => sample_prime,
            Self::None => None,
        }
    }
}

/// Development protocol only; functions stay in the actual public-call adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProgressMode {
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressRoles {
    pub accepted: ProgressMode,
    pub candidate: ProgressMode,
}

/// Public initialization policy for each measured role; omitted historical metadata stays scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadRoles {
    pub accepted: Threads,
    pub candidate: Threads,
}

/// Developer-only selector. It never changes the public request or cache identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RowStage {
    Resize,
    Indexed,
    Mixing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowBandParameters {
    /// Zero keeps this stage scalar; positive values choose an absolute output band height.
    pub height: u32,
    pub active_workers: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowPolicyRoles {
    pub stage: RowStage,
    pub accepted: RowBandParameters,
    pub candidate: RowBandParameters,
}

/// Recorded only after the real instance accepts its private policy setter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowPolicyObservation {
    pub stage: RowStage,
    pub parameters: RowBandParameters,
    pub pool_size: u32,
}

/// JavaScript context owning the package and its call timers. Historical records use the page.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrowserExecution {
    #[default]
    Page,
    HostWorker,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserCase {
    /// Explicit developer stability allocation bound. Historical requests omit the override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained_output_limit_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_policy: Option<RowPolicyRoles>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<BrowserExecution>,
    pub operation: PublicOperation,
    pub accepted: BrowserBackend,
    pub candidate: BrowserBackend,
    pub preparation: BrowserPreparation,
    pub cache: CacheCapability,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressRoles>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threads: Option<ThreadRoles>,
    /// Developer diagnostics only. Differences remain incorrect and retain review artifacts.
    #[serde(default)]
    pub measure_nonexact: bool,
}

impl BrowserCase {
    pub fn backend(&self, role: Role) -> BrowserBackend {
        match role {
            Role::Accepted => self.accepted,
            Role::Candidate => self.candidate,
        }
    }
}

impl PublicOperation {
    pub fn subject(&self, backend: BrowserBackend) -> &'static str {
        match (self, backend) {
            (Self::Process { .. }, BrowserBackend::Package) => "public:process:request:package",
            (Self::Process { .. }, BrowserBackend::PackageStaged) => {
                "public:process:request:package-staged"
            }
            (Self::Process { .. }, BrowserBackend::TypeScript) => {
                "public:process:request:typescript"
            }
            (_, BrowserBackend::PackageStaged) => "public:unsupported:request:package-staged",
            (Self::ResizeTrilinear { .. }, BrowserBackend::Package) => {
                "public:resize:trilinear:package"
            }
            (Self::ResizeTrilinear { .. }, BrowserBackend::TypeScript) => {
                "public:resize:trilinear:typescript"
            }
            (Self::Diffusion { .. }, BrowserBackend::Package) => {
                "public:dither-and-quantize:diffusion:package"
            }
            (Self::Diffusion { .. }, BrowserBackend::TypeScript) => {
                "public:dither-and-quantize:diffusion:typescript"
            }
            (Self::Yliluoma { .. }, BrowserBackend::Package) => {
                "public:dither-and-quantize:yliluoma:package"
            }
            (Self::Yliluoma { .. }, BrowserBackend::TypeScript) => {
                "public:dither-and-quantize:yliluoma:typescript"
            }
            (Self::Perturb { .. }, BrowserBackend::Package) => "public:perturb:request:package",
            (Self::Perturb { .. }, BrowserBackend::TypeScript) => {
                "public:perturb:request:typescript"
            }
            (Self::Separable { .. }, BrowserBackend::Package) => {
                "public:dither-and-quantize:request:package"
            }
            (Self::Separable { .. }, BrowserBackend::TypeScript) => {
                "public:dither-and-quantize:request:typescript"
            }
            (Self::Quantize { .. }, BrowserBackend::Package) => "public:quantize:request:package",
            (Self::Quantize { .. }, BrowserBackend::TypeScript) => {
                "public:quantize:request:typescript"
            }
            (Self::ResizeNearest { .. }, BrowserBackend::Package) => {
                "public:resize:nearest:package"
            }
            (Self::ResizeNearest { .. }, BrowserBackend::TypeScript) => {
                "public:resize:nearest:typescript"
            }
            (Self::ResizeArea {}, BrowserBackend::Package) => "public:resize:area:package",
            (Self::ResizeArea {}, BrowserBackend::TypeScript) => "public:resize:area:typescript",
            (Self::ResizeBilinear { .. }, BrowserBackend::Package) => {
                "public:resize:bilinear:package"
            }
            (Self::ResizeBilinear { .. }, BrowserBackend::TypeScript) => {
                "public:resize:bilinear:typescript"
            }
            (Self::ResizeBicubic { .. }, BrowserBackend::Package) => {
                "public:resize:bicubic:package"
            }
            (Self::ResizeBicubic { .. }, BrowserBackend::TypeScript) => {
                "public:resize:bicubic:typescript"
            }
            (Self::ResizeLanczos2 { .. }, BrowserBackend::Package) => {
                "public:resize:lanczos2:package"
            }
            (Self::ResizeLanczos2 { .. }, BrowserBackend::TypeScript) => {
                "public:resize:lanczos2:typescript"
            }
            (Self::ResizeLanczos3 { .. }, BrowserBackend::Package) => {
                "public:resize:lanczos3:package"
            }
            (Self::ResizeLanczos3 { .. }, BrowserBackend::TypeScript) => {
                "public:resize:lanczos3:typescript"
            }
        }
    }

    pub fn reference_subject(&self) -> &'static str {
        match self {
            Self::Process { .. } => "spec:process:request:v1",
            Self::ResizeTrilinear { .. } => "spec:resize:trilinear:mip-area",
            Self::Diffusion { .. } => "spec:dither-and-quantize:request:v1",
            Self::Perturb { .. } => "spec:perturb:request:v1",
            Self::Separable { .. } | Self::Yliluoma { .. } => "spec:dither-and-quantize:request:v1",
            Self::Quantize { .. } => "spec:quantize:request:v1",
            Self::ResizeNearest { .. } => "spec:resize:nearest:scalar",
            Self::ResizeArea {} => "spec:resize:area:scalar",
            Self::ResizeBilinear { .. } => "spec:resize:bilinear:scalar",
            Self::ResizeBicubic {
                support: Support::Fixed,
                ..
            } => "spec:resize:bicubic:catmull-rom",
            Self::ResizeBicubic {
                support: Support::ScaleAware,
                ..
            } => "spec:resize:bicubic:catmull-rom-scale-aware",
            Self::ResizeLanczos2 {
                support: Support::Fixed,
                ..
            } => "spec:resize:lanczos2:fixed",
            Self::ResizeLanczos2 {
                support: Support::ScaleAware,
                ..
            } => "spec:resize:lanczos2:scale-aware",
            Self::ResizeLanczos3 {
                support: Support::Fixed,
                ..
            } => "spec:resize:lanczos3:fixed",
            Self::ResizeLanczos3 {
                support: Support::ScaleAware,
                ..
            } => "spec:resize:lanczos3:scale-aware",
        }
    }

    /// Area has no anchor setting; the registry ignores this placeholder.
    pub fn anchor(&self) -> Anchor {
        match *self {
            Self::ResizeNearest { anchor }
            | Self::ResizeTrilinear { anchor }
            | Self::ResizeBilinear { anchor }
            | Self::ResizeBicubic { anchor, .. }
            | Self::ResizeLanczos2 { anchor, .. }
            | Self::ResizeLanczos3 { anchor, .. } => anchor,
            Self::ResizeArea {}
            | Self::Process { .. }
            | Self::Diffusion { .. }
            | Self::Quantize { .. }
            | Self::Perturb { .. }
            | Self::Separable { .. }
            | Self::Yliluoma { .. } => Anchor::Center,
        }
    }

    /// Build one semantic identity shared by package, TypeScript, and frozen output.
    pub fn identity(
        &self,
        source: Dimensions,
        rgba: &[u8],
        output: Dimensions,
    ) -> io::Result<CaseIdentity> {
        if let Some(request) = self.processing_request(source, rgba)? {
            if output != request.dimensions().map_err(io::Error::other)? {
                return Err(io::Error::other(
                    "processing output dimensions differ from the validated recipe",
                ));
            }
            return Ok(CaseIdentity {
                semantics: request.semantics(),
                input: input_digest(source, rgba),
                settings: settings_digest(&request).map_err(io::Error::other)?,
                output,
            });
        }
        let semantics = SemanticIdentity {
            operation: Operation::Resize,
            recipe: match self {
                Self::ResizeNearest { .. } => "nearest-public-v1",
                Self::ResizeTrilinear { .. } => "trilinear-public-v1",
                Self::ResizeArea {} => "area-public-v1",
                Self::ResizeBilinear { .. } => "bilinear-public-v1",
                Self::ResizeBicubic { .. } => "bicubic-public-v1",
                Self::ResizeLanczos2 { .. } => "lanczos2-public-v1",
                Self::ResizeLanczos3 { .. } => "lanczos3-public-v1",
                Self::Quantize { .. }
                | Self::Process { .. }
                | Self::Perturb { .. }
                | Self::Separable { .. }
                | Self::Diffusion { .. }
                | Self::Yliluoma { .. } => {
                    unreachable!("processing returned above")
                }
            }
            .into(),
            version: 1,
            space: None,
        };
        Ok(CaseIdentity {
            settings: settings_digest(&(self, output)).map_err(io::Error::other)?,
            semantics,
            input: input_digest(source, rgba),
            output,
        })
    }

    /// Typed non-resize reference requests reuse the same frozen registry as native calls.
    pub fn processing_request<'a>(
        &'a self,
        source: Dimensions,
        rgba: &'a [u8],
    ) -> io::Result<Option<ditherette_wasm::bench_subjects::reference::ReferenceRequest<'a>>> {
        match self {
            Self::Process { settings } => settings.reference_request(source, rgba).map(Some),
            Self::Diffusion { settings } => settings.reference_request(source, rgba).map(Some),
            Self::Quantize { settings } => settings.reference_request(source, rgba).map(Some),
            Self::Perturb { settings } => {
                super::fields::perturb_request(*settings, source, rgba).map(Some)
            }
            Self::Separable { settings } => settings.reference_request(source, rgba).map(Some),
            Self::Yliluoma { settings } => settings.reference_request(source, rgba).map(Some),
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetFile {
    pub path: String,
    pub bytes: u64,
    pub mode: u32,
    pub digest: Digest256,
    /// Other names for the same file point to the first sorted path in their group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias_of: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetTree {
    pub root: PathBuf,
    pub files: Vec<AssetFile>,
    pub digest: Digest256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetEntries {
    pub package: String,
    pub typescript: String,
    pub transport: String,
    pub page: String,
    pub wasm: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetBundle {
    pub tree: AssetTree,
    pub entries: AssetEntries,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBinary {
    pub path: PathBuf,
    pub digest: Digest256,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePackage {
    pub tree: AssetTree,
    pub entry: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrowserEngine {
    Chromium,
    Firefox,
    Webkit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserRuntime {
    pub engine: BrowserEngine,
    pub node: RuntimeBinary,
    pub browser: RuntimeBinary,
    /// Browser install closure includes launcher, private libraries, and resources.
    pub browser_assets: AssetTree,
    pub playwright: RuntimePackage,
    pub launch_args: Vec<String>,
    pub headless: bool,
    pub cross_origin_isolated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedBrowser {
    pub accepted: AssetBundle,
    pub candidate: AssetBundle,
    pub runtime: BrowserRuntime,
}

impl PreparedBrowser {
    pub fn trial(&self, role: Role) -> BrowserTrial {
        BrowserTrial {
            assets: match role {
                Role::Accepted => self.accepted.clone(),
                Role::Candidate => self.candidate.clone(),
            },
            runtime: self.runtime.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserTrial {
    pub assets: AssetBundle,
    pub runtime: BrowserRuntime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserObservation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_policy: Option<RowPolicyObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<BrowserExecution>,
    pub engine: BrowserEngine,
    pub browser_version: String,
    pub node_version: String,
    pub playwright_version: String,
    pub user_agent: String,
    pub cross_origin_isolated: bool,
    pub timer_resolution_ns: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserEvidence {
    pub assets: Digest256,
    pub runtime: Digest256,
    pub backend: BrowserBackend,
    pub preparation: BrowserPreparation,
    pub cache: CacheCapability,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressRoles>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threads: Option<ThreadRoles>,
    #[serde(default)]
    pub measure_nonexact: bool,
    pub observation: BrowserObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimingSkipped {
    ReferenceMismatch,
}

/// Node preserves public output even when its untimed reference check prevents timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserTransportResult {
    /// Optional only so retained pre-oracle transport records remain readable.
    /// Newly executed browser trials require an independently identified Wasm reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<OracleOutput>,
    /// Frozen output checked after every declared stage prime; absent from historical trials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prime_reference_output: Option<VerificationOutput>,
    pub role: Role,
    pub pair: usize,
    pub case_name: String,
    pub input: Digest256,
    pub settings: Digest256,
    pub sample_ns: Vec<f64>,
    pub iterations_per_sample: usize,
    pub warmup_iterations: usize,
    pub warmup_elapsed_ns: u128,
    pub output: VerificationOutput,
    /// Instability marker containing the first actual output; `output` holds its first distinct successor.
    /// Neither field claims to hold the final sample. Workers preserve both and always reject publication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unstable_output: Option<VerificationOutput>,
    pub observation: BrowserObservation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing_skipped: Option<TimingSkipped>,
}

/// Sorted relative names, sizes, modes, and every byte digest form the tree identity.
pub fn tree_digest(files: &[AssetFile]) -> io::Result<Digest256> {
    settings_digest(&("ditherette-browser-tree-v1", files)).map_err(io::Error::other)
}

/// Runtime locations may change without changing the runtime content identity.
pub fn runtime_digest(runtime: &BrowserRuntime) -> io::Result<Digest256> {
    settings_digest(&(
        "ditherette-browser-runtime-v1",
        runtime.engine,
        (&runtime.node.digest, &runtime.node.version),
        (&runtime.browser.digest, &runtime.browser.version),
        runtime.browser_assets.digest,
        (
            &runtime.playwright.tree.digest,
            &runtime.playwright.entry,
            &runtime.playwright.version,
        ),
        &runtime.launch_args,
        runtime.headless,
        runtime.cross_origin_isolated,
    ))
    .map_err(io::Error::other)
}

/// Both output and artifact-local reference bind the worker, served closure, and runtime.
pub fn artifact_identity(
    worker: &ArtifactIdentity,
    assets: &AssetBundle,
    runtime: &BrowserRuntime,
) -> io::Result<ArtifactIdentity> {
    Ok(ArtifactIdentity {
        revision: worker.revision.clone(),
        content: settings_digest(&(
            "ditherette-browser-artifact-v1",
            worker,
            assets.tree.digest,
            &assets.entries,
            runtime_digest(runtime)?,
        ))
        .map_err(io::Error::other)?,
    })
}

pub fn validate_case(case: &PairCase) -> io::Result<()> {
    if let Some(native) = &case.native {
        if case.browser.is_some()
            || case.measurement.scope != native.scope()
            || (case.measurement.application_cache != ApplicationCache::NotApplicable
                && !matches!(native, super::native::NativeOperation::Processor { .. }))
        {
            return Err(io::Error::other(
                "native operation requires its declared scope without browser or cache claims",
            ));
        }
        if let super::native::NativeOperation::Processor { cache, settings } = native {
            validate_preparation_cache(*cache, &case.measurement)?;
            if let Some(prime) = cache.sample_prime() {
                settings.prime_request(prime, case.source, &case.rgba)?;
            }
        }
        if case.identity != native.identity(case.source, &case.rgba)?
            || case.reference_subject != native.reference_subject()
        {
            return Err(io::Error::other(
                "native operation settings or reference identity differs",
            ));
        }
        return Ok(());
    }
    let Some(browser) = &case.browser else {
        return if case.measurement.scope == CallScope::NativeKernel
            && case.measurement.application_cache == ApplicationCache::NotApplicable
        {
            Ok(())
        } else {
            Err(io::Error::other(
                "public scopes require an explicit browser case",
            ))
        };
    };
    let m = &case.measurement;
    if let Some(limit) = browser.retained_output_limit_bytes {
        const DEFAULT: u64 = 64 * 1024 * 1024;
        const MAXIMUM: u64 = 384 * 1024 * 1024;
        if !(DEFAULT..=MAXIMUM).contains(&limit)
            || m.scope != CallScope::CompleteCall
            || m.mode != SampleMode::SingleCall
            || !matches!(
                browser.operation,
                PublicOperation::Quantize { .. }
                    | PublicOperation::Process { .. }
                    | PublicOperation::Separable { .. }
                    | PublicOperation::Diffusion { .. }
                    | PublicOperation::Yliluoma { .. }
            )
            || browser.accepted != BrowserBackend::Package
            || browser.candidate != BrowserBackend::Package
        {
            return Err(io::Error::other("retained-output override requires bounded single indexed package calls (64–384 MiB)"));
        }
    }
    if browser.execution == Some(BrowserExecution::HostWorker)
        && (!matches!(m.scope, CallScope::Initialization | CallScope::CompleteCall)
            || !matches!(
                browser.accepted,
                BrowserBackend::Package | BrowserBackend::PackageStaged
            )
            || !matches!(
                browser.candidate,
                BrowserBackend::Package | BrowserBackend::PackageStaged
            ))
    {
        return Err(io::Error::other(
            "host-worker execution requires package initialization or complete calls",
        ));
    }
    if browser.threads.is_some()
        && (!matches!(
            browser.accepted,
            BrowserBackend::Package | BrowserBackend::PackageStaged
        ) || !matches!(
            browser.candidate,
            BrowserBackend::Package | BrowserBackend::PackageStaged
        ))
    {
        return Err(io::Error::other(
            "thread policies require ordinary package calls",
        ));
    }
    if let Some(policy) = browser.row_policy {
        if browser.execution != Some(BrowserExecution::HostWorker)
            || m.scope != CallScope::CompleteCall
            || browser.threads
                != Some(ThreadRoles {
                    accepted: Threads::Required,
                    candidate: Threads::Required,
                })
            || [policy.accepted, policy.candidate]
                .iter()
                .any(|p| p.height > 32_768 || !(1..=8).contains(&p.active_workers))
        {
            return Err(io::Error::other(
                "row policies require bounded complete calls in a required-thread host",
            ));
        }
        let stage_matches = match browser.operation {
            PublicOperation::Process { .. } => true,
            PublicOperation::Quantize { .. }
            | PublicOperation::Perturb { .. }
            | PublicOperation::Separable { .. } => policy.stage == RowStage::Indexed,
            PublicOperation::Yliluoma { .. } => policy.stage == RowStage::Mixing,
            PublicOperation::Diffusion { .. } => false,
            _ => policy.stage == RowStage::Resize,
        };
        if !stage_matches {
            return Err(io::Error::other(
                "row policy stage differs from the measured operation",
            ));
        }
    }
    if browser.progress.is_some()
        && (browser.preparation != BrowserPreparation::FreshInstance
            || m.scope != CallScope::CompleteCall
            || m.mode != SampleMode::SingleCall
            || m.application_cache != ApplicationCache::Cold
            || browser.accepted != BrowserBackend::Package
            || browser.candidate != BrowserBackend::Package)
    {
        return Err(io::Error::other(
            "progress comparisons require cold single ordinary package calls",
        ));
    }
    if [browser.accepted, browser.candidate].contains(&BrowserBackend::PackageStaged)
        && !matches!(browser.operation, PublicOperation::Process { .. })
    {
        return Err(io::Error::other(
            "staged package calls require the Process operation",
        ));
    }
    if browser.cache != CacheCapability::None
        || m.application_cache != ApplicationCache::NotApplicable
    {
        validate_preparation_cache(browser.cache, m)?;
        let preparation = match m.application_cache {
            ApplicationCache::Cold => BrowserPreparation::FreshInstance,
            ApplicationCache::Warm if browser.cache.sample_prime().is_some() => {
                BrowserPreparation::PrimedSample
            }
            ApplicationCache::Warm => BrowserPreparation::PrimedInstance,
            ApplicationCache::NotApplicable => unreachable!("validated cache state"),
        };
        if browser.preparation != preparation
            || browser.accepted != BrowserBackend::Package
            || browser.candidate != BrowserBackend::Package
        {
            return Err(io::Error::other("preparation cache cases require ordinary package calls and matching instance lifecycle"));
        }
        if let Some(prime) = browser.cache.sample_prime() {
            use super::preparation::SamplePrime;
            if !matches!(
                (prime, &browser.operation),
                (SamplePrime::SameCall, _)
                    | (SamplePrime::Resize, PublicOperation::Process { .. })
                    | (SamplePrime::Perturb, PublicOperation::Separable { .. })
                    | (SamplePrime::NoDither, PublicOperation::Quantize { .. })
            ) {
                return Err(io::Error::other(
                    "stage prime does not match the measured operation",
                ));
            }
        }
    }
    match browser.preparation {
        BrowserPreparation::FreshInstance
            if m.scope == CallScope::CompleteCall && m.mode == SampleMode::SingleCall => {}
        BrowserPreparation::PrimedSample
            if m.scope == CallScope::CompleteCall
                && m.mode == SampleMode::SingleCall
                && browser.cache.sample_prime().is_some() => {}
        BrowserPreparation::PrimedInstance if m.scope == CallScope::CompleteCall => {}
        BrowserPreparation::InitializationBytes | BrowserPreparation::InitializationCompiled
            if m.scope == CallScope::Initialization
                && m.mode == SampleMode::SingleCall
                && browser.accepted == BrowserBackend::Package
                && browser.candidate == BrowserBackend::Package => {}
        _ => {
            return Err(io::Error::other(
                "unsupported browser scope/preparation/mode or TypeScript initialization",
            ))
        }
    }
    if [browser.accepted, browser.candidate].contains(&BrowserBackend::TypeScript) {
        // Runtime admission also checks source colors and the actual nearest coordinate map.
        let indexed_supported = match &browser.operation {
            PublicOperation::Quantize { settings } => {
                settings.matching == super::quantize::MatchPolicy::SrgbEuclidean
                    && matches!(
                        settings.alpha,
                        super::quantize::AlphaPolicy::Preserve { .. }
                    )
            }
            PublicOperation::Process { settings } => {
                use ditherette_wasm::spec::contract::request as spec;
                settings.recipe.matching == spec::MatchPolicy::SrgbEuclidean
                    && matches!(settings.recipe.alpha, spec::AlphaPolicy::Preserve { .. })
                    && matches!(settings.recipe.dither, spec::DitherPolicy::None {})
                    && matches!(
                        settings.recipe.output.resize,
                        spec::ResizePolicy::Nearest {
                            anchor: spec::Anchor::Center
                        }
                    )
            }
            _ => true,
        };
        if !indexed_supported {
            return Err(io::Error::other("no faithful TypeScript indexed comparison outside nearest/no-dither/sRGB/preserve alpha"));
        }
        if matches!(
            browser.operation,
            PublicOperation::Yliluoma { .. }
                | PublicOperation::Diffusion { .. }
                | PublicOperation::Perturb { .. }
                | PublicOperation::Separable { .. }
        ) {
            return Err(io::Error::other(
                "no faithful TypeScript field, diffusion, or mixing adapter is registered",
            ));
        }
        if matches!(
            browser.operation,
            PublicOperation::ResizeBicubic { .. } | PublicOperation::ResizeTrilinear { .. }
        ) {
            return Err(io::Error::other(
                "The website has no bicubic or trilinear implementation",
            ));
        }
        if browser.operation.anchor() != Anchor::Center {
            return Err(io::Error::other(
                "TypeScript resize supports only the center anchor",
            ));
        }
    }
    if case.identity
        != browser
            .operation
            .identity(case.source, &case.rgba, case.identity.output)?
        || case.reference_subject != browser.operation.reference_subject()
        || case.accepted_subject != browser.operation.subject(browser.accepted)
        || case.candidate_subject != browser.operation.subject(browser.candidate)
    {
        return Err(io::Error::other(
            "browser operation, subjects, input, or settings identity differs",
        ));
    }
    Ok(())
}

pub fn validate_preparation_cache(
    cache: CacheCapability,
    measurement: &Measurement,
) -> io::Result<()> {
    if !matches!(cache, CacheCapability::Roles { .. })
        || measurement.application_cache == ApplicationCache::NotApplicable
        || measurement.mode != SampleMode::SingleCall
        || !matches!(
            measurement.scope,
            CallScope::NativeCompleteCall | CallScope::CompleteCall
        )
    {
        return Err(io::Error::other("preparation comparison requires explicit role capabilities and cold/warm single complete calls"));
    }
    if let CacheCapability::Roles {
        accepted,
        candidate,
        sample_prime,
    } = cache
    {
        let stages = [accepted, candidate].contains(&PreparationCapability::ImageStages);
        if sample_prime.is_some()
            != (stages && measurement.application_cache == ApplicationCache::Warm)
        {
            return Err(io::Error::other("image-stage warmth requires an explicit per-sample prime; cold and preparation-only cases have none"));
        }
    }
    Ok(())
}

fn relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && std::path::Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn validate_tree(tree: &AssetTree) -> io::Result<()> {
    let mut paths = BTreeSet::new();
    if !tree.root.is_absolute()
        || tree.files.is_empty()
        || tree.files.iter().any(|file| {
            !relative(&file.path)
                || !paths.insert(&file.path)
                || file.mode & 0o222 != 0
                || file.mode & !0o777 != 0
        })
        || tree
            .files
            .windows(2)
            .any(|pair| pair[0].path >= pair[1].path)
        || tree.digest != tree_digest(&tree.files)?
    {
        return Err(io::Error::other(
            "browser tree manifest is not complete, sorted, immutable, or correctly identified",
        ));
    }
    for file in &tree.files {
        let Some(alias) = &file.alias_of else {
            continue;
        };
        if alias >= &file.path
            || !tree.files.iter().any(|target| {
                target.path == *alias
                    && target.alias_of.is_none()
                    && target.bytes == file.bytes
                    && target.mode == file.mode
                    && target.digest == file.digest
            })
        {
            return Err(io::Error::other("asset alias must identify an earlier canonical file with identical content and mode"));
        }
    }
    Ok(())
}

fn entry(tree: &AssetTree, path: &str) -> bool {
    relative(path) && tree.files.iter().any(|file| file.path == path)
}

/// Structural validation is separate from the asset owner's filesystem verification.
pub fn validate_trial(trial: &BrowserTrial) -> io::Result<()> {
    validate_tree(&trial.assets.tree)?;
    let e = &trial.assets.entries;
    if [&e.package, &e.typescript, &e.transport, &e.page, &e.wasm]
        .iter()
        .any(|path| !entry(&trial.assets.tree, path))
    {
        return Err(io::Error::other(
            "browser entrypoint is absent from its served closure",
        ));
    }
    let runtime = &trial.runtime;
    validate_tree(&runtime.browser_assets)?;
    validate_tree(&runtime.playwright.tree)?;
    let browser_entry = runtime
        .browser
        .path
        .strip_prefix(&runtime.browser_assets.root)
        .ok()
        .and_then(|path| path.to_str());
    if !browser_entry.is_some_and(|path| {
        relative(path)
            && runtime
                .browser_assets
                .files
                .iter()
                .any(|file| file.path == path && file.digest == runtime.browser.digest)
    }) {
        return Err(io::Error::other(
            "browser launch entry is absent from its runtime closure",
        ));
    }
    if !entry(&runtime.playwright.tree, &runtime.playwright.entry)
        || runtime.playwright.version.is_empty()
        || [&runtime.node, &runtime.browser].iter().any(|binary| {
            !binary.path.is_absolute() || binary.version.is_empty() || binary.digest.0 == [0; 32]
        })
    {
        return Err(io::Error::other("browser runtime identity is incomplete"));
    }
    Ok(())
}

pub fn validate_observation(
    expected: &BrowserRuntime,
    actual: &BrowserObservation,
) -> io::Result<()> {
    if actual.engine != expected.engine
        || actual.browser_version != expected.browser.version
        || actual.node_version != expected.node.version
        || actual.playwright_version != expected.playwright.version
        || actual.cross_origin_isolated != expected.cross_origin_isolated
        || actual.user_agent.is_empty()
        || !actual.timer_resolution_ns.is_finite()
        || actual.timer_resolution_ns <= 0.0
    {
        return Err(io::Error::other(
            "observed browser/runtime/timer evidence differs or is incomplete",
        ));
    }
    Ok(())
}

pub(super) fn validate_evidence(
    prepared: &PreparedPair,
    case: &PairCase,
    result: &TrialResult,
) -> io::Result<ArtifactIdentity> {
    validate_case(case)?;
    let worker = match result.role {
        Role::Accepted => &prepared.accepted.identity,
        Role::Candidate => &prepared.candidate.identity,
    };
    let Some(browser_case) = &case.browser else {
        return if result.browser.is_none() {
            Ok(worker.clone())
        } else {
            Err(io::Error::other("native result contains browser claims"))
        };
    };
    let trial = prepared
        .browser
        .as_ref()
        .ok_or_else(|| io::Error::other("missing prepared browser assets"))?
        .trial(result.role);
    validate_trial(&trial)?;
    let evidence = result
        .browser
        .as_ref()
        .ok_or_else(|| io::Error::other("missing browser runtime evidence"))?;
    validate_observation(&trial.runtime, &evidence.observation)?;
    match (browser_case.row_policy, evidence.observation.row_policy) {
        (None, None) => {}
        (Some(policy), Some(observed)) => {
            let expected = match result.role {
                Role::Accepted => policy.accepted,
                Role::Candidate => policy.candidate,
            };
            if observed.stage != policy.stage
                || observed.parameters != expected
                || !(1..=8).contains(&observed.pool_size)
                || expected.active_workers > observed.pool_size
            {
                return Err(io::Error::other(
                    "observed row policy differs from the measured role",
                ));
            }
        }
        _ => {
            return Err(io::Error::other(
                "missing or unexpected row-policy observation",
            ))
        }
    }
    if evidence.assets != trial.assets.tree.digest
        || evidence.runtime != runtime_digest(&trial.runtime)?
        || evidence.backend != browser_case.backend(result.role)
        || evidence.preparation != browser_case.preparation
        || evidence.cache != browser_case.cache
        || evidence.progress != browser_case.progress
        || evidence.threads != browser_case.threads
        || evidence.observation.execution.unwrap_or_default()
            != browser_case.execution.unwrap_or_default()
        || evidence.measure_nonexact != browser_case.measure_nonexact
    {
        return Err(io::Error::other(
            "browser trial artifact or preparation differs",
        ));
    }
    artifact_identity(worker, &trial.assets, &trial.runtime)
}
