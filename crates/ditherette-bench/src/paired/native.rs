//! Center-anchor native recipes bind support policy independently of implementation IDs.

use crate::verification::{input_digest, settings_digest};
use ditherette_bench_api::{verification::*, SubjectId};
use std::io;

/// Build a normalized identity from the exact frozen recipe, including support policy.
pub fn identity(
    reference: &str,
    source: Dimensions,
    rgba: &[u8],
    output: Dimensions,
) -> io::Result<CaseIdentity> {
    let id = SubjectId::parse(reference).map_err(io::Error::other)?;
    if id.module() != "spec" || id.domain() != "resize" {
        return Err(io::Error::other(
            "native reference must name a frozen spec subject",
        ));
    }
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
        }
        assert!(identity(
            "prod:resize:bicubic:catmull-rom",
            dimensions,
            &[1, 2, 3, 255],
            dimensions
        )
        .is_err());
    }
}
