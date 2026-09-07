//! Three-way conformance, full content identities, and retained review evidence.

mod rgba;
pub use rgba::{verify_with_bounds, VerificationBounds, VerificationReport};

use ditherette_bench_api::{verification::*, SubjectId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io, path::Path};

/// SHA-256 of complete bytes. No shortened identifiers enter artifacts.
pub fn content_digest(bytes: &[u8]) -> Digest256 {
    Digest256(Sha256::digest(bytes).into())
}

/// Includes dimensions and every source byte, with a versioned domain prefix.
pub fn input_digest(dimensions: Dimensions, rgba: &[u8]) -> Digest256 {
    let mut hash = Sha256::new();
    hash.update(b"ditherette-rgba8-input-v1\0");
    hash.update(dimensions.width.to_le_bytes());
    hash.update(dimensions.height.to_le_bytes());
    hash.update(rgba);
    Digest256(hash.finalize().into())
}

/// Hash typed normalized settings using canonical JSON object ordering.
/// Callers validate numeric settings through their concrete request contract first.
pub fn settings_digest<P: Serialize>(settings: &P) -> Result<Digest256, serde_json::Error> {
    let canonical = serde_json::to_value(settings)?;
    Ok(content_digest(&serde_json::to_vec(&canonical)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationStatus {
    Exact,
    NeedsVisualApproval,
    Failed,
    Incomplete,
}

/// Numeric coordinate error is independent of rendered byte-color distance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateReport {
    pub differing_coordinates: usize,
    pub max_abs_delta: f64,
    pub mean_abs_delta: f64,
    pub rms_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairComparison {
    pub exact: bool,
    pub within_rgba_bounds: bool,
    pub metadata_mismatches: Vec<String>,
    pub differing_indices: Option<usize>,
    pub coordinates: Option<CoordinateReport>,
    pub rgba: Option<VerificationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeWayReport {
    pub status: VerificationStatus,
    pub reference_state: ReferenceState,
    pub issues: Vec<String>,
    pub reference_accepted: Option<PairComparison>,
    pub reference_candidate: Option<PairComparison>,
    pub accepted_candidate: Option<PairComparison>,
}

impl ThreeWayReport {
    /// Release conformance requires an exact triplet and a completed frozen reference.
    pub fn release_conformant(&self) -> bool {
        self.status == VerificationStatus::Exact && self.reference_state == ReferenceState::Frozen
    }
}

/// Compare all required roles. Missing, invalid, or unrelated evidence cannot pass.
pub fn verify_three_way(
    expected: &CaseIdentity,
    outputs: &ThreeWayOutputs,
    bounds: VerificationBounds,
) -> ThreeWayReport {
    let mut report = ThreeWayReport {
        status: VerificationStatus::Incomplete,
        reference_state: outputs.reference_state,
        issues: Vec::new(),
        reference_accepted: None,
        reference_candidate: None,
        accepted_candidate: None,
    };
    if expected.semantics.recipe.is_empty() || expected.semantics.version == 0 {
        report
            .issues
            .push("case requires a named, versioned semantic recipe".into());
    }
    if expected.semantics.operation != Operation::Resize && expected.semantics.space.is_none() {
        report
            .issues
            .push("operation requires an explicit working-space identity".into());
    }
    if [
        bounds.max_color_distance,
        bounds.max_mean_color_distance,
        bounds.max_rms_color_distance,
    ]
    .iter()
    .any(|value| !value.is_finite() || *value < 0.0)
    {
        report
            .issues
            .push("RGBA bounds must be finite and nonnegative".into());
    }
    for (role, record) in roles(outputs) {
        let Some(record) = record else {
            report
                .issues
                .push(format!("missing required {role} output"));
            continue;
        };
        if record.case != *expected {
            report.issues.push(format!("{role} case identity differs"));
        }
        let revision = &record.implementation.artifact.revision;
        if ![40, 64].contains(&revision.len())
            || !revision.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            report
                .issues
                .push(format!("{role} requires a full source revision"));
        }
        if SubjectId::parse(&record.implementation.subject).is_err() {
            report
                .issues
                .push(format!("{role} has an invalid subject identity"));
        }
        if record.output.dimensions != expected.output {
            report
                .issues
                .push(format!("{role} output dimensions differ from the case"));
        }
        if !operation_matches(expected.semantics.operation, &record.output.pixels) {
            report
                .issues
                .push(format!("{role} output format differs from the operation"));
        }
        if let Pixels::Color { space, .. } = record.output.pixels {
            if expected.semantics.space != Some(space) {
                report
                    .issues
                    .push(format!("{role} coordinate space differs from the case"));
            }
        }
        if let Err(error) = render_rgba(&record.output) {
            report.issues.push(format!("{role}: {error}"));
        }
    }
    if !report.issues.is_empty() {
        return report;
    }
    let (Some(reference), Some(accepted), Some(candidate)) =
        (&outputs.reference, &outputs.accepted, &outputs.candidate)
    else {
        return report;
    };
    report.reference_accepted = Some(compare(&reference.output, &accepted.output, bounds));
    report.reference_candidate = Some(compare(&reference.output, &candidate.output, bounds));
    report.accepted_candidate = Some(compare(&accepted.output, &candidate.output, bounds));
    let pairs = [
        &report.reference_accepted,
        &report.reference_candidate,
        &report.accepted_candidate,
    ];
    report.status = if pairs.into_iter().flatten().all(|pair| pair.exact) {
        VerificationStatus::Exact
    } else if pairs
        .into_iter()
        .flatten()
        .any(|pair| !pair.metadata_mismatches.is_empty() || !pair.within_rgba_bounds)
    {
        VerificationStatus::Failed
    } else {
        // Numeric bounds describe the candidate; only Mia can approve non-exact output.
        VerificationStatus::NeedsVisualApproval
    };
    report
}

fn roles(outputs: &ThreeWayOutputs) -> [(&str, Option<&RecordedOutput>); 3] {
    [
        ("reference", outputs.reference.as_ref()),
        ("accepted", outputs.accepted.as_ref()),
        ("candidate", outputs.candidate.as_ref()),
    ]
}

fn operation_matches(operation: Operation, pixels: &Pixels) -> bool {
    matches!(
        (operation, pixels),
        (Operation::Resize | Operation::Perturb, Pixels::Rgba8 { .. })
            | (
                Operation::Quantize | Operation::DitherAndQuantize | Operation::Process,
                Pixels::Indexed8 { .. }
            )
            | (Operation::Color, Pixels::Color { .. })
    )
}

fn compare(
    left: &VerificationOutput,
    right: &VerificationOutput,
    bounds: VerificationBounds,
) -> PairComparison {
    let mut mismatches = Vec::new();
    if left.dimensions != right.dimensions {
        mismatches.push("dimensions".into());
    }
    if left.warnings != right.warnings {
        mismatches.push("warnings".into());
    }
    let mut differing_indices = None;
    let mut coordinates = None;
    match (&left.pixels, &right.pixels) {
        (
            Pixels::Indexed8 {
                indices: a,
                palette_rgba: ap,
                transparent_index: at,
            },
            Pixels::Indexed8 {
                indices: b,
                palette_rgba: bp,
                transparent_index: bt,
            },
        ) => {
            differing_indices =
                Some(a.iter().zip(b).filter(|(a, b)| a != b).count() + a.len().abs_diff(b.len()));
            if ap != bp {
                mismatches.push("palette-order-and-rgba".into());
            }
            if at != bt {
                mismatches.push("transparent-index".into());
            }
        }
        (
            Pixels::Color {
                space: a_space,
                coordinates: a,
                alpha: a_alpha,
                rendered_rgba: ar,
            },
            Pixels::Color {
                space: b_space,
                coordinates: b,
                alpha: b_alpha,
                rendered_rgba: br,
            },
        ) => {
            if a_space != b_space {
                mismatches.push("coordinate-space".into());
            }
            if a_alpha != b_alpha {
                mismatches.push("alpha".into());
            }
            if ar.is_some() != br.is_some() {
                mismatches.push("inverse-rendering-availability".into());
            }
            coordinates = Some(coordinate_error(a, b));
        }
        (Pixels::Rgba8 { .. }, Pixels::Rgba8 { .. }) => {}
        _ => mismatches.push("pixel-format".into()),
    }
    let rgba = match (render_rgba(left), render_rgba(right)) {
        (Ok(Some(a)), Ok(Some(b))) => {
            if a.chunks_exact(4)
                .zip(b.chunks_exact(4))
                .any(|(a, b)| a[3] != b[3])
            {
                mismatches.push("alpha".into());
            }
            Some(verify_with_bounds(&a, &b, bounds))
        }
        _ => None,
    };
    let coordinates_exact = coordinates
        .as_ref()
        .is_none_or(|value| value.differing_coordinates == 0);
    let exact = mismatches.is_empty()
        && differing_indices.is_none_or(|count| count == 0)
        && coordinates_exact
        && rgba.as_ref().is_none_or(|value| value.differing_bytes == 0);
    PairComparison {
        exact,
        within_rgba_bounds: coordinates_exact
            && rgba.as_ref().is_none_or(|value| value.within_bounds),
        metadata_mismatches: mismatches,
        differing_indices,
        coordinates,
        rgba,
    }
}

fn coordinate_error(left: &[f32], right: &[f32]) -> CoordinateReport {
    let mut report = CoordinateReport {
        differing_coordinates: 0,
        max_abs_delta: 0.0,
        mean_abs_delta: 0.0,
        rms_delta: 0.0,
    };
    for (&a, &b) in left.iter().zip(right) {
        report.differing_coordinates += usize::from(a.to_bits() != b.to_bits());
        let delta = (f64::from(a) - f64::from(b)).abs();
        report.max_abs_delta = report.max_abs_delta.max(delta);
        report.mean_abs_delta += delta;
        report.rms_delta += delta * delta;
    }
    report.mean_abs_delta /= left.len() as f64;
    report.rms_delta = (report.rms_delta / left.len() as f64).sqrt();
    report
}

/// Validate raw storage and render indexed pixels through the retained palette.
/// Color output renders only when its adapter supplies actual inverse conversion.
pub fn render_rgba(output: &VerificationOutput) -> Result<Option<Vec<u8>>, String> {
    let dimensions = output.dimensions;
    if dimensions.width == 0 || dimensions.height == 0 {
        return Err("empty dimensions".into());
    }
    let pixels = usize::try_from(u64::from(dimensions.width) * u64::from(dimensions.height))
        .map_err(|_| "dimensions overflow storage")?;
    let bytes = pixels.checked_mul(4).ok_or("RGBA length overflow")?;
    match &output.pixels {
        Pixels::Rgba8 { data } => {
            if data.len() != bytes {
                return Err("RGBA length differs from dimensions".into());
            }
            Ok(Some(data.clone()))
        }
        Pixels::Indexed8 {
            indices,
            palette_rgba,
            transparent_index,
        } => {
            let entries = palette_rgba.len() / 4;
            if indices.len() != pixels
                || palette_rgba.len() % 4 != 0
                || !(1..=256).contains(&entries)
            {
                return Err("invalid indexed storage or normalized palette length".into());
            }
            if transparent_index.is_some_and(|index| usize::from(index) >= entries) {
                return Err("transparent index is outside the palette".into());
            }
            let mut rgba = Vec::with_capacity(bytes);
            for &index in indices {
                let start = usize::from(index) * 4;
                let entry = palette_rgba
                    .get(start..start + 4)
                    .ok_or("pixel index is outside the palette")?;
                rgba.extend_from_slice(entry);
            }
            Ok(Some(rgba))
        }
        Pixels::Color {
            coordinates,
            alpha,
            rendered_rgba,
            ..
        } => {
            if coordinates.len() != pixels.checked_mul(3).ok_or("coordinate length overflow")?
                || alpha.len() != pixels
            {
                return Err("packed color storage differs from dimensions".into());
            }
            if coordinates.iter().any(|value| !value.is_finite()) {
                return Err("non-finite color coordinate".into());
            }
            if let Some(rgba) = rendered_rgba {
                if rgba.len() != bytes {
                    return Err("inverse-rendered RGBA length differs from dimensions".into());
                }
                if rgba
                    .chunks_exact(4)
                    .zip(alpha)
                    .any(|(pixel, alpha)| pixel[3] != *alpha)
                {
                    return Err("inverse-rendered alpha differs from raw alpha".into());
                }
            }
            Ok(rendered_rgba.clone())
        }
    }
}

/// Verify and automatically preserve raw results and available images on every non-exact outcome.
/// The caller supplies a fresh directory; existing evidence is never overwritten.
pub fn verify_and_preserve(
    expected: &CaseIdentity,
    outputs: &ThreeWayOutputs,
    bounds: VerificationBounds,
    directory: &Path,
) -> io::Result<ThreeWayReport> {
    let report = verify_three_way(expected, outputs, bounds);
    if report.status != VerificationStatus::Exact {
        write_review_artifacts(directory, expected, outputs, &report)?;
    }
    Ok(report)
}

/// Write an immutable review bundle, including raw outputs that cannot render.
pub fn write_review_artifacts(
    directory: &Path,
    expected: &CaseIdentity,
    outputs: &ThreeWayOutputs,
    report: &ThreeWayReport,
) -> io::Result<()> {
    fs::create_dir(directory)?;
    #[derive(Serialize)]
    struct Artifact<'a> {
        schema: &'static str,
        version: u32,
        case: &'a CaseIdentity,
        outputs: &'a ThreeWayOutputs,
        report: &'a ThreeWayReport,
    }
    let artifact = Artifact {
        schema: "ditherette-verification",
        version: 1,
        case: expected,
        outputs,
        report,
    };
    let bytes = serde_json::to_vec_pretty(&artifact).map_err(io::Error::other)?;
    fs::write(directory.join("results.json"), bytes)?;
    for (role, record) in roles(outputs) {
        let Some(record) = record else {
            continue;
        };
        if let Ok(Some(rgba)) = render_rgba(&record.output) {
            write_png(
                &directory.join(format!("{role}.png")),
                record.output.dimensions,
                &rgba,
            )?;
        }
    }
    for (name, left, right) in [
        (
            "reference-candidate",
            &outputs.reference,
            &outputs.candidate,
        ),
        ("reference-accepted", &outputs.reference, &outputs.accepted),
        ("accepted-candidate", &outputs.accepted, &outputs.candidate),
    ] {
        let (Some(left), Some(right)) = (left, right) else {
            continue;
        };
        if left.output.dimensions != right.output.dimensions {
            continue;
        }
        let (Ok(Some(a)), Ok(Some(b))) = (render_rgba(&left.output), render_rgba(&right.output))
        else {
            continue;
        };
        let difference: Vec<_> = a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .flat_map(|(a, b)| {
                let alpha = a[3].abs_diff(b[3]);
                [
                    a[0].abs_diff(b[0]).max(alpha),
                    a[1].abs_diff(b[1]).max(alpha),
                    a[2].abs_diff(b[2]).max(alpha),
                    255,
                ]
            })
            .collect();
        write_png(
            &directory.join(format!("difference-{name}.png")),
            left.output.dimensions,
            &difference,
        )?;
    }
    Ok(())
}

fn write_png(path: &Path, dimensions: Dimensions, rgba: &[u8]) -> io::Result<()> {
    image::save_buffer_with_format(
        path,
        rgba,
        dimensions.width,
        dimensions.height,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .map_err(io::Error::other)
}
