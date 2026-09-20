//! Stable benchmark subject contracts shared by Ditherette crates.
//!
//! This crate intentionally contains only small typed contracts. Benchmarkable
//! implementation adapters live in the implementation crate, while the CLI and
//! measurement loops live in `ditherette-bench`.

use std::{collections::BTreeMap, error::Error, fmt, str::FromStr};

pub mod verification;

/// Stable identifier for a benchmarkable implementation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectId(String);

impl SubjectId {
    /// Parses `module:domain:filter:variant` subject IDs.
    pub fn parse(value: impl Into<String>) -> Result<Self, SubjectIdError> {
        let value = value.into();
        let parts: Vec<_> = value.split(':').collect();
        if parts.len() != 4 || parts.iter().any(|part| part.is_empty()) {
            return Err(SubjectIdError { value });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn module(&self) -> &str {
        self.part(0)
    }

    pub fn domain(&self) -> &str {
        self.part(1)
    }

    pub fn filter(&self) -> &str {
        self.part(2)
    }

    pub fn variant(&self) -> &str {
        self.part(3)
    }

    fn part(&self, index: usize) -> &str {
        self.0
            .split(':')
            .nth(index)
            .expect("validated subject IDs have four parts")
    }
}

impl fmt::Display for SubjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for SubjectId {
    type Err = SubjectIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Invalid subject ID syntax.
#[derive(Debug, Clone)]
pub struct SubjectIdError {
    value: String,
}

impl fmt::Display for SubjectIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid subject id {:?}; expected module:domain:filter:variant",
            self.value
        )
    }
}

impl Error for SubjectIdError {}

/// Supported pixel formats for initial benchmark contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PixelFormat {
    Rgba8,
    Indexed8,
    Color32,
    Score32,
}

impl fmt::Display for PixelFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rgba8 => formatter.write_str("rgba8"),
            Self::Indexed8 => formatter.write_str("indexed8"),
            Self::Color32 => formatter.write_str("color32"),
            Self::Score32 => formatter.write_str("score32"),
        }
    }
}

/// Capabilities exposed by a benchmark subject.
#[derive(Debug, Clone)]
pub struct SubjectCapabilities {
    pub pixel_formats: Vec<PixelFormat>,
    pub supports_strided_io: bool,
    pub supports_tiling_params: bool,
    pub scalar_control: Option<SubjectId>,
}

impl SubjectCapabilities {
    pub fn rgba8_packed() -> Self {
        Self {
            pixel_formats: vec![PixelFormat::Rgba8],
            supports_strided_io: false,
            supports_tiling_params: false,
            scalar_control: None,
        }
    }
}

/// Human- and machine-readable subject metadata.
#[derive(Debug, Clone)]
pub struct SubjectDescriptor {
    pub id: SubjectId,
    pub display_name: String,
    pub source_file: String,
    pub source_line: u32,
    pub default_oracle: Option<SubjectId>,
    pub capabilities: SubjectCapabilities,
    pub params_schema: ParamSchema,
}

/// Placeholder for future per-subject parameter schemas.
#[derive(Debug, Clone, Default)]
pub struct ParamSchema {
    pub fields: Vec<ParamField>,
}

/// One declared parameter field.
#[derive(Debug, Clone)]
pub struct ParamField {
    pub name: String,
    pub description: String,
}

/// Runtime parameter map for extension params.
pub type ParamMap = BTreeMap<String, String>;

/// Resize anchor parameter mirrored from the implementation crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizeAnchorParam {
    TopLeft,
    Top,
    TopRight,
    Left,
    #[default]
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

/// Support policy for reconstruction filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportPolicyParam {
    Fixed,
    ScaleAware,
}

/// Optional tiling policy parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TilePolicyParam {
    pub tile_height: Option<u32>,
    pub worker_count: Option<u32>,
    pub chunking_strategy: Option<String>,
}

/// Resize parameters shared by resize subjects.
#[derive(Debug, Clone)]
pub struct ResizeParams {
    pub anchor: ResizeAnchorParam,
    pub support_policy: Option<SupportPolicyParam>,
    pub tile_policy: Option<TilePolicyParam>,
    pub extra: ParamMap,
}

impl Default for ResizeParams {
    fn default() -> Self {
        Self {
            anchor: ResizeAnchorParam::Center,
            support_policy: None,
            tile_policy: None,
            extra: ParamMap::new(),
        }
    }
}

/// Borrowed RGBA8 resize input.
#[derive(Debug, Clone, Copy)]
pub struct ResizeInputU8Rgba<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub row_stride_elements: usize,
}

/// Borrowed RGBA8 resize output.
#[derive(Debug)]
pub struct ResizeOutputU8Rgba<'a> {
    pub data: &'a mut [u8],
    pub width: u32,
    pub height: u32,
    pub row_stride_elements: usize,
}

/// Error returned by a benchmark subject adapter.
#[derive(Debug, Clone)]
pub struct BenchSubjectError {
    message: String,
}

impl BenchSubjectError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for BenchSubjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl Error for BenchSubjectError {}

/// Function pointer type for RGBA8 resize subjects.
pub type ResizeU8RgbaFn = for<'a> fn(
    ResizeInputU8Rgba<'a>,
    ResizeOutputU8Rgba<'a>,
    &ResizeParams,
) -> Result<(), BenchSubjectError>;

/// Registered resize subject.
#[derive(Clone)]
pub struct ResizeBenchSubject {
    pub descriptor: SubjectDescriptor,
    pub resize_u8_rgba: ResizeU8RgbaFn,
}

impl ResizeBenchSubject {
    pub fn resize_u8_rgba(
        &self,
        input: ResizeInputU8Rgba<'_>,
        output: ResizeOutputU8Rgba<'_>,
        params: &ResizeParams,
    ) -> Result<(), BenchSubjectError> {
        (self.resize_u8_rgba)(input, output, params)
    }
}

/// A callable typed conformance adapter. The implementation crate owns its request type.
#[derive(Clone)]
pub struct ConformanceBenchSubject<F> {
    pub descriptor: SubjectDescriptor,
    pub operation: verification::Operation,
    pub run: F,
}

/// Registered benchmark subject. Legacy users need no concrete conformance protocol.
#[derive(Clone)]
pub enum BenchSubject<F = std::convert::Infallible> {
    Resize(ResizeBenchSubject),
    Conformance(ConformanceBenchSubject<F>),
}

impl<F> BenchSubject<F> {
    pub fn descriptor(&self) -> &SubjectDescriptor {
        match self {
            Self::Resize(subject) => &subject.descriptor,
            Self::Conformance(subject) => &subject.descriptor,
        }
    }
}
