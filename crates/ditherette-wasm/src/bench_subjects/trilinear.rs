//! Complete native trilinear preparation and execution, excluding caller-owned output allocation.

use super::*;
use crate::prod::resize::scalar::trilinear::PreparedTrilinear;

pub(super) fn subject() -> BenchSubject {
    resize_subject_with_oracle(
        "prod:resize:trilinear:mip-area",
        "prepared trilinear per-call allocation and execution",
        "crates/ditherette-wasm/src/bench_subjects/trilinear.rs",
        |input, output, params| {
            with_views(input, output, |source, output| {
                let mut prepared = PreparedTrilinear::<Rgba8>::try_new(
                    source.dimensions(),
                    output.dimensions(),
                    prod_bilinear_anchor(params),
                    u64::MAX,
                )?;
                prepared.execute(source, output)
            })?
            .map_err(|failure| {
                BenchSubjectError::new(format!("{:?} at {:?}", failure.code, failure.path))
            })
        },
        Some("spec:resize:trilinear:mip-area"),
    )
}
