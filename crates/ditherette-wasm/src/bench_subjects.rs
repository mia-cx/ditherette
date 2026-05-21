//! Benchmark subject registration for the fresh Ditherette core.
//!
//! This module is compiled only with the `bench-subjects` feature. It keeps
//! benchmark adapters in the implementation crate so `ditherette-bench` can
//! consume stable subject descriptors without deep-importing internal modules.

use std::cell::RefCell;

use ditherette_bench_api::{
    BenchSubject, BenchSubjectError, ParamSchema, PixelFormat, ResizeBenchSubject,
    ResizeInputU8Rgba, ResizeOutputU8Rgba, ResizeParams, ResizeU8RgbaFn, SubjectCapabilities,
    SubjectDescriptor, SubjectId, SupportPolicyParam,
};

use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8, RowStride},
    prod::resize::scalar::{
        area::resize_area_rgba8_into as resize_prod_area_rgba8_into,
        nearest::{
            alignment::ResizeAnchor as ProdResizeAnchor,
            resize_nearest_rgba8_with_plan_into as resize_prod_nearest_rgba8_with_plan_into,
            NearestResizePlan,
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
    vec![
        resize_subject(
            "spec:resize:nearest:scalar",
            "spec nearest scalar",
            "crates/ditherette-wasm/src/spec/resize/scalar/nearest.rs",
            resize_nearest_subject,
        ),
        resize_subject(
            "prod:resize:nearest:scalar",
            "prod nearest scalar",
            "crates/ditherette-wasm/src/prod/resize/scalar/nearest.rs",
            resize_prod_nearest_subject,
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
            "crates/ditherette-wasm/src/prod/resize/scalar/area.rs",
            resize_prod_area_subject,
        ),
        resize_subject(
            "spec:resize:bilinear:scalar",
            "spec bilinear scalar",
            "crates/ditherette-wasm/src/spec/resize/scalar/bilinear.rs",
            resize_bilinear_subject,
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
        resize_subject(
            "spec:resize:trilinear:mip-area",
            "spec trilinear area mip pyramid",
            "crates/ditherette-wasm/src/spec/resize/scalar/trilinear.rs",
            resize_trilinear_subject,
        ),
    ]
}

fn resize_subject(
    id: &str,
    display_name: &str,
    source_file: &str,
    resize_u8_rgba: ResizeU8RgbaFn,
) -> BenchSubject {
    let id = SubjectId::parse(id).expect("hard-coded subject ID should be valid");
    BenchSubject::Resize(ResizeBenchSubject {
        descriptor: SubjectDescriptor {
            id: id.clone(),
            display_name: display_name.to_owned(),
            source_file: source_file.to_owned(),
            source_line: 1,
            default_oracle: if id.module() == "spec" {
                None
            } else {
                Some(
                    SubjectId::parse(format!("spec:{}:{}:scalar", id.domain(), id.filter()))
                        .expect("constructed default oracle should be valid"),
                )
            },
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

thread_local! {
    static PROD_NEAREST_PLAN: RefCell<Option<NearestResizePlan>> = const { RefCell::new(None) };
}

fn resize_prod_nearest_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        let anchor = prod_anchor(params);
        PROD_NEAREST_PLAN.with_borrow_mut(|cached| {
            if !cached
                .as_ref()
                .is_some_and(|plan| plan.matches(source.dimensions(), output.dimensions(), anchor))
            {
                *cached = Some(NearestResizePlan::new(
                    source.dimensions(),
                    output.dimensions(),
                    anchor,
                ));
            }

            let plan = cached.as_ref().expect("nearest plan should be initialized");
            resize_prod_nearest_rgba8_with_plan_into(source, output, plan);
        });
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

fn resize_trilinear_subject(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, output| {
        resize_trilinear_into(source, output, anchor(params));
    })
}

fn with_views(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    resize: impl FnOnce(ImageView<'_, Rgba8>, ImageViewMut<'_, Rgba8>),
) -> Result<(), BenchSubjectError> {
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

    resize(source, output);
    Ok(())
}

fn prod_anchor(params: &ResizeParams) -> ProdResizeAnchor {
    match params.anchor {
        ditherette_bench_api::ResizeAnchorParam::TopLeft => ProdResizeAnchor::TopLeft,
        ditherette_bench_api::ResizeAnchorParam::Top => ProdResizeAnchor::Top,
        ditherette_bench_api::ResizeAnchorParam::TopRight => ProdResizeAnchor::TopRight,
        ditherette_bench_api::ResizeAnchorParam::Left => ProdResizeAnchor::Left,
        ditherette_bench_api::ResizeAnchorParam::Center => ProdResizeAnchor::Center,
        ditherette_bench_api::ResizeAnchorParam::Right => ProdResizeAnchor::Right,
        ditherette_bench_api::ResizeAnchorParam::BottomLeft => ProdResizeAnchor::BottomLeft,
        ditherette_bench_api::ResizeAnchorParam::Bottom => ProdResizeAnchor::Bottom,
        ditherette_bench_api::ResizeAnchorParam::BottomRight => ProdResizeAnchor::BottomRight,
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
