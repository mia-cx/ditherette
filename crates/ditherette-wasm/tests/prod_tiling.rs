use ditherette_wasm::{
    image::ImageDimensions,
    prod::tiling::{
        for_each_row_band, for_each_tile, RowBand, RowBandPlan, RowBandWorkPlan, Tile, TileGrid,
        WorkerBudget,
    },
};

#[test]
fn budgeted_work_matches_existing_assignment_and_reports_only_on_caller() {
    use ditherette_wasm::prod::{
        contract::error::ErrorCode, resize::common::allocation::CapacityBudget,
        tiling::execute_row_band_work,
    };
    let dimensions = ImageDimensions::new(3, 19).unwrap();
    let pool = WorkerBudget::new(4);
    let mut insufficient = CapacityBudget::new(0);
    assert_eq!(
        RowBandWorkPlan::try_for_output_height(dimensions, 3, pool, 3, &mut insufficient)
            .unwrap_err()
            .code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(insufficient.used(), 0);
    let mut budget = CapacityBudget::new(8192);
    let work = RowBandWorkPlan::try_for_output_height(dimensions, 3, pool, 3, &mut budget).unwrap();
    let bands = RowBandPlan::for_output_height(dimensions, 3).unwrap();
    assert_eq!(work, RowBandWorkPlan::new(&bands, pool, 3).unwrap());
    assert!(work.capacity_bytes() <= budget.used());
    let mut output = [0u8; 3 * 19];
    let mut scratch = [0usize; 3];
    let caller = std::thread::current().id();
    let mut reports = Vec::new();
    execute_row_band_work(
        &work,
        &mut output,
        3,
        &mut scratch,
        &|band, output, calls| {
            *calls += 1;
            for (local, row) in output.chunks_exact_mut(3).enumerate() {
                row.fill((band.y_start() as usize + local) as u8);
            }
            Ok::<u64, ()>(u64::from(band.height()))
        },
        &mut |completed| {
            assert_eq!(std::thread::current().id(), caller);
            reports.push(completed);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        output.as_slice(),
        (0u8..19).flat_map(|y| [y; 3]).collect::<Vec<_>>()
    );
    assert_eq!(scratch, [3, 2, 2]);
    assert_eq!(reports, [9, 16, 19]);
    scratch.fill(0);
    let error = execute_row_band_work(
        &work,
        &mut output,
        3,
        &mut scratch,
        &|band, _, calls| {
            *calls += 1;
            if band.y_start() == 9 {
                Err(9)
            } else {
                Ok(u64::from(band.height()))
            }
        },
        &mut |_| panic!("a failed batch cannot report success"),
    )
    .unwrap_err();
    assert_eq!(error, 9);
    assert_eq!(
        scratch,
        [1, 1, 1],
        "every worker in the failed batch joins before return; no later batch starts"
    );
}

#[test]
fn fallible_assignments_match_frozen_worker_caps_and_band_order() {
    use ditherette_wasm::{
        prod::resize::common::allocation::CapacityBudget, spec::tiling as frozen,
    };
    let dimensions = ImageDimensions::new(3, 19).unwrap();
    for height in [1, 2, 3, 7, 19] {
        for pool in [1, 4, 8] {
            for requested in [0, 1, 2, 4, 9] {
                let required = RowBandWorkPlan::required_bytes(
                    dimensions,
                    height,
                    WorkerBudget::new(pool),
                    requested,
                )
                .unwrap();
                let mut budget = CapacityBudget::new(required);
                let actual = RowBandWorkPlan::try_for_output_height(
                    dimensions,
                    height,
                    WorkerBudget::new(pool),
                    requested,
                    &mut budget,
                )
                .unwrap();
                let bands = frozen::RowBandPlan::for_output_height(dimensions, height).unwrap();
                let expected = frozen::RowBandWorkPlan::new(
                    &bands,
                    frozen::WorkerBudget::new(pool),
                    requested,
                )
                .unwrap();
                assert_eq!(actual.active_workers(), expected.active_workers());
                assert_eq!(actual.capacity_bytes(), budget.used());
                for (actual, expected) in actual.assignments().iter().zip(expected.assignments()) {
                    assert_eq!(actual.worker_index(), expected.worker_index());
                    assert_eq!(
                        actual
                            .bands()
                            .iter()
                            .map(|band| (band.y_start(), band.y_end()))
                            .collect::<Vec<_>>(),
                        expected
                            .bands()
                            .iter()
                            .map(|band| (band.y_start(), band.y_end()))
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}

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
fn worker_budget_uses_half_cpu_budget_clamped_to_eight() {
    assert_eq!(WorkerBudget::from_available_parallelism(1).pool_size(), 1);
    assert_eq!(WorkerBudget::from_available_parallelism(2).pool_size(), 1);
    assert_eq!(WorkerBudget::from_available_parallelism(8).pool_size(), 4);
    assert_eq!(WorkerBudget::from_available_parallelism(64).pool_size(), 8);

    assert_eq!(
        WorkerBudget::from_available_parallelism(8)
            .worker_counts()
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
}

#[test]
fn worker_budget_caps_active_workers_by_pool_and_work_items() {
    let budget = WorkerBudget::new(4);

    assert_eq!(budget.active_workers(8, 10), 4);
    assert_eq!(budget.active_workers(3, 2), 2);
    assert_eq!(budget.active_workers(0, 3), 1);
    assert_eq!(budget.active_workers(3, 0), 0);

    assert!(budget.can_use_workers(2, 2));
    assert!(!budget.can_use_workers(3, 2));
    assert!(!budget.can_use_workers(5, 8));
}

#[test]
fn row_band_work_plan_assigns_contiguous_chunks_to_active_workers() {
    let dimensions = ImageDimensions::new(8, 10).unwrap();
    let bands = RowBandPlan::for_output_height(dimensions, 2).unwrap();
    let work = RowBandWorkPlan::new(&bands, WorkerBudget::new(8), 3).unwrap();

    assert_eq!(work.requested_workers(), 3);
    assert_eq!(work.active_workers(), 3);
    assert_eq!(work.assignments().len(), 3);
    assert_eq!(work.assignments()[0].worker_index(), 0);
    assert_eq!(
        work.assignments()[0].bands(),
        &[RowBand::new(0, 2).unwrap(), RowBand::new(2, 4).unwrap()]
    );
    assert_eq!(
        work.assignments()[2].bands(),
        &[RowBand::new(8, 10).unwrap()]
    );
}

#[test]
fn row_band_work_plan_creates_one_assignment_per_active_worker() {
    let dimensions = ImageDimensions::new(8, 8).unwrap();
    let bands = RowBandPlan::for_output_height(dimensions, 2).unwrap();
    let work = RowBandWorkPlan::new(&bands, WorkerBudget::new(8), 3).unwrap();

    assert_eq!(work.active_workers(), 3);
    assert_eq!(work.assignments().len(), 3);
    assert_eq!(
        work.assignments()[0].bands(),
        &[RowBand::new(0, 2).unwrap(), RowBand::new(2, 4).unwrap()]
    );
    assert_eq!(
        work.assignments()[1].bands(),
        &[RowBand::new(4, 6).unwrap()]
    );
    assert_eq!(
        work.assignments()[2].bands(),
        &[RowBand::new(6, 8).unwrap()]
    );
}

#[test]
fn row_band_work_plan_caps_workers_by_band_count() {
    let dimensions = ImageDimensions::new(8, 4).unwrap();
    let bands = RowBandPlan::for_output_height(dimensions, 2).unwrap();
    let work = RowBandWorkPlan::new(&bands, WorkerBudget::new(8), 8).unwrap();

    assert_eq!(work.requested_workers(), 8);
    assert_eq!(work.active_workers(), 2);
    assert_eq!(work.assignments().len(), 2);
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
