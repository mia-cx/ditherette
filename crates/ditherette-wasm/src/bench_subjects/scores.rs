//! Standalone scalar scores over a versioned cyclic-successor fixture.
//! Conversion uses frozen forward functions before timing. Alpha is not a metric input.

use super::{reference::ReferenceRequest, BenchSubject};
use crate::spec::{
    self,
    contract::request::{Source, WorkingSpace},
};
use ditherette_bench_api::{
    verification::*, BenchSubjectError, ConformanceBenchSubject, ParamSchema, PixelFormat,
    SubjectCapabilities, SubjectDescriptor, SubjectId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MetricFamily {
    Euclidean,
    Chord,
    Arc,
    Compuphase,
    Rec601,
    Rec709,
    Ciede2000,
}

pub type ScorePair = [[f32; 3]; 2];
pub type ScoreFn = fn([f32; 3], [f32; 3]) -> f32;

impl MetricFamily {
    pub const ALL: [Self; 7] = [
        Self::Euclidean,
        Self::Chord,
        Self::Arc,
        Self::Compuphase,
        Self::Rec601,
        Self::Rec709,
        Self::Ciede2000,
    ];

    /// Coordinate domains are part of the versioned fixture recipe.
    pub fn space(self) -> WorkingSpace {
        match self {
            Self::Euclidean => WorkingSpace::Srgb,
            Self::Chord => WorkingSpace::Oklch,
            Self::Arc => WorkingSpace::Oklch,
            Self::Compuphase => WorkingSpace::Srgb,
            Self::Rec601 => WorkingSpace::Srgb,
            Self::Rec709 => WorkingSpace::Srgb,
            Self::Ciede2000 => WorkingSpace::Cielab,
        }
    }

    pub fn reference_subject(self) -> &'static str {
        match self {
            Self::Euclidean => "spec:metric:euclidean:cyclic-scores-v1",
            Self::Chord => "spec:metric:chord:cyclic-scores-v1",
            Self::Arc => "spec:metric:arc:cyclic-scores-v1",
            Self::Compuphase => "spec:metric:compuphase:cyclic-scores-v1",
            Self::Rec601 => "spec:metric:rec601:cyclic-scores-v1",
            Self::Rec709 => "spec:metric:rec709:cyclic-scores-v1",
            Self::Ciede2000 => "spec:metric:ciede2000:cyclic-scores-v1",
        }
    }

    pub fn prod_subject(self) -> &'static str {
        match self {
            Self::Euclidean => "prod:metric:euclidean:cyclic-scores-v1",
            Self::Chord => "prod:metric:chord:cyclic-scores-v1",
            Self::Arc => "prod:metric:arc:cyclic-scores-v1",
            Self::Compuphase => "prod:metric:compuphase:cyclic-scores-v1",
            Self::Rec601 => "prod:metric:rec601:cyclic-scores-v1",
            Self::Rec709 => "prod:metric:rec709:cyclic-scores-v1",
            Self::Ciede2000 => "prod:metric:ciede2000:cyclic-scores-v1",
        }
    }

    /// Select outside timing; the batch invokes the existing metric unchanged.
    pub fn prod_function(self) -> ScoreFn {
        use crate::prod::quantize::metric::*;
        match self {
            Self::Euclidean => euclidean3_squared,
            Self::Chord => circular_hue3_squared,
            Self::Arc => hue_arc3_squared,
            Self::Compuphase => |a, b| weighted_rgb_squared(a, b, WeightedRgbMetric::CompuPhase),
            Self::Rec601 => |a, b| weighted_rgb_squared(a, b, WeightedRgbMetric::Rec601),
            Self::Rec709 => |a, b| weighted_rgb_squared(a, b, WeightedRgbMetric::Rec709),
            Self::Ciede2000 => ciede2000_distance,
        }
    }

    fn reference_function(self) -> ScoreFn {
        use crate::spec::quantize::metric::*;
        match self {
            Self::Euclidean => euclidean3_squared,
            Self::Chord => circular_hue3_squared,
            Self::Arc => hue_arc3_squared,
            Self::Compuphase => |a, b| weighted_rgb_squared(a, b, WeightedRgbMetric::CompuPhase),
            Self::Rec601 => |a, b| weighted_rgb_squared(a, b, WeightedRgbMetric::Rec601),
            Self::Rec709 => |a, b| weighted_rgb_squared(a, b, WeightedRgbMetric::Rec709),
            Self::Ciede2000 => ciede2000_distance,
        }
    }
}

/// Each source pixel is paired with its cyclic successor in row-major order.
/// Both sides use the frozen forward conversion. Storage and conversion are untimed.
pub fn prepare_pairs(
    source: Source<'_>,
    metric: MetricFamily,
) -> Result<Vec<ScorePair>, BenchSubjectError> {
    ReferenceRequest::MetricScores { source, metric }.dimensions()?;
    let coordinates: Vec<_> = source
        .data
        .chunks_exact(4)
        .map(|p| spec::color::rgb8_to_coordinates([p[0], p[1], p[2]], metric.space()))
        .collect();
    Ok(coordinates
        .iter()
        .enumerate()
        .map(|(i, &a)| [a, coordinates[(i + 1) % coordinates.len()]])
        .collect())
}

/// One complete batch into caller-owned score storage; no allocation or conversion.
pub fn score_into(pairs: &[ScorePair], values: &mut [f32], score: ScoreFn) {
    assert_eq!(pairs.len(), values.len());
    for (pair, output) in pairs.iter().zip(values) {
        *output = score(pair[0], pair[1]);
    }
}

fn output(
    request: &ReferenceRequest<'_>,
    expected: MetricFamily,
    prod: bool,
) -> Result<VerificationOutput, BenchSubjectError> {
    let ReferenceRequest::MetricScores { source, metric } = *request else {
        return Err(BenchSubjectError::new(
            "score subject requires its typed metric request",
        ));
    };
    if metric != expected {
        return Err(BenchSubjectError::new("score subject and metric differ"));
    }
    let dimensions = request.dimensions()?;
    let pairs = prepare_pairs(source, metric)?;
    let mut values = vec![0.0; pairs.len()];
    score_into(
        &pairs,
        &mut values,
        if prod {
            metric.prod_function()
        } else {
            metric.reference_function()
        },
    );
    Ok(VerificationOutput {
        dimensions,
        pixels: Pixels::Scores { values },
        warnings: vec![],
    })
}

pub(super) fn subjects() -> Vec<BenchSubject> {
    let entries: [(
        MetricFamily,
        super::reference::ReferenceFn,
        super::reference::ReferenceFn,
    ); 7] = [
        (
            MetricFamily::Euclidean,
            |r| output(r, MetricFamily::Euclidean, false),
            |r| output(r, MetricFamily::Euclidean, true),
        ),
        (
            MetricFamily::Chord,
            |r| output(r, MetricFamily::Chord, false),
            |r| output(r, MetricFamily::Chord, true),
        ),
        (
            MetricFamily::Arc,
            |r| output(r, MetricFamily::Arc, false),
            |r| output(r, MetricFamily::Arc, true),
        ),
        (
            MetricFamily::Compuphase,
            |r| output(r, MetricFamily::Compuphase, false),
            |r| output(r, MetricFamily::Compuphase, true),
        ),
        (
            MetricFamily::Rec601,
            |r| output(r, MetricFamily::Rec601, false),
            |r| output(r, MetricFamily::Rec601, true),
        ),
        (
            MetricFamily::Rec709,
            |r| output(r, MetricFamily::Rec709, false),
            |r| output(r, MetricFamily::Rec709, true),
        ),
        (
            MetricFamily::Ciede2000,
            |r| output(r, MetricFamily::Ciede2000, false),
            |r| output(r, MetricFamily::Ciede2000, true),
        ),
    ];
    entries
        .into_iter()
        .flat_map(|(metric, reference, prod)| {
            [(false, reference), (true, prod)].map(|(production, run)| {
                BenchSubject::Conformance(ConformanceBenchSubject {
                    descriptor: SubjectDescriptor {
                        id: SubjectId::parse(if production {
                            metric.prod_subject()
                        } else {
                            metric.reference_subject()
                        })
                        .expect("literal ID"),
                        display_name: format!("{metric:?} cyclic-successor scalar score batch"),
                        source_file: format!(
                            "crates/ditherette-wasm/src/{}/quantize/metric.rs",
                            if production { "prod" } else { "spec" }
                        ),
                        source_line: 1,
                        default_oracle: production.then(|| {
                            SubjectId::parse(metric.reference_subject()).expect("literal oracle")
                        }),
                        capabilities: SubjectCapabilities {
                            pixel_formats: vec![PixelFormat::Score32],
                            supports_strided_io: false,
                            supports_tiling_params: false,
                            scalar_control: None,
                        },
                        params_schema: ParamSchema::default(),
                    },
                    operation: Operation::MetricScores,
                    run,
                })
            })
        })
        .collect()
}
