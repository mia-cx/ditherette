use ditherette_wasm::{
    image::ImageDimensions,
    prod::tiling::{for_each_row_band, for_each_tile, RowBand, RowBandPlan, Tile, TileGrid},
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
    let dimensions = ImageDimensions::new(8, 6).unwrap();

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

#[test]
fn row_band_plan_can_split_by_target_height() {
    let dimensions = ImageDimensions::new(8, 7).unwrap();
    let plan = RowBandPlan::for_output_height(dimensions, 3).unwrap();

    assert_eq!(
        plan.bands(),
        &[
            RowBand::new(0, 3).unwrap(),
            RowBand::new(3, 6).unwrap(),
            RowBand::new(6, 7).unwrap(),
        ]
    );
    assert!(RowBandPlan::for_output_height(dimensions, 0).is_none());
}

#[test]
fn tile_is_half_open_and_non_empty() {
    let tile = Tile::new(2, 3, 7, 11).unwrap();

    assert_eq!(tile.x_start(), 2);
    assert_eq!(tile.y_start(), 3);
    assert_eq!(tile.x_end(), 7);
    assert_eq!(tile.y_end(), 11);
    assert_eq!(tile.width(), 5);
    assert_eq!(tile.height(), 8);
    assert_eq!(Tile::new(2, 3, 2, 11), None);
    assert_eq!(Tile::new(2, 3, 7, 3), None);
}

#[test]
fn tile_grid_covers_output_in_row_major_tiles() {
    let dimensions = ImageDimensions::new(5, 4).unwrap();
    let grid = TileGrid::new(dimensions, 2, 3).unwrap();

    assert_eq!(grid.output_dimensions(), dimensions);
    assert_eq!(
        grid.tiles(),
        &[
            Tile::new(0, 0, 2, 3).unwrap(),
            Tile::new(2, 0, 4, 3).unwrap(),
            Tile::new(4, 0, 5, 3).unwrap(),
            Tile::new(0, 3, 2, 4).unwrap(),
            Tile::new(2, 3, 4, 4).unwrap(),
            Tile::new(4, 3, 5, 4).unwrap(),
        ]
    );
    assert!(TileGrid::new(dimensions, 0, 3).is_none());
    assert!(TileGrid::new(dimensions, 2, 0).is_none());
}

#[test]
fn executors_visit_plans_in_order_and_propagate_errors() {
    let dimensions = ImageDimensions::new(5, 4).unwrap();
    let bands = RowBandPlan::for_output_height(dimensions, 2).unwrap();
    let mut visited_bands = Vec::new();
    for_each_row_band::<() /* error */>(&bands, |band| {
        visited_bands.push(band);
        Ok(())
    })
    .unwrap();
    assert_eq!(visited_bands, bands.bands());

    let grid = TileGrid::new(dimensions, 3, 2).unwrap();
    let mut visited_tiles = Vec::new();
    let result = for_each_tile(&grid, |tile| {
        visited_tiles.push(tile);
        (visited_tiles.len() < 3).then_some(()).ok_or("stop")
    });

    assert_eq!(result, Err("stop"));
    assert_eq!(&visited_tiles, &grid.tiles()[..3]);
}
