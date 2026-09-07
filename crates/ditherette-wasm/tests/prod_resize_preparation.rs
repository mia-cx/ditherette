use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::resize::{
        common::allocation::CapacityBudget,
        scalar::{
            area::{self, AreaResizePlan},
            bilinear::{self, alignment::ResizeAnchor, BilinearResizePlan},
        },
    },
};

#[test]
fn fallible_bilinear_preserves_landed_bytes_and_fits_its_preflight() {
    for (sw, sh, ow, oh) in [
        (7, 5, 3, 2),
        (3, 2, 7, 5),
        (3, 7, 3, 5),
        (7, 3, 5, 3),
        (1, 1, 1, 1),
    ] {
        let source = ImageDimensions::new(sw, sh).unwrap();
        let output = ImageDimensions::new(ow, oh).unwrap();
        let bytes: Vec<u8> = (0..sw * sh * 4).map(|x| (x * 73) as u8).collect();
        for anchor in [
            ResizeAnchor::TopLeft,
            ResizeAnchor::Top,
            ResizeAnchor::TopRight,
            ResizeAnchor::Left,
            ResizeAnchor::Center,
            ResizeAnchor::Right,
            ResizeAnchor::BottomLeft,
            ResizeAnchor::Bottom,
            ResizeAnchor::BottomRight,
        ] {
            let required = BilinearResizePlan::required_bytes(source, output, anchor).unwrap();
            let mut budget = CapacityBudget::new(required);
            let plan = BilinearResizePlan::try_new(source, output, anchor, &mut budget).unwrap();
            let mut scratch = budget.vector::<f32>(plan.scratch_elements()).unwrap();
            scratch.resize(plan.scratch_elements(), 0.0);
            assert_eq!(
                budget.used(),
                plan.capacity_bytes() + scratch.capacity() as u64 * 4
            );
            let mut expected = vec![0; (ow * oh * 4) as usize];
            let mut actual = expected.clone();
            bilinear::resize_bilinear_rgba8_into(
                ImageView::packed(&bytes, source).unwrap(),
                ImageViewMut::packed(&mut expected, output).unwrap(),
                anchor,
            );
            bilinear::resize_bilinear_rgba8_with_plan_and_scratch_into(
                ImageView::<Rgba8>::packed(&bytes, source).unwrap(),
                ImageViewMut::packed(&mut actual, output).unwrap(),
                &plan,
                &mut scratch,
            );
            assert_eq!(actual, expected, "{sw}x{sh}->{ow}x{oh} {anchor:?}");
        }
    }
}

#[test]
fn fallible_area_preserves_landed_fast_paths_and_fractional_bytes() {
    for (sw, sh, ow, oh) in [
        (8, 6, 4, 3),
        (2, 3, 8, 9),
        (7, 5, 3, 2),
        (3, 2, 7, 5),
        (3, 7, 3, 5),
        (7, 3, 5, 3),
        (1, 1, 1, 1),
    ] {
        let source = ImageDimensions::new(sw, sh).unwrap();
        let output = ImageDimensions::new(ow, oh).unwrap();
        let bytes: Vec<u8> = (0..sw * sh * 4).map(|x| (x * 73) as u8).collect();
        let mut budget =
            CapacityBudget::new(AreaResizePlan::required_bytes(source, output).unwrap());
        let plan = AreaResizePlan::try_new(source, output, &mut budget).unwrap();
        let mut scratch = budget.vector::<f32>(plan.scratch_elements()).unwrap();
        scratch.resize(plan.scratch_elements(), 0.0);
        assert_eq!(
            budget.used(),
            plan.capacity_bytes() + scratch.capacity() as u64 * 4
        );
        let mut expected = vec![0; (ow * oh * 4) as usize];
        let mut actual = expected.clone();
        area::resize_area_rgba8_into(
            ImageView::packed(&bytes, source).unwrap(),
            ImageViewMut::packed(&mut expected, output).unwrap(),
        );
        area::resize_area_rgba8_with_plan_and_scratch_into(
            ImageView::<Rgba8>::packed(&bytes, source).unwrap(),
            ImageViewMut::packed(&mut actual, output).unwrap(),
            &plan,
            &mut scratch,
        );
        assert_eq!(actual, expected, "{sw}x{sh}->{ow}x{oh}");
    }
}
