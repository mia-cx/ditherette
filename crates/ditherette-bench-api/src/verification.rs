//! Typed conformance records. Concrete request types remain in the implementation crate.

use crate::BenchSubjectError;
use serde::{Deserialize, Serialize};

/// Complete SHA-256 content identity. Serialization retains all 256 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Digest256(pub [u8; 32]);

/// Semantic operation, independent of scalar/threaded execution policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    MetricScores,
    FieldEvaluation,
    PlacementMask,
    ColorInverse,
    Resize,
    Color,
    Perturb,
    Quantize,
    DitherAndQuantize,
    Process,
}

/// Named coordinates used by color results and semantic case identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorSpace {
    Srgb,
    LinearRgb,
    Oklab,
    Oklch,
    Cielab,
    Cielch,
    Ycbcr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

/// Named recipe identity shared by reference, accepted implementation, and candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticIdentity {
    pub operation: Operation,
    pub recipe: String,
    pub version: u32,
    /// Primary output-coordinate or matching space, not the complete stage recipe.
    /// The settings digest must also bind every independent perturb/matching space.
    pub space: Option<ColorSpace>,
}

/// The complete semantic case. Digests cover source bytes/dimensions and normalized settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseIdentity {
    pub semantics: SemanticIdentity,
    pub input: Digest256,
    pub settings: Digest256,
    pub output: Dimensions,
}

/// A named build and its full content digest, never a historical timing label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactIdentity {
    pub revision: String,
    pub content: Digest256,
}

/// A concrete executable adapter for one named artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImplementationIdentity {
    pub subject: String,
    pub artifact: ArtifactIdentity,
}

/// Raw requests stay typed, including their borrowed input lifetimes.
pub struct VerificationCase<P> {
    pub identity: CaseIdentity,
    pub request: P,
}

/// An adapter accepts its implementation's concrete request type.
pub struct VerificationSubject<P> {
    pub identity: ImplementationIdentity,
    pub semantics: SemanticIdentity,
    pub run: fn(&P) -> Result<VerificationOutput, BenchSubjectError>,
}

impl<P> VerificationSubject<P> {
    /// Run a matching typed case and retain the exact case and artifact identities.
    pub fn evaluate(
        &self,
        case: &VerificationCase<P>,
    ) -> Result<RecordedOutput, BenchSubjectError> {
        if self.semantics != case.identity.semantics {
            return Err(BenchSubjectError::new(
                "subject and case semantic identities differ",
            ));
        }
        Ok(RecordedOutput {
            case: case.identity.clone(),
            implementation: self.identity.clone(),
            output: (self.run)(&case.request)?,
        })
    }
}

/// Stable warning codes mirror caller-visible result metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WarningCode {
    PaletteTruncated,
    TransparentOnly,
    TransparentFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning {
    pub code: WarningCode,
    pub message: String,
}

/// Raw outputs retain metadata and coordinates even when rendered pixels agree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "kebab-case")]
pub enum Pixels {
    /// One scalar score per fixture position, not an image or rendered color.
    Scores {
        #[serde(with = "float_bits", rename = "score_bits")]
        values: Vec<f32>,
    },
    Rgba8 {
        data: Vec<u8>,
    },
    Indexed8 {
        indices: Vec<u8>,
        palette_rgba: Vec<u8>,
        transparent_index: Option<u8>,
    },
    Color {
        space: ColorSpace,
        #[serde(with = "float_bits", rename = "coordinate_bits")]
        coordinates: Vec<f32>,
        alpha: Vec<u8>,
        /// Actual inverse-rendered pixels, when the reference adapter provides them.
        rendered_rgba: Option<Vec<u8>>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationOutput {
    pub dimensions: Dimensions,
    pub pixels: Pixels,
    pub warnings: Vec<Warning>,
}

/// Independently computed case and exact output from the isolated frozen oracle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OracleOutput {
    pub case: CaseIdentity,
    pub output: VerificationOutput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedOutput {
    pub case: CaseIdentity,
    pub implementation: ImplementationIdentity,
    pub output: VerificationOutput,
}

/// Reference completion is explicit; pre-freeze exactness is provisional evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReferenceState {
    PreFreeze,
    Frozen,
}

/// Required role slots are explicit. An absent slot is incomplete evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeWayOutputs {
    pub reference_state: ReferenceState,
    pub reference: Option<RecordedOutput>,
    pub accepted: Option<RecordedOutput>,
    pub candidate: Option<RecordedOutput>,
}

mod float_bits {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(values: &[f32], serializer: S) -> Result<S::Ok, S::Error> {
        values
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<f32>, D::Error> {
        Ok(Vec::<u32>::deserialize(deserializer)?
            .into_iter()
            .map(f32::from_bits)
            .collect())
    }
}
