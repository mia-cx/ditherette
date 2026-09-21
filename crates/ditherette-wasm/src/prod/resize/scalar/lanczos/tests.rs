use super::super::convolution::ReconstructionKernel;
use super::*;
use crate::prod::contract::{error::ErrorCode, failure::ErrorPath};

struct CustomRadiusThree;
impl ReconstructionKernel for CustomRadiusThree {
    fn radius(&self) -> f64 {
        3.0
    }
    fn weight(&self, distance: f64) -> f64 {
        (3.0 - distance.abs()).max(0.0)
    }
}

#[test]
fn fixed_separable_dispatch_is_lanczos3_only_and_bounded_on_both_axes() {
    for (sw, sh, ow, oh, separable) in [
        (16, 14, 8, 7, true),
        (17, 13, 12, 10, true),
        (17, 14, 8, 7, false),
        (16, 15, 8, 7, false),
        (16, 14, 16, 7, false),
        (16, 14, 8, 14, false),
        (16, 14, 17, 15, false),
    ] {
        let source = ImageDimensions::new(sw, sh).unwrap();
        let output = ImageDimensions::new(ow, oh).unwrap();
        let expected = if separable {
            sh as usize * ow as usize * 4
        } else {
            0
        };
        let mut budget = CapacityBudget::new(u64::MAX);
        let fixed = LanczosResizePlan::try_new3(
            source,
            output,
            ResizeAnchor::Center,
            SupportPolicy::Fixed,
            &mut budget,
        )
        .unwrap();
        let dynamic = LanczosResizePlan::new(
            source,
            output,
            ResizeAnchor::Center,
            NonZeroU32::new(3).unwrap(),
            SupportPolicy::Fixed,
        );
        assert_eq!(fixed.scratch_elements().unwrap(), expected);
        assert_eq!(dynamic.scratch_elements().unwrap(), expected);
        let custom = ConvolutionResizePlan::new(
            source,
            output,
            ResizeAnchor::Center,
            &CustomRadiusThree,
            SupportPolicy::Fixed,
        );
        assert_eq!(custom.scratch_elements().unwrap(), 0);
        let lanczos2 = LanczosResizePlan::new(
            source,
            output,
            ResizeAnchor::Center,
            NonZeroU32::new(2).unwrap(),
            SupportPolicy::Fixed,
        );
        assert_eq!(lanczos2.scratch_elements().unwrap(), 0);
    }
}

#[test]
fn fixed_separable_preflights_scratch_and_reports_both_passes() {
    let source_dimensions = ImageDimensions::new(16, 14).unwrap();
    let output_dimensions = ImageDimensions::new(8, 7).unwrap();
    let required = LanczosResizePlan::required_bytes(
        source_dimensions,
        output_dimensions,
        NonZeroU32::new(3).unwrap(),
        SupportPolicy::Fixed,
    )
    .unwrap();
    let mut short_budget = CapacityBudget::new(required - 1);
    assert_eq!(
        LanczosResizePlan::try_new3(
            source_dimensions,
            output_dimensions,
            ResizeAnchor::Center,
            SupportPolicy::Fixed,
            &mut short_budget
        )
        .err()
        .unwrap()
        .code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(short_budget.used(), 0);
    let mut budget = CapacityBudget::new(required);
    let plan = LanczosResizePlan::try_new3(
        source_dimensions,
        output_dimensions,
        ResizeAnchor::Center,
        SupportPolicy::Fixed,
        &mut budget,
    )
    .unwrap();
    let len = plan.scratch_elements().unwrap();
    let mut scratch = budget.vector::<f64>(len).unwrap();
    scratch.resize(len, 0.0);
    let bytes = vec![127; source_dimensions.storage_len::<Rgba8>().unwrap()];
    let source = ImageView::packed(&bytes, source_dimensions).unwrap();
    let mut output = vec![213; output_dimensions.storage_len::<Rgba8>().unwrap()];
    let error = resize_lanczos_with_progress(
        source,
        ImageViewMut::packed(&mut output, output_dimensions).unwrap(),
        &plan,
        &mut scratch[..len - 1],
        &mut |_, _| panic!("preflight must precede progress"),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::MemoryLimit);
    assert!(output.iter().all(|&byte| byte == 213));
    let mut events = Vec::new();
    resize_lanczos_with_progress(
        source,
        ImageViewMut::packed(&mut output, output_dimensions).unwrap(),
        &plan,
        &mut scratch,
        &mut |done, total| {
            events.push((done, total));
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(events, (0..=21).map(|done| (done, 21)).collect::<Vec<_>>());
    output.fill(213);
    let error = resize_lanczos_with_progress(
        source,
        ImageViewMut::packed(&mut output, output_dimensions).unwrap(),
        &plan,
        &mut scratch,
        &mut |done, _| {
            if done == 1 {
                return Err(Failure::new(ErrorCode::Callback, ErrorPath::OnProgress));
            }
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::Callback);
    assert!(output.iter().all(|&byte| byte == 213));
}
