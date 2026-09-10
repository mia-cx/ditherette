//! Readable content identity, capacity, eviction, and publication model.
//!
//! Tiny owned byte values stand for materialized stages. Their declared capacities
//! describe production ownership, not this model's physical Vec allocations.
//! See `cache.md` for the boundary between this model and actual allocation.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[cfg(any(target_arch = "wasm32", test))]
mod sha256;

#[cfg(not(target_arch = "wasm32"))]
use sha2::Sha256 as SourceSha256;
#[cfg(target_arch = "wasm32")]
use sha256::Sha256 as SourceSha256;

use crate::image::contracts::PaletteEntry;

use super::{
    error::{DitheretteError, ErrorCode},
    lifecycle::{InitOptions, InstanceModel, MAX_CACHE_ENTRIES},
    request::{
        AlphaPolicy, DitherPolicy, MatchPolicy, Output, PerturbPolicy, Request, Source,
        WorkingSpace, MAX_PALETTE_ENTRIES, RECIPE_VERSION,
    },
};

/// Full content identity. No source storage, frontend identifier, or execution setting is retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Identity(pub [u8; 32]);

/// Hashes every current source byte and both dimensions, after request validation.
pub fn source_identity(source: Source<'_>) -> Identity {
    let mut hash = SourceSha256::new();
    hash.update(b"ditherette-rgba8-input-v1\0");
    hash.update(source.width.to_le_bytes());
    hash.update(source.height.to_le_bytes());
    hash.update(source.data);
    Identity(hash.finalize().into())
}

/// Hashes retained palette order, duplicates, and the warning-affecting truncation flag.
/// Differences beyond the retained prefix are immaterial once truncation is already true.
pub fn palette_identity(entries: &[PaletteEntry]) -> Identity {
    #[derive(Serialize)]
    struct Palette<'a> {
        retained: &'a [PaletteEntry],
        truncated: bool,
    }
    hash_settings(
        b"ditherette-palette-v1\0",
        &Palette {
            retained: &entries[..entries.len().min(MAX_PALETTE_ENTRIES)],
            truncated: entries.len() > MAX_PALETTE_ENTRIES,
        },
    )
}

/// Typed semantic stages shared across public methods, never keyed by the calling method.
/// Quantize and no-dither fused calls both use Indexed with DitherPolicy::None.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "stage", rename_all = "kebab-case")]
pub enum StageOptions {
    Resize {
        output: Output,
    },
    Perturb {
        perturb: PerturbPolicy,
    },
    Color {
        space: WorkingSpace,
    },
    Alpha {
        palette: Identity,
        alpha: AlphaPolicy,
    },
    Indexed {
        palette: Identity,
        alpha: AlphaPolicy,
        matching: MatchPolicy,
        dither: DitherPolicy,
    },
    PreparedPalette {
        palette: Identity,
        alpha: AlphaPolicy,
        matching: MatchPolicy,
    },
    ResizePlan {
        source_width: u32,
        source_height: u32,
        output: Output,
    },
}

/// Hashes a parent and validated typed settings, preserving f64 alpha thresholds.
/// Image stages use their parent identity; source-independent preparation uses None.
/// Signed floating zero is normalized because both signs have identical request semantics.
pub fn stage_identity(parent: Option<Identity>, version: u32, stage: StageOptions) -> Identity {
    #[derive(Serialize)]
    struct Stage {
        parent: Option<Identity>,
        version: u32,
        options: StageOptions,
    }
    hash_settings(
        b"ditherette-stage-v1\0",
        &Stage {
            parent,
            version,
            options: stage,
        },
    )
}

/// A logical dependency or a public RGBA8 boundary. Content and operation identities are distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityParent {
    Input,
    Operation(usize),
    RgbaOutput(usize),
}

/// A named semantic operation. Describing it does not allocate or cache an intermediate.
#[derive(Debug, Clone, Copy)]
pub struct IdentityStage {
    pub parent: IdentityParent,
    pub options: StageOptions,
}

/// Canonical request composition. RGBA boundary parents resolve after output bytes exist.
#[derive(Debug)]
pub struct IdentityPlan {
    pub input: Identity,
    pub preparation: Vec<Identity>,
    pub stages: Vec<IdentityStage>,
}

/// Completed keys, with the output's operation identity distinct from any output content digest.
#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedIdentities {
    pub input: Identity,
    pub preparation: Vec<Identity>,
    pub operations: Vec<Identity>,
    pub final_identity: Identity,
}

/// Validates and describes all five methods without executing any image operation.
pub fn request_identity_plan(request: Request<'_>) -> Result<IdentityPlan, DitheretteError> {
    let layout = request.validate()?;
    let dimensions = layout.source.dimensions();
    let mut plan = IdentityPlan {
        input: source_identity(Source {
            width: dimensions.width(),
            height: dimensions.height(),
            data: layout.source.data(),
        }),
        preparation: Vec::new(),
        stages: Vec::new(),
    };
    match request {
        Request::Process(request) => {
            let resize = plan.resize(request.source, request.recipe.output);
            plan.fused(
                IdentityParent::RgbaOutput(resize),
                palette_identity(request.palette),
                request.recipe.alpha,
                request.recipe.matching,
                request.recipe.dither,
            );
        }
        Request::Resize(request) => {
            plan.resize(request.source, request.output);
        }
        Request::Perturb(request) => {
            plan.perturb(IdentityParent::Input, request.perturb);
        }
        Request::Quantize(request) => {
            plan.quantize(
                IdentityParent::Input,
                palette_identity(request.palette),
                request.alpha,
                request.matching,
                DitherPolicy::None {},
            );
        }
        Request::DitherAndQuantize(request) => {
            plan.fused(
                IdentityParent::Input,
                palette_identity(request.quantize.palette),
                request.quantize.alpha,
                request.quantize.matching,
                request.dither,
            );
        }
    }
    Ok(plan)
}

impl IdentityPlan {
    /// Resolves an available operation before execution or cache lookup.
    /// Each supplied pair identifies a prior operation and its materialized RGBA8 content digest.
    pub fn key_at(
        &self,
        index: usize,
        rgba_outputs: &[(usize, Identity)],
    ) -> Result<Identity, DitheretteError> {
        let keys = self.resolve_through(index, rgba_outputs)?;
        Ok(*keys.last().unwrap())
    }

    /// Resolves every operation once required RGBA8 outputs have materialized or hit the cache.
    pub fn resolve(
        &self,
        rgba_outputs: &[(usize, Identity)],
    ) -> Result<ResolvedIdentities, DitheretteError> {
        let operations = self.resolve_through(self.stages.len() - 1, rgba_outputs)?;
        Ok(ResolvedIdentities {
            input: self.input,
            preparation: self.preparation.clone(),
            final_identity: *operations.last().unwrap(),
            operations,
        })
    }

    fn resolve_through(
        &self,
        index: usize,
        rgba_outputs: &[(usize, Identity)],
    ) -> Result<Vec<Identity>, DitheretteError> {
        let stages = self
            .stages
            .get(..=index)
            .ok_or_else(|| control_error("Operation is outside the identity plan."))?;
        let mut keys = Vec::new();
        for stage in stages {
            let parent = match stage.parent {
                IdentityParent::Input => self.input,
                IdentityParent::Operation(parent) => keys[parent],
                IdentityParent::RgbaOutput(parent) => rgba_outputs
                    .iter()
                    .find_map(|&(operation, content)| (operation == parent).then_some(content))
                    .ok_or_else(|| {
                        control_error("RGBA output content identity is not available yet.")
                    })?,
            };
            keys.push(stage_identity(Some(parent), RECIPE_VERSION, stage.options));
        }
        Ok(keys)
    }

    fn append(&mut self, parent: IdentityParent, options: StageOptions) -> usize {
        let index = self.stages.len();
        self.stages.push(IdentityStage { parent, options });
        index
    }

    fn resize(&mut self, source: Source<'_>, output: Output) -> usize {
        self.preparation.push(stage_identity(
            None,
            RECIPE_VERSION,
            StageOptions::ResizePlan {
                source_width: source.width,
                source_height: source.height,
                output,
            },
        ));
        self.append(IdentityParent::Input, StageOptions::Resize { output })
    }

    fn perturb(&mut self, parent: IdentityParent, perturb: PerturbPolicy) -> usize {
        self.append(
            parent,
            StageOptions::Color {
                space: perturb.space,
            },
        );
        self.append(parent, StageOptions::Perturb { perturb })
    }

    fn fused(
        &mut self,
        parent: IdentityParent,
        palette: Identity,
        alpha: AlphaPolicy,
        matching: MatchPolicy,
        dither: DitherPolicy,
    ) {
        if let DitherPolicy::Separable { perturb } = dither {
            let perturbed = self.perturb(parent, perturb);
            self.quantize(
                IdentityParent::RgbaOutput(perturbed),
                palette,
                alpha,
                matching,
                DitherPolicy::None {},
            );
            return;
        }
        self.quantize(parent, palette, alpha, matching, dither);
    }

    fn quantize(
        &mut self,
        parent: IdentityParent,
        palette: Identity,
        alpha: AlphaPolicy,
        matching: MatchPolicy,
        dither: DitherPolicy,
    ) {
        self.preparation.push(stage_identity(
            None,
            RECIPE_VERSION,
            StageOptions::PreparedPalette {
                palette,
                alpha,
                matching,
            },
        ));
        let alpha_stage = self.append(parent, StageOptions::Alpha { palette, alpha });
        let color_stage = self.append(
            IdentityParent::Operation(alpha_stage),
            StageOptions::Color {
                space: matching.space(),
            },
        );
        self.append(
            IdentityParent::Operation(color_stage),
            StageOptions::Indexed {
                palette,
                alpha,
                matching,
                dither,
            },
        );
    }
}

fn hash_settings(domain: &[u8], settings: &impl Serialize) -> Identity {
    let mut value = serde_json::to_value(settings).expect("validated typed settings serialize");
    normalize_zero(&mut value);
    // serde_json's default object map sorts keys, independent of struct field ordering.
    let bytes = serde_json::to_vec(&value).expect("JSON values serialize");
    let mut hash = Sha256::new();
    hash.update(domain);
    hash.update(bytes);
    Identity(hash.finalize().into())
}

fn normalize_zero(value: &mut Value) {
    match value {
        Value::Number(number) if number.is_f64() && number.as_f64() == Some(0.0) => {
            *number = serde_json::Number::from_f64(0.0).unwrap();
        }
        Value::Array(values) => values.iter_mut().for_each(normalize_zero),
        Value::Object(values) => values.values_mut().for_each(normalize_zero),
        _ => {}
    }
}

/// Planned peak capacities, not image lengths. Work includes materialized cache candidates.
/// Returned/caller JS buffers and fixed module overhead are excluded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MemoryPlan {
    pub work: u64,
    pub scratch: u64,
    pub boundary_copies: u64,
}

/// Disjoint modeled owners. Transferring work into pending entries does not add capacity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capacity {
    pub retained: u64,
    pub idle_scratch: u64,
    pub active_scratch: u64,
    pub work: u64,
    pub boundary_copies: u64,
    pub pending: u64,
}

impl Capacity {
    /// Live modeled capacity. Internal transitions keep this within the validated instance limit.
    pub fn total(self) -> u64 {
        self.retained
            + self.idle_scratch
            + self.active_scratch
            + self.work
            + self.boundary_copies
            + self.pending
    }
}

/// Owned small stage value plus its RGBA8 output content identity, when it has that boundary.
/// Cache hits carry the recorded content identity forward without rehashing or retaining the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedValue {
    pub bytes: Vec<u8>,
    pub rgba_content: Option<Identity>,
}

#[derive(Debug)]
struct Entry {
    identity: Identity,
    value: CachedValue,
    capacity: u64,
}

#[derive(Debug)]
struct Call {
    plan: MemoryPlan,
    pending: Vec<Entry>,
}

/// Independent instance ownership, using an oldest-first Vec instead of an optimized cache.
/// The lifecycle model separately guards callbacks, readiness, and reentry.
#[derive(Debug)]
pub struct CacheModel {
    options: InitOptions,
    entries: Vec<Entry>,
    idle_scratch: u64,
    call: Option<Call>,
    disposed: bool,
}

impl CacheModel {
    /// Validates an instance budget without allocating any represented image capacity.
    pub fn new(options: InitOptions) -> Result<Self, DitheretteError> {
        Ok(Self {
            options: options.validate()?,
            entries: Vec::new(),
            idle_scratch: 0,
            call: None,
            disposed: false,
        })
    }

    /// Counts every disjoint owner. Diagnostic access remains private to the reference/runtime.
    pub fn capacity(&self) -> Capacity {
        let mut capacity = Capacity {
            retained: self.entries.iter().map(|entry| entry.capacity).sum(),
            idle_scratch: self.idle_scratch,
            ..Capacity::default()
        };
        if let Some(call) = &self.call {
            capacity.active_scratch = call.plan.scratch;
            capacity.work = call.plan.work;
            capacity.boundary_copies = call.plan.boundary_copies;
            capacity.pending = call.pending.iter().map(|entry| entry.capacity).sum();
        }
        capacity
    }

    /// Returns published identities in least-to-most-recently-used order for reference tests.
    pub fn retained_identities(&self) -> Vec<Identity> {
        self.entries.iter().map(|entry| entry.identity).collect()
    }

    /// Reserves a call's peak before processing. Pressure drops idle scratch, then LRU entries.
    /// Explicit reuse transfers all idle scratch into active ownership and counts its full capacity.
    pub fn begin(
        &mut self,
        mut plan: MemoryPlan,
        reuse_idle_scratch: bool,
    ) -> Result<(), DitheretteError> {
        self.require_ready()?;
        if reuse_idle_scratch {
            if plan.scratch > self.idle_scratch {
                return Err(control_error(
                    "Idle scratch cannot satisfy the requested capacity.",
                ));
            }
            plan.scratch = self.idle_scratch;
        }
        let required = plan
            .work
            .checked_add(plan.scratch)
            .and_then(|total| total.checked_add(plan.boundary_copies))
            .ok_or_else(memory_error)?;
        self.options.preflight(required)?;
        if reuse_idle_scratch {
            self.idle_scratch = 0;
        }
        if self.capacity().total() + required > self.options.memory_limit_bytes {
            self.idle_scratch = 0;
        }
        while self.capacity().total() + required > self.options.memory_limit_bytes {
            self.entries.remove(0);
        }
        self.call = Some(Call {
            plan,
            pending: Vec::new(),
        });
        Ok(())
    }

    /// Reads only published entries and makes a hit most-recently used.
    /// The small owned copy models durable returned ownership, not a physical JS boundary copy.
    pub fn lookup(&mut self, identity: Identity) -> Result<Option<CachedValue>, DitheretteError> {
        self.require_live()?;
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.identity == identity)
        else {
            return Ok(None);
        };
        let entry = self.entries.remove(index);
        let result = entry.value.clone();
        self.entries.push(entry);
        Ok(Some(result))
    }

    /// Stages an already-materialized owned value, transferring capacity out of planned work.
    /// Oversized or already-published/staged values remain uncached and return false.
    pub fn stage(
        &mut self,
        identity: Identity,
        value: CachedValue,
        capacity: u64,
    ) -> Result<bool, DitheretteError> {
        self.require_live()?;
        let call = self
            .call
            .as_mut()
            .ok_or_else(|| control_error("No cache call is active."))?;
        if capacity < value.bytes.len() as u64 || capacity > call.plan.work {
            return Err(control_error(
                "Materialized capacity must fit its planned work ownership.",
            ));
        }
        if capacity > self.options.cache_limit_bytes()
            || self.entries.iter().any(|entry| entry.identity == identity)
            || call.pending.iter().any(|entry| entry.identity == identity)
        {
            return Ok(false);
        }
        call.plan.work -= capacity;
        call.pending.push(Entry {
            identity,
            value,
            capacity,
        });
        Ok(true)
    }

    /// Publishes pending entries only after lifecycle output/completion checks succeed.
    /// Completed active scratch becomes idle; exceeding either cache limit evicts oldest entries.
    pub fn finish(&mut self, instance: &mut InstanceModel) -> Result<(), DitheretteError> {
        self.require_live()?;
        if self.call.is_none() {
            return Err(control_error("No cache call is active."));
        }
        instance.finish()?;
        let call = self.call.take().unwrap();
        self.idle_scratch += call.plan.scratch;
        for entry in call.pending {
            while self.entries.len() >= MAX_CACHE_ENTRIES
                || self.capacity().retained + entry.capacity > self.options.cache_limit_bytes()
            {
                self.entries.remove(0);
            }
            self.entries.push(entry);
        }
        Ok(())
    }

    /// Drops pending/work/active scratch ownership on failure. Existing evictions and hits remain.
    /// The caller separately ends the lifecycle call or reports its thrown callback.
    pub fn fail(&mut self) -> Result<(), DitheretteError> {
        self.require_live()?;
        self.call
            .take()
            .ok_or_else(|| control_error("No cache call is active."))?;
        Ok(())
    }

    /// Models an unexpected browser allocation failure, not this model's own allocator behavior.
    pub fn allocation_failed(&mut self) -> DitheretteError {
        if let Err(error) = self.fail() {
            return error;
        }
        DitheretteError::new(
            ErrorCode::WasmMemoryUnavailable,
            "memory",
            "Wasm allocation was unavailable.",
        )
    }

    /// Releases all modeled owned capacity after lifecycle disposal permits it.
    /// Repeated disposal is harmless; no Wasm page-shrinking guarantee is made.
    pub fn dispose(&mut self, instance: &mut InstanceModel) -> Result<(), DitheretteError> {
        if self.call.is_some() {
            return Err(DitheretteError::new(
                ErrorCode::ReentrantCall,
                "instance",
                "Processing and disposal cannot reenter an active call.",
            ));
        }
        instance.dispose()?;
        self.entries = Vec::new();
        self.idle_scratch = 0;
        self.disposed = true;
        Ok(())
    }

    fn require_live(&self) -> Result<(), DitheretteError> {
        if self.disposed {
            return Err(DitheretteError::new(
                ErrorCode::Disposed,
                "instance",
                "Processor is disposed.",
            ));
        }
        Ok(())
    }

    fn require_ready(&self) -> Result<(), DitheretteError> {
        self.require_live()?;
        if self.call.is_some() {
            return Err(DitheretteError::new(
                ErrorCode::ReentrantCall,
                "instance",
                "Processing and disposal cannot reenter an active call.",
            ));
        }
        Ok(())
    }
}

fn control_error(message: &str) -> DitheretteError {
    DitheretteError::new(ErrorCode::Runtime, "cache", message)
}

fn memory_error() -> DitheretteError {
    DitheretteError::new(
        ErrorCode::MemoryLimit,
        "memoryLimitBytes",
        "Planned allocation capacity exceeds the instance memory limit.",
    )
}
