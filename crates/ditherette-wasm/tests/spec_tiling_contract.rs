use ditherette_wasm::{
    image::ImageDimensions,
    spec::tiling::contract::{RowBand, RowBandPlan},
};

#[test]
fn row_band_is_half_open_and_non_empty() {
    let band = RowBand::new(2, 5).unwrap();

    assert_eq!(band.y_start(), 2);
    assert_eq!(band.y_end(), 5);
    assert_eq!(band.height(), 3);
    assert_eq!(RowBand::new(2, 2), None);
}

#[test]
fn row_band_plan_requires_complete_contiguous_coverage() {
    let dimensions = ImageDimensions::new(4, 6).unwrap();
    let plan = RowBandPlan::new(
        dimensions,
        vec![
            RowBand::new(0, 2).unwrap(),
            RowBand::new(2, 4).unwrap(),
            RowBand::new(4, 6).unwrap(),
        ],
    )
    .unwrap();

    assert_eq!(plan.output_dimensions(), dimensions);
    assert_eq!(plan.bands().len(), 3);
}

#[test]
fn row_band_plan_rejects_gaps_overlaps_and_short_coverage() {
    let dimensions = ImageDimensions::new(4, 6).unwrap();

    assert!(RowBandPlan::new(
        dimensions,
        vec![RowBand::new(0, 2).unwrap(), RowBand::new(3, 6).unwrap()]
    )
    .is_none());
    assert!(RowBandPlan::new(
        dimensions,
        vec![RowBand::new(0, 3).unwrap(), RowBand::new(2, 6).unwrap()]
    )
    .is_none());
    assert!(RowBandPlan::new(dimensions, vec![RowBand::new(0, 5).unwrap()]).is_none());
}
