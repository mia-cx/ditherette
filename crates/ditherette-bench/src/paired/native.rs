//! Center-anchor native recipes bind support policy independently of implementation IDs.

use super::PairCase;
use crate::verification::{input_digest, settings_digest};
use ditherette_bench_api::{verification::*, SubjectId};
use std::io;

fn reference_for(subject: &str) -> io::Result<String> {
    let id = SubjectId::parse(subject).map_err(io::Error::other)?;
    if id.domain() != "resize" {
        return Err(io::Error::other("native recipe requires a resize subject"));
    }
    let variant = match (id.module(), id.filter(), id.variant()) {
        ("spec" | "prod", "nearest" | "area" | "bilinear", "scalar") => "scalar",
        ("spec" | "prod", "bicubic", "catmull-rom") => "catmull-rom",
        ("spec" | "prod", "bicubic", "catmull-rom-scale-aware") => "catmull-rom-scale-aware",
        ("spec" | "prod", "lanczos2" | "lanczos3", "fixed") => "fixed",
        ("spec" | "prod", "lanczos2" | "lanczos3", "scale-aware") => "scale-aware",
        ("spec" | "prod", "trilinear", "mip-area") => "mip-area",
        ("candidate", "nearest", "legacy" | "incremental") => "scalar",
        ("candidate", "area" | "bilinear", "budgeted") => "scalar",
        ("candidate", "bicubic", "budgeted-fixed") => "catmull-rom",
        ("candidate", "bicubic", "budgeted-scale-aware") => "catmull-rom-scale-aware",
        ("candidate", "lanczos2" | "lanczos3", "budgeted-fixed") => "fixed",
        ("candidate", "lanczos2" | "lanczos3", "budgeted-scale-aware") => "scale-aware",
        _ => return Err(io::Error::other("unregistered native semantic recipe")),
    };
    Ok(format!("spec:resize:{}:{variant}", id.filter()))
}

/// Build a normalized identity from the exact frozen recipe, including support policy.
pub fn identity(
    reference: &str,
    source: Dimensions,
    rgba: &[u8],
    output: Dimensions,
) -> io::Result<CaseIdentity> {
    if reference_for(reference)? != reference {
        return Err(io::Error::other(
            "native reference must name a frozen spec subject",
        ));
    }
    let id = SubjectId::parse(reference).map_err(io::Error::other)?;
    let recipe = if id.variant() == "scalar" {
        // Preserve existing nearest/area/bilinear identities, which have no support option.
        format!("{}-center-default", id.filter())
    } else {
        format!("{}-{}-center-default", id.filter(), id.variant())
    };
    let semantics = SemanticIdentity {
        operation: Operation::Resize,
        recipe,
        version: 1,
        space: None,
    };
    Ok(CaseIdentity {
        input: input_digest(source, rgba),
        settings: settings_digest(&(semantics.clone(), output, "center-default"))
            .map_err(io::Error::other)?,
        semantics,
        output,
    })
}

/// Reject mismatched support policies even when all subjects have the same filter name.
pub fn validate_case(case: &PairCase) -> io::Result<()> {
    for subject in [&case.accepted_subject, &case.candidate_subject] {
        if reference_for(subject)? != case.reference_subject {
            return Err(io::Error::other(
                "native subject differs from the frozen recipe",
            ));
        }
    }
    if case.identity
        != identity(
            &case.reference_subject,
            case.source,
            &case.rgba,
            case.identity.output,
        )?
    {
        return Err(io::Error::other(
            "native normalized recipe identity differs",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_policy_changes_the_semantic_identity() {
        let dimensions = Dimensions {
            width: 1,
            height: 1,
        };
        for filter in ["bicubic", "lanczos2", "lanczos3"] {
            let fixed = if filter == "bicubic" {
                "catmull-rom"
            } else {
                "fixed"
            };
            let scaled = if filter == "bicubic" {
                "catmull-rom-scale-aware"
            } else {
                "scale-aware"
            };
            let a = identity(
                &format!("spec:resize:{filter}:{fixed}"),
                dimensions,
                &[1, 2, 3, 255],
                dimensions,
            )
            .unwrap();
            let b = identity(
                &format!("spec:resize:{filter}:{scaled}"),
                dimensions,
                &[1, 2, 3, 255],
                dimensions,
            )
            .unwrap();
            assert_ne!(a.semantics, b.semantics);
            assert_ne!(a.settings, b.settings);
            assert_eq!(
                reference_for(&format!("candidate:resize:{filter}:budgeted-fixed")).unwrap(),
                format!("spec:resize:{filter}:{fixed}")
            );
            assert_eq!(
                reference_for(&format!("candidate:resize:{filter}:budgeted-scale-aware")).unwrap(),
                format!("spec:resize:{filter}:{scaled}")
            );
        }
        assert!(reference_for("candidate:resize:bicubic:invented").is_err());
    }
}
