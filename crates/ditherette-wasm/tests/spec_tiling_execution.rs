use ditherette_wasm::{image::ImageDimensions, spec::tiling::*};

#[test]
fn grids_cover_odd_outputs_once_in_row_major_order() {
    let dimensions = ImageDimensions::new(5, 3).unwrap();
    let grid = TileGrid::new(dimensions, 3, 2).unwrap();
    assert_eq!(grid.output_dimensions(), dimensions);
    assert_eq!(
        grid.tiles(),
        &[
            Tile::new(0, 0, 3, 2).unwrap(),
            Tile::new(3, 0, 5, 2).unwrap(),
            Tile::new(0, 2, 3, 3).unwrap(),
            Tile::new(3, 2, 5, 3).unwrap(),
        ]
    );
    let mut visits = [0; 15];
    for_each_tile(&grid, |tile| {
        assert_eq!(tile.width(), tile.x_end() - tile.x_start());
        assert_eq!(tile.height(), tile.y_end() - tile.y_start());
        for y in tile.y_start()..tile.y_end() {
            for x in tile.x_start()..tile.x_end() {
                visits[(y * 5 + x) as usize] += 1;
            }
        }
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(visits, [1; 15]);
    assert!(Tile::new(1, 0, 1, 2).is_none());
    assert!(Tile::new(0, 2, 1, 1).is_none());
    assert!(TileGrid::new(dimensions, 0, 1).is_none());
    assert!(TileGrid::new(dimensions, 1, 0).is_none());
    assert_eq!(
        TileGrid::new(dimensions, u32::MAX, u32::MAX)
            .unwrap()
            .tiles()
            .len(),
        1
    );
}

#[test]
fn band_planning_and_worker_assignment_preserve_global_order() {
    let dimensions = ImageDimensions::new(2, 13).unwrap();
    let plan = RowBandPlan::for_output_height(dimensions, 2).unwrap();
    assert_eq!(
        plan.bands()
            .iter()
            .map(|b| (b.y_start(), b.y_end()))
            .collect::<Vec<_>>(),
        vec![(0, 2), (2, 4), (4, 6), (6, 8), (8, 10), (10, 12), (12, 13)]
    );
    let work = RowBandWorkPlan::new(&plan, WorkerBudget::new(8), 3).unwrap();
    assert_eq!((work.requested_workers(), work.active_workers()), (3, 3));
    assert_eq!(
        work.assignments()
            .iter()
            .map(|a| (a.worker_index(), a.bands().len()))
            .collect::<Vec<_>>(),
        vec![(0, 3), (1, 2), (2, 2)]
    );
    assert_eq!(
        work.assignments()
            .iter()
            .flat_map(|a| a.bands())
            .copied()
            .collect::<Vec<_>>(),
        plan.bands()
    );
    assert!(RowBandPlan::for_output_height(dimensions, 0).is_none());
    assert_eq!(
        RowBandPlan::for_output_height(dimensions, u32::MAX)
            .unwrap()
            .bands(),
        &[RowBand::new(0, 13).unwrap()]
    );
}

#[test]
fn worker_counts_cap_requests_without_inventing_empty_work() {
    for (cpus, expected) in [(0, 1), (1, 1), (7, 3), (16, 8), (u32::MAX, 8)] {
        assert_eq!(
            WorkerBudget::from_available_parallelism(cpus).pool_size(),
            expected
        );
    }
    let budget = WorkerBudget::new(3);
    assert_eq!(budget.worker_counts().collect::<Vec<_>>(), vec![1, 2, 3]);
    assert_eq!(budget.active_workers(0, 9), 1);
    assert_eq!(budget.active_workers(9, 2), 2);
    assert_eq!(budget.active_workers(9, 0), 0);
    assert!(!budget.can_use_workers(0, 9));
    assert!(!budget.can_use_workers(4, 9));
    assert!(!budget.can_use_workers(3, 2));
    assert!(budget.can_use_workers(3, 3));
}

#[test]
fn sequential_visitors_stop_at_the_first_error() {
    let dimensions = ImageDimensions::new(3, 5).unwrap();
    let plan = RowBandPlan::for_output_height(dimensions, 2).unwrap();
    let mut rows = Vec::new();
    let error = for_each_row_band(&plan, |band| {
        rows.push(band.y_start());
        if band.y_start() == 2 {
            Err("row failure")
        } else {
            Ok(())
        }
    });
    assert_eq!(error, Err("row failure"));
    assert_eq!(rows, vec![0, 2]);
    let grid = TileGrid::new(dimensions, 2, 2).unwrap();
    let mut tiles = Vec::new();
    let error = for_each_tile(&grid, |tile| {
        tiles.push((tile.x_start(), tile.y_start()));
        if tile.x_start() == 2 {
            Err(7)
        } else {
            Ok(())
        }
    });
    assert_eq!(error, Err(7));
    assert_eq!(tiles, vec![(0, 0), (2, 0)]);
}
