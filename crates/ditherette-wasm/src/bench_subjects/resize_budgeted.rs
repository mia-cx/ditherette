//! Per-call fallible production preparation, followed by the landed resize kernel.
//! Native callers supply output storage. Plan/scratch allocation, filling and disposal
//! remain inside this call; these subjects do not measure the public JS boundary.

use super::*;
use crate::prod::{
    contract::failure::Failure,
    resize::{
        common::allocation::CapacityBudget,
        scalar::{area, bicubic, bilinear, convolution::SupportPolicy, lanczos},
    },
};

#[derive(Clone, Copy)]
enum Filter {
    Area,
    Bilinear,
    Bicubic,
    Lanczos2,
    Lanczos3,
}

pub(super) fn subjects() -> Vec<BenchSubject> {
    use Filter::*;
    use SupportPolicy::{Fixed, ScaleAware};
    let entries: [(&str, &str, ResizeU8RgbaFn); 8] = [
        ("area:budgeted", "area:scalar", |i, o, p| {
            run(i, o, p, Area, Fixed)
        }),
        ("bilinear:budgeted", "bilinear:scalar", |i, o, p| {
            run(i, o, p, Bilinear, Fixed)
        }),
        (
            "bicubic:budgeted-fixed",
            "bicubic:catmull-rom",
            |i, o, p| run(i, o, p, Bicubic, Fixed),
        ),
        (
            "bicubic:budgeted-scale-aware",
            "bicubic:catmull-rom-scale-aware",
            |i, o, p| run(i, o, p, Bicubic, ScaleAware),
        ),
        ("lanczos2:budgeted-fixed", "lanczos2:fixed", |i, o, p| {
            run(i, o, p, Lanczos2, Fixed)
        }),
        (
            "lanczos2:budgeted-scale-aware",
            "lanczos2:scale-aware",
            |i, o, p| run(i, o, p, Lanczos2, ScaleAware),
        ),
        ("lanczos3:budgeted-fixed", "lanczos3:fixed", |i, o, p| {
            run(i, o, p, Lanczos3, Fixed)
        }),
        (
            "lanczos3:budgeted-scale-aware",
            "lanczos3:scale-aware",
            |i, o, p| run(i, o, p, Lanczos3, ScaleAware),
        ),
    ];
    entries
        .into_iter()
        .map(|(id, oracle, call)| {
            resize_subject_with_oracle(
                &format!("candidate:resize:{id}"),
                &format!("landed {id} per-call preparation"),
                "crates/ditherette-wasm/src/bench_subjects/resize_budgeted.rs",
                call,
                Some(&format!("spec:resize:{oracle}")),
            )
        })
        .collect()
}

fn run(
    input: ResizeInputU8Rgba<'_>,
    output: ResizeOutputU8Rgba<'_>,
    params: &ResizeParams,
    filter: Filter,
    policy: SupportPolicy,
) -> Result<(), BenchSubjectError> {
    with_views(input, output, |source, mut output| -> Result<(), Failure> {
        // Match the one-shot production identity dispatch before constructing metadata.
        if source.dimensions() == output.dimensions() {
            output.data_mut().copy_from_slice(source.data());
            return Ok(());
        }
        let mut budget = CapacityBudget::new(u64::MAX);
        let source_dimensions = source.dimensions();
        let output_dimensions = output.dimensions();
        match filter {
            Filter::Area => {
                let plan = area::AreaResizePlan::try_new(
                    source_dimensions,
                    output_dimensions,
                    &mut budget,
                )?;
                let mut scratch = budget.vector(plan.scratch_elements())?;
                scratch.resize(plan.scratch_elements(), 0.0);
                area::resize_area_rgba8_with_plan_and_scratch_into(
                    source,
                    output,
                    &plan,
                    &mut scratch,
                );
            }
            Filter::Bilinear => {
                let plan = bilinear::BilinearResizePlan::try_new(
                    source_dimensions,
                    output_dimensions,
                    prod_bilinear_anchor(params),
                    &mut budget,
                )?;
                let mut scratch = budget.vector(plan.scratch_elements())?;
                scratch.resize(plan.scratch_elements(), 0.0);
                bilinear::resize_bilinear_rgba8_with_plan_and_scratch_into(
                    source,
                    output,
                    &plan,
                    &mut scratch,
                );
            }
            Filter::Bicubic => {
                let plan = bicubic::BicubicResizePlan::try_new(
                    source_dimensions,
                    output_dimensions,
                    prod_convolution_anchor(params),
                    policy,
                    &mut budget,
                )?;
                let elements = plan.scratch_elements()?;
                let mut scratch = budget.vector(elements)?;
                scratch.resize(elements, 0.0);
                bicubic::resize_bicubic_rgba8_with_plan_and_scratch_into(
                    source,
                    output,
                    &plan,
                    &mut scratch,
                )?;
            }
            Filter::Lanczos2 | Filter::Lanczos3 => {
                let prepare = match filter {
                    Filter::Lanczos2 => lanczos::LanczosResizePlan::try_new2,
                    _ => lanczos::LanczosResizePlan::try_new3,
                };
                let plan = prepare(
                    source_dimensions,
                    output_dimensions,
                    prod_convolution_anchor(params),
                    policy,
                    &mut budget,
                )?;
                let elements = plan.scratch_elements()?;
                let mut scratch = budget.vector(elements)?;
                scratch.resize(elements, 0.0);
                lanczos::resize_lanczos_rgba8_with_plan_and_scratch_into(
                    source,
                    output,
                    &plan,
                    &mut scratch,
                )?;
            }
        }
        Ok(())
    })?
    .map_err(|failure| BenchSubjectError::new(format!("{:?} at {:?}", failure.code, failure.path)))
}
