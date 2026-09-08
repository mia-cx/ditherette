//! Benchmark subject registration for the fresh Ditherette core.
//!
//! This module is compiled only with the `bench-subjects` feature. It keeps
//! benchmark adapters in the implementation crate so `ditherette-bench` can
//! consume stable subject descriptors without deep-importing internal modules.

pub mod field_calls;
pub mod fields;
pub mod quantize;
pub mod reference;
mod resize_budgeted;
pub mod scores;
mod trilinear;
pub mod verification;

/// Existing registry with this crate's concrete, borrowed conformance protocol.
pub type BenchSubject = ditherette_bench_api::BenchSubject<reference::ReferenceFn>;

use ditherette_bench_api::{
    BenchSubjectError, ParamSchema, PixelFormat, ResizeBenchSubject, ResizeInputU8Rgba,
    ResizeOutputU8Rgba, ResizeParams, ResizeU8RgbaFn, SubjectCapabilities, SubjectDescriptor,
    SubjectId, SupportPolicyParam,
};

use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8, RowStride},
    prod::resize::scalar::{
        area::resize_area_rgba8_into as resize_prod_area_rgba8_into,
        bicubic::resize_bicubic_rgba8_into as resize_prod_bicubic_rgba8_into,
        bilinear::{
            alignment::ResizeAnchor as ProdBilinearResizeAnchor,
            resize_bilinear_rgba8_into as resize_prod_bilinear_rgba8_into,
        },
        convolution::{
            ResizeAnchor as ProdConvolutionResizeAnchor,
            SupportPolicy as ProdConvolutionSupportPolicy,
        },
        lanczos::{
            resize_lanczos2_rgba8_into as resize_prod_lanczos2_rgba8_into,
            resize_lanczos3_rgba8_into as resize_prod_lanczos3_rgba8_into,
        },
        nearest::{
            alignment::ResizeAnchor as CandidateNearestResizeAnchor,
            resize_nearest_rgba8_into as resize_candidate_nearest_rgba8_into,
        },
    },
    spec::resize::{
        common::alignment::ResizeAnchor,
        scalar::{
            area::resize_area_into,
            bicubic::resize_bicubic_into,
            bilinear::resize_bilinear_into,
            convolution::SupportPolicy,
            lanczos::{resize_lanczos2_into, resize_lanczos3_into},
            nearest::resize_nearest_into,
            trilinear::resize_trilinear_into,
        },
    },
};

/// Returns all benchmark subjects exposed by this crate.
pub fn bench_subjects() -> Vec<BenchSubject> {
    let mut subjects = vec![
        trilinear::subject(),
        resize_subject(
            "spec:resize:nearest:scalar",
            "spec nearest scalar",
            "crates/ditherette-wasm/src/spec/resize/scalar/nearest.rs",
            resize_nearest_subject,
        ),
        resize_subject(
            "prod:resize:nearest:scalar",
            "landed packed nearest scalar",
            "crates/ditherette-wasm/src/prod/resize/scalar/nearest/mod.rs",
            resize_candidate_nearest_subject,
        ),
        resize_subject(
            "candidate:resize:nearest:legacy",
            "landed packed nearest (historical legacy ID)",
            "crates/ditherette-wasm/src/prod/resize/scalar/nearest/mod.rs",
            resize_candidate_nearest_subject,
        ),
        resize_subject(
            "spec:resize:area:scalar",
            "spec area scalar",
            "crates/ditherette-wasm/src/spec/resize/scalar/area.rs",
            resize_area_subject,
        ),
        resize_subject(
            "prod:resize:area:scalar",
            "prod area scalar",
            "crates/ditherette-wasm/src/prod/resize/scalar/area/mod.rs",
            resize_prod_area_subject,
        ),
        resize_subject(
            "spec:resize:bilinear:scalar",
            "spec bilinear scalar",
            "crates/ditherette-wasm/src/spec/resize/scalar/bilinear.rs",
            resize_bilinear_subject,
        ),
        resize_subject(
            "prod:resize:bilinear:scalar",
            "prod bilinear scalar",
            "crates/ditherette-wasm/src/prod/resize/scalar/bilinear/mod.rs",
            resize_prod_bilinear_subject,
        ),
        resize_subject(
            "spec:resize:bicubic:catmull-rom",
            "spec bicubic Catmull-Rom",
            "crates/ditherette-wasm/src/spec/resize/scalar/bicubic.rs",
            resize_bicubic_fixed_subject,
        ),
        resize_subject(
            "spec:resize:bicubic:catmull-rom-scale-aware",
            "spec bicubic Catmull-Rom scale-aware",
            "crates/ditherette-wasm/src/spec/resize/scalar/bicubic.rs",
            resize_bicubic_scale_aware_subject,
        ),
        resize_subject_with_oracle(
            "prod:resize:bicubic:catmull-rom",
            "prod bicubic Catmull-Rom",
            "crates/ditherette-wasm/src/prod/resize/scalar/bicubic/mod.rs",
            resize_prod_bicubic_fixed_subject,
            Some("spec:resize:bicubic:catmull-rom"),
        ),
        resize_subject_with_oracle(
            "prod:resize:bicubic:catmull-rom-scale-aware",
            "prod bicubic Catmull-Rom scale-aware",
            "crates/ditherette-wasm/src/prod/resize/scalar/bicubic/mod.rs",
            resize_prod_bicubic_scale_aware_subject,
            Some("spec:resize:bicubic:catmull-rom-scale-aware"),
        ),
        resize_subject(
            "spec:resize:lanczos2:fixed",
            "spec Lanczos2 fixed support",
            "crates/ditherette-wasm/src/spec/resize/scalar/lanczos.rs",
            resize_lanczos2_fixed_subject,
        ),
        resize_subject(
            "spec:resize:lanczos2:scale-aware",
            "spec Lanczos2 scale-aware",
            "crates/ditherette-wasm/src/spec/resize/scalar/lanczos.rs",
            resize_lanczos2_scale_aware_subject,
        ),
        resize_subject_with_oracle(
            "prod:resize:lanczos2:fixed",
            "prod Lanczos2 fixed support",
            "crates/ditherette-wasm/src/prod/resize/scalar/lanczos/mod.rs",
            resize_prod_lanczos2_fixed_subject,
            Some("spec:resize:lanczos2:fixed"),
        ),
        resize_subject_with_oracle(
            "prod:resize:lanczos2:scale-aware",
            "prod Lanczos2 scale-aware",
            "crates/ditherette-wasm/src/prod/resize/scalar/lanczos/mod.rs",
            resize_prod_lanczos2_scale_aware_subject,
            Some("spec:resize:lanczos2:scale-aware"),
        ),
        resize_subject(
            "spec:resize:lanczos3:fixed",
            "spec Lanczos3 fixed support",
            "crates/ditherette-wasm/src/spec/resize/scalar/lanczos.rs",
            resize_lanczos3_fixed_subject,
        ),
        resize_subject(
            "spec:resize:lanczos3:scale-aware",
            "spec Lanczos3 scale-aware",
            "crates/ditherette-wasm/src/spec/resize/scalar/lanczos.rs",
            resize_lanczos3_scale_aware_subject,
        ),
        resize_subject_with_oracle(
            "prod:resize:lanczos3:fixed",
            "prod Lanczos3 fixed support",
            "crates/ditherette-wasm/src/prod/resize/scalar/lanczos/mod.rs",
            resize_prod_lanczos3_fixed_subject,
            Some("spec:resize:lanczos3:fixed"),
        ),
        resize_subject_with_oracle(
            "prod:resize:lanczos3:scale-aware",
            "prod Lanczos3 scale-aware",
            "crates/ditherette-wasm/src/prod/resize/scalar/lanczos/mod.rs",
            resize_prod_lanczos3_scale_aware_subject,
            Some("spec:resize:lanczos3:scale-aware"),
        ),
        resize_subject(
            "spec:resize:trilinear:mip-area",
            "spec trilinear area mip pyramid",
            "crates/ditherette-wasm/src/spec/resize/scalar/trilinear.rs",
            resize_trilinear_subject,
        ),
    ];
    subjects.extend(resize_budgeted::subjects());
    subjects.extend(reference::subjects());
    subjects.extend(quantize::subjects());
    subjects.extend(scores::subjects());
    subjects.extend(fields::subjects());
    subjects.extend(field_calls::subjects());
    subjects
}

fn resize_subject(
    id: &str,
    display_name: &str,
    source_file: &str,
    resize_u8_rgba: ResizeU8RgbaFn,
) -> BenchSubject {
    let id = SubjectId::parse(id).expect("hard-coded subject ID should be valid");
    let default_oracle = if id.module() == "spec" {
        None
    } else {
        Some(
            SubjectId::parse(format!("spec:{}:{}:scalar", id.domain(), id.filter()))
                .expect("constructed default oracle should be valid"),
        )
    };
    resize_subject_from_id(
        id,
        display_name,
        source_file,
        resize_u8_rgba,
        default_oracle,
    )
}

fn resize_subject_with_oracle(
    id: &str,
    display_name: &str,
    source_file: &str,
    resize_u8_rgba: ResizeU8RgbaFn,
    default_oracle: Option<&str>,
) -> BenchSubject {
    let id = SubjectId::parse(id).expect("hard-coded subject ID should be valid");
    let default_oracle = default_oracle.map(|oracle| {
        SubjectId::parse(oracle).expect("hard-coded oracle subject ID should be valid")
    });
    resize_subject_from_id(
        id,
        display_name,
        source_file,
        resize_u8_rgba,
        default_oracle,
    )
}

fn resize_subject_from_id(
    id: SubjectId,
    display_name: &str,
    source_file: &str,
    resize_u8_rgba: ResizeU8RgbaFn,
    default_oracle: Option<SubjectId>,
) -> BenchSubject {
    BenchSubject::Resize(ResizeBenchSubject {
        descriptor: SubjectDescriptor {
            id,
            display_name: display_name.to_owned(),
            source_file: source_file.to_owned(),
            source_line: 1,
            default_oracle,
            capabilities: SubjectCapabilities {
                pixel_formats: vec![PixelFormat::Rgba8],
                ..SubjectCapabilities::rgba8_packed()
            },
            params_schema: ParamSchema::default(),
        },
        resize_u8_rgba,
    })
}

fn resize_nearest_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_nearest_into(source, output, anchor(params));
    })
}

fn resize_candidate_nearest_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_candidate_nearest_rgba8_into(source, output, candidate_nearest_anchor(params));
    })
}

fn resize_area_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    _params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, resize_area_into::<Rgba8>)
}

fn resize_prod_area_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    _params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, resize_prod_area_rgba8_into)
}

fn resize_bilinear_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_bilinear_into(source, output, anchor(params));
    })
}

fn resize_prod_bilinear_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_prod_bilinear_rgba8_into(source, output, prod_bilinear_anchor(params));
    })
}

fn resize_bicubic_fixed_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_bicubic_into(source, output, anchor(params), SupportPolicy::Fixed);
    })
}

fn resize_bicubic_scale_aware_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_bicubic_into(source, output, anchor(params), SupportPolicy::ScaleAware);
    })
}

fn resize_prod_bicubic_fixed_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_prod_bicubic_rgba8_into(
            source,
            output,
            prod_convolution_anchor(params),
            ProdConvolutionSupportPolicy::Fixed,
        );
    })
}

fn resize_prod_bicubic_scale_aware_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_prod_bicubic_rgba8_into(
            source,
            output,
            prod_convolution_anchor(params),
            ProdConvolutionSupportPolicy::ScaleAware,
        );
    })
}

fn resize_lanczos2_fixed_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_lanczos2_into(source, output, anchor(params), SupportPolicy::Fixed);
    })
}

fn resize_lanczos2_scale_aware_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_lanczos2_into(source, output, anchor(params), SupportPolicy::ScaleAware);
    })
}

fn resize_prod_lanczos2_fixed_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_prod_lanczos2_rgba8_into(
            source,
            output,
            prod_convolution_anchor(params),
            ProdConvolutionSupportPolicy::Fixed,
        );
    })
}

fn resize_prod_lanczos2_scale_aware_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_prod_lanczos2_rgba8_into(
            source,
            output,
            prod_convolution_anchor(params),
            ProdConvolutionSupportPolicy::ScaleAware,
        );
    })
}

fn resize_lanczos3_fixed_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_lanczos3_into(source, output, anchor(params), SupportPolicy::Fixed);
    })
}

fn resize_lanczos3_scale_aware_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_lanczos3_into(source, output, anchor(params), SupportPolicy::ScaleAware);
    })
}

fn resize_prod_lanczos3_fixed_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_prod_lanczos3_rgba8_into(
            source,
            output,
            prod_convolution_anchor(params),
            ProdConvolutionSupportPolicy::Fixed,
        );
    })
}

fn resize_prod_lanczos3_scale_aware_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_prod_lanczos3_rgba8_into(
            source,
            output,
            prod_convolution_anchor(params),
            ProdConvolutionSupportPolicy::ScaleAware,
        );
    })
}

fn resize_trilinear_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_trilinear_into(source, output, anchor(params));
    })
}

fn with_views<T>(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    resize: impl FnOnce(ImageView<'_, Rgba8>, ImageViewMut<'_, Rgba8>) -> T,
) -> Result<T, BenchSubjectError> {
    let input_dimensions = ImageDimensions::new(input.width, input.height)
        .map_err(|error| BenchSubjectError::new(error.to_string()))?;
    let output_dimensions = ImageDimensions::new(output.width, output.height)
        .map_err(|error| BenchSubjectError::new(error.to_string()))?;
    let source = ImageView::<Rgba8>::new(
        input.data,
        input_dimensions,
        RowStride::new(input.row_stride_elements)
            .map_err(|error| BenchSubjectError::new(error.to_string()))?,
    )
    .map_err(|error| BenchSubjectError::new(error.to_string()))?;
    let output = ImageViewMut::<Rgba8>::new(
        output.data,
        output_dimensions,
        RowStride::new(output.row_stride_elements)
            .map_err(|error| BenchSubjectError::new(error.to_string()))?,
    )
    .map_err(|error| BenchSubjectError::new(error.to_string()))?;

    Ok(resize(source, output))
}

fn candidate_nearest_anchor(params: &ResizeParams) -> CandidateNearestResizeAnchor {
    match params.anchor {
        ditherette_bench_api::ResizeAnchorParam::TopLeft => CandidateNearestResizeAnchor::TopLeft,
        ditherette_bench_api::ResizeAnchorParam::Top => CandidateNearestResizeAnchor::Top,
        ditherette_bench_api::ResizeAnchorParam::TopRight => CandidateNearestResizeAnchor::TopRight,
        ditherette_bench_api::ResizeAnchorParam::Left => CandidateNearestResizeAnchor::Left,
        ditherette_bench_api::ResizeAnchorParam::Center => CandidateNearestResizeAnchor::Center,
        ditherette_bench_api::ResizeAnchorParam::Right => CandidateNearestResizeAnchor::Right,
        ditherette_bench_api::ResizeAnchorParam::BottomLeft => {
            CandidateNearestResizeAnchor::BottomLeft
        }
        ditherette_bench_api::ResizeAnchorParam::Bottom => CandidateNearestResizeAnchor::Bottom,
        ditherette_bench_api::ResizeAnchorParam::BottomRight => {
            CandidateNearestResizeAnchor::BottomRight
        }
    }
}

fn prod_bilinear_anchor(params: &ResizeParams) -> ProdBilinearResizeAnchor {
    match params.anchor {
        ditherette_bench_api::ResizeAnchorParam::TopLeft => ProdBilinearResizeAnchor::TopLeft,
        ditherette_bench_api::ResizeAnchorParam::Top => ProdBilinearResizeAnchor::Top,
        ditherette_bench_api::ResizeAnchorParam::TopRight => ProdBilinearResizeAnchor::TopRight,
        ditherette_bench_api::ResizeAnchorParam::Left => ProdBilinearResizeAnchor::Left,
        ditherette_bench_api::ResizeAnchorParam::Center => ProdBilinearResizeAnchor::Center,
        ditherette_bench_api::ResizeAnchorParam::Right => ProdBilinearResizeAnchor::Right,
        ditherette_bench_api::ResizeAnchorParam::BottomLeft => ProdBilinearResizeAnchor::BottomLeft,
        ditherette_bench_api::ResizeAnchorParam::Bottom => ProdBilinearResizeAnchor::Bottom,
        ditherette_bench_api::ResizeAnchorParam::BottomRight => {
            ProdBilinearResizeAnchor::BottomRight
        }
    }
}

fn prod_convolution_anchor(params: &ResizeParams) -> ProdConvolutionResizeAnchor {
    match params.anchor {
        ditherette_bench_api::ResizeAnchorParam::TopLeft => ProdConvolutionResizeAnchor::TopLeft,
        ditherette_bench_api::ResizeAnchorParam::Top => ProdConvolutionResizeAnchor::Top,
        ditherette_bench_api::ResizeAnchorParam::TopRight => ProdConvolutionResizeAnchor::TopRight,
        ditherette_bench_api::ResizeAnchorParam::Left => ProdConvolutionResizeAnchor::Left,
        ditherette_bench_api::ResizeAnchorParam::Center => ProdConvolutionResizeAnchor::Center,
        ditherette_bench_api::ResizeAnchorParam::Right => ProdConvolutionResizeAnchor::Right,
        ditherette_bench_api::ResizeAnchorParam::BottomLeft => {
            ProdConvolutionResizeAnchor::BottomLeft
        }
        ditherette_bench_api::ResizeAnchorParam::Bottom => ProdConvolutionResizeAnchor::Bottom,
        ditherette_bench_api::ResizeAnchorParam::BottomRight => {
            ProdConvolutionResizeAnchor::BottomRight
        }
    }
}

fn anchor(params: &ResizeParams) -> ResizeAnchor {
    match params.anchor {
        ditherette_bench_api::ResizeAnchorParam::TopLeft => ResizeAnchor::TopLeft,
        ditherette_bench_api::ResizeAnchorParam::Top => ResizeAnchor::Top,
        ditherette_bench_api::ResizeAnchorParam::TopRight => ResizeAnchor::TopRight,
        ditherette_bench_api::ResizeAnchorParam::Left => ResizeAnchor::Left,
        ditherette_bench_api::ResizeAnchorParam::Center => ResizeAnchor::Center,
        ditherette_bench_api::ResizeAnchorParam::Right => ResizeAnchor::Right,
        ditherette_bench_api::ResizeAnchorParam::BottomLeft => ResizeAnchor::BottomLeft,
        ditherette_bench_api::ResizeAnchorParam::Bottom => ResizeAnchor::Bottom,
        ditherette_bench_api::ResizeAnchorParam::BottomRight => ResizeAnchor::BottomRight,
    }
}

#[allow(dead_code)]
fn support_policy(params: &ResizeParams, default: SupportPolicy) -> SupportPolicy {
    match params.support_policy {
        Some(SupportPolicyParam::Fixed) => SupportPolicy::Fixed,
        Some(SupportPolicyParam::ScaleAware) => SupportPolicy::ScaleAware,
        None => default,
    }
}
