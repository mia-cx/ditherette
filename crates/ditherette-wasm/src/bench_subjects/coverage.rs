//! Coverage-stage subjects. Output allocation and downstream dithering are excluded.

use super::*;
use crate::{
    prod::contract::request::{
        Anchor as ProdAnchor, ResizePolicy as ProdPolicy, Support as ProdSupport,
    },
    spec::contract::request::{Anchor, Output, ResizePolicy, ResizeRequest, Source, Support},
};

pub(super) fn subjects() -> Vec<BenchSubject> {
    vec![
        resize_subject_with_oracle(
            "spec:coverage:bilinear:scalar",
            "spec coverage bilinear",
            "crates/ditherette-wasm/src/spec/coverage/resize.rs",
            spec_bilinear,
            None,
        ),
        resize_subject_with_oracle(
            "prod:coverage:bilinear:scalar",
            "production coverage bilinear",
            "crates/ditherette-wasm/src/prod/coverage/resize.rs",
            prod_bilinear,
            Some("spec:coverage:bilinear:scalar"),
        ),
        resize_subject_with_oracle(
            "spec:coverage:lanczos3:scale-aware",
            "spec coverage Lanczos3 scale-aware",
            "crates/ditherette-wasm/src/spec/coverage/resize.rs",
            spec_lanczos3,
            None,
        ),
        resize_subject_with_oracle(
            "prod:coverage:lanczos3:scale-aware",
            "production coverage Lanczos3 scale-aware",
            "crates/ditherette-wasm/src/prod/coverage/resize.rs",
            prod_lanczos3,
            Some("spec:coverage:lanczos3:scale-aware"),
        ),
    ]
}

fn spec_bilinear(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    spec_resize(
        input,
        output,
        ResizePolicy::Bilinear {
            anchor: spec_anchor(params),
        },
    )
}

fn spec_lanczos3(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    spec_resize(
        input,
        output,
        ResizePolicy::Lanczos3 {
            anchor: spec_anchor(params),
            support: Support::ScaleAware,
        },
    )
}

fn spec_resize(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    policy: ResizePolicy,
) -> Result<(), BenchSubjectError> {
    let resized = crate::spec::coverage::resize(ResizeRequest {
        version: 1,
        source: Source {
            width: input.width,
            height: input.height,
            data: input.data,
        },
        output: Output {
            width: output.width,
            height: output.height,
            resize: policy,
        },
    })
    .map_err(|error| BenchSubjectError::new(error.to_string()))?;
    output.data.copy_from_slice(resized.data());
    Ok(())
}

fn prod_bilinear(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    prod_resize(
        input,
        output,
        ProdPolicy::Bilinear {
            anchor: prod_request_anchor(params),
        },
    )
}

fn prod_lanczos3(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    prod_resize(
        input,
        output,
        ProdPolicy::Lanczos3 {
            anchor: prod_request_anchor(params),
            support: ProdSupport::ScaleAware,
        },
    )
}

fn prod_resize(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    policy: ProdPolicy,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        crate::prod::coverage::resize(source, output, policy, u64::MAX, &mut |_, _| Ok(()))
    })?
    .map_err(|failure| BenchSubjectError::new(format!("{:?} at {:?}", failure.code, failure.path)))
}

fn spec_anchor(params: &ResizeParams) -> Anchor {
    match prod_request_anchor(params) {
        ProdAnchor::TopLeft => Anchor::TopLeft,
        ProdAnchor::Top => Anchor::Top,
        ProdAnchor::TopRight => Anchor::TopRight,
        ProdAnchor::Left => Anchor::Left,
        ProdAnchor::Center => Anchor::Center,
        ProdAnchor::Right => Anchor::Right,
        ProdAnchor::BottomLeft => Anchor::BottomLeft,
        ProdAnchor::Bottom => Anchor::Bottom,
        ProdAnchor::BottomRight => Anchor::BottomRight,
    }
}

fn prod_request_anchor(params: &ResizeParams) -> ProdAnchor {
    match params.anchor {
        ditherette_bench_api::ResizeAnchorParam::TopLeft => ProdAnchor::TopLeft,
        ditherette_bench_api::ResizeAnchorParam::Top => ProdAnchor::Top,
        ditherette_bench_api::ResizeAnchorParam::TopRight => ProdAnchor::TopRight,
        ditherette_bench_api::ResizeAnchorParam::Left => ProdAnchor::Left,
        ditherette_bench_api::ResizeAnchorParam::Center => ProdAnchor::Center,
        ditherette_bench_api::ResizeAnchorParam::Right => ProdAnchor::Right,
        ditherette_bench_api::ResizeAnchorParam::BottomLeft => ProdAnchor::BottomLeft,
        ditherette_bench_api::ResizeAnchorParam::Bottom => ProdAnchor::Bottom,
        ditherette_bench_api::ResizeAnchorParam::BottomRight => ProdAnchor::BottomRight,
    }
}
