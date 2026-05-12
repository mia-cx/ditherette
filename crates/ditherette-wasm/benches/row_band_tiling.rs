use std::{hint::black_box, time::Duration};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ditherette_wasm::resize::cpu_tiling::{
    plan_row_bands, process_row_bands, process_row_bands_with_plan, RowBandTiling,
};

#[derive(Clone, Copy)]
struct OutputCase {
    label: &'static str,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy)]
struct WorkerCase {
    max_workers: usize,
    min_pixels_per_band: usize,
}

const OUTPUT_CASES: [OutputCase; 2] = [
    OutputCase {
        label: "near_identity_downscale",
        width: 2470,
        height: 3959,
    },
    OutputCase {
        label: "below_threshold",
        width: 650,
        height: 1042,
    },
];

const WORKER_CASES: [WorkerCase; 3] = [
    WorkerCase {
        max_workers: 1,
        min_pixels_per_band: usize::MAX,
    },
    WorkerCase {
        max_workers: 5,
        min_pixels_per_band: 1,
    },
    WorkerCase {
        max_workers: 8,
        min_pixels_per_band: 1,
    },
];

fn bench_plan_row_bands(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_band_tiling/plan");

    for output in OUTPUT_CASES {
        for workers in WORKER_CASES {
            let tiling = tiling_for(workers);
            let plan = plan_row_bands(output.width, output.height, tiling);
            group.bench_function(
                BenchmarkId::new(
                    output.label,
                    format!(
                        "max_workers={} resolved_bands={}",
                        workers.max_workers, plan.band_count
                    ),
                ),
                |b| {
                    b.iter(|| {
                        black_box(plan_row_bands(
                            black_box(output.width),
                            black_box(output.height),
                            black_box(tiling),
                        ));
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_process_row_bands(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_band_tiling/process");

    for output in OUTPUT_CASES {
        let byte_len = output.width * output.height * 4;

        for workers in WORKER_CASES {
            let tiling = tiling_for(workers);
            let plan = plan_row_bands(output.width, output.height, tiling);
            let parameter = format!(
                "max_workers={} bands={} band_height={}",
                workers.max_workers, plan.band_count, plan.band_height
            );

            group.bench_function(
                BenchmarkId::new(format!("{}/noop", output.label), &parameter),
                |b| {
                    let mut output_rgba = vec![0; byte_len];
                    b.iter(|| {
                        process_row_bands(
                            black_box(&mut output_rgba),
                            black_box(output.width),
                            black_box(output.height),
                            black_box(tiling),
                            |band, rows| {
                                black_box(band);
                                black_box(rows.len());
                                Ok(())
                            },
                        )
                        .unwrap();
                        black_box(&output_rgba);
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new(format!("{}/noop_preplanned", output.label), &parameter),
                |b| {
                    let mut output_rgba = vec![0; byte_len];
                    b.iter(|| {
                        process_row_bands_with_plan(
                            black_box(&mut output_rgba),
                            black_box(plan),
                            |band, rows| {
                                black_box(band);
                                black_box(rows.len());
                                Ok(())
                            },
                        )
                        .unwrap();
                        black_box(&output_rgba);
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new(format!("{}/tiny_row", output.label), &parameter),
                |b| {
                    let mut output_rgba = vec![0; byte_len];
                    b.iter(|| {
                        process_row_bands(
                            black_box(&mut output_rgba),
                            black_box(output.width),
                            black_box(output.height),
                            black_box(tiling),
                            |band, rows| {
                                tiny_row_kernel(output.width, band, rows);
                                Ok(())
                            },
                        )
                        .unwrap();
                        black_box(&output_rgba);
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new(format!("{}/tiny_row_preplanned", output.label), &parameter),
                |b| {
                    let mut output_rgba = vec![0; byte_len];
                    b.iter(|| {
                        process_row_bands_with_plan(
                            black_box(&mut output_rgba),
                            black_box(plan),
                            |band, rows| {
                                tiny_row_kernel(output.width, band, rows);
                                Ok(())
                            },
                        )
                        .unwrap();
                        black_box(&output_rgba);
                    });
                },
            );
        }
    }

    group.finish();
}

fn tiny_row_kernel(
    output_width: usize,
    band: ditherette_wasm::resize::cpu_tiling::RowBand,
    rows: &mut [u8],
) {
    let row_byte_len = output_width * 4;
    for row in 0..band.output_y_end - band.output_y_start {
        rows[row * row_byte_len] = rows[row * row_byte_len].wrapping_add(1);
    }
}

fn tiling_for(workers: WorkerCase) -> RowBandTiling {
    RowBandTiling::new(0, workers.min_pixels_per_band, 1, workers.max_workers)
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_millis(800));
    targets = bench_plan_row_bands, bench_process_row_bands
}
criterion_main!(benches);
