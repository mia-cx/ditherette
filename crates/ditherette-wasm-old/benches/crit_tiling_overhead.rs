use std::{hint::black_box, time::Duration};

use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use ditherette_wasm_old::{
    image::ImageDimensions,
    resize::{
        area::{
            resize_rgba_area_dynamic_tiling_plan, resize_rgba_area_into,
            resize_rgba_area_scalar_into,
        },
        cpu_tiling::{process_row_bands_with_plan, RowBand, RowBandPlan},
        nearest::{
            resize_rgba_nearest_dynamic_tiling_plan, resize_rgba_nearest_into,
            resize_rgba_nearest_scalar_into,
        },
    },
};

const SOURCE_WIDTH: u32 = 650;
const SOURCE_HEIGHT: u32 = 1042;
const SCALES: [Scale; 6] = [
    Scale::new("2x", 2.0),
    Scale::new("1x", 1.0),
    Scale::new("0.8x", 0.8),
    Scale::new("0.5x", 0.5),
    Scale::new("0.25x", 0.25),
    Scale::new("0.125x", 0.125),
];

#[derive(Clone, Copy)]
struct Scale {
    label: &'static str,
    factor: f64,
}

impl Scale {
    const fn new(label: &'static str, factor: f64) -> Self {
        Self { label, factor }
    }
}

#[derive(Clone, Copy)]
enum Filter {
    Area,
    Nearest,
}

impl Filter {
    fn label(self) -> &'static str {
        match self {
            Self::Area => "area",
            Self::Nearest => "nearest",
        }
    }

    fn dynamic_plan(
        self,
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
    ) -> Option<RowBandPlan> {
        match self {
            Self::Area => {
                resize_rgba_area_dynamic_tiling_plan(source_dimensions, output_dimensions)
            }
            Self::Nearest => {
                resize_rgba_nearest_dynamic_tiling_plan(source_dimensions, output_dimensions)
            }
        }
        .unwrap()
    }
}

fn bench_tiling_overhead(c: &mut Criterion) {
    let source_dimensions = dimensions(SOURCE_WIDTH, SOURCE_HEIGHT);
    let mut group = c.benchmark_group("tiling_overhead/shared");
    group.sample_size(20);
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_millis(800));

    for filter in [Filter::Nearest, Filter::Area] {
        for scale in SCALES {
            let output_dimensions = scaled_dimensions(source_dimensions, scale);
            let plan = filter
                .dynamic_plan(source_dimensions, output_dimensions)
                .unwrap_or_else(|| scalar_plan(output_dimensions));
            let mut output_rgba = output_buffer(output_dimensions);
            group.throughput(Throughput::Bytes(output_rgba.len() as u64));

            let parameters = format!(
                "{} bands={} workers={} rows={} pixels_per_band={}",
                scale.label,
                plan.band_count,
                plan.worker_count,
                plan.band_height,
                plan.min_pixels_per_band
            );

            group.bench_function(
                BenchmarkId::new(format!("{}/noop", filter.label()), &parameters),
                |b| {
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
                BenchmarkId::new(format!("{}/one_byte_per_row", filter.label()), &parameters),
                |b| {
                    b.iter(|| {
                        process_row_bands_with_plan(
                            black_box(&mut output_rgba),
                            black_box(plan),
                            |band, rows| {
                                touch_one_byte_per_row(plan.output_width, band, rows);
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

fn bench_resize_context(c: &mut Criterion) {
    let source_dimensions = dimensions(SOURCE_WIDTH, SOURCE_HEIGHT);
    let source_rgba = source_image(source_dimensions);
    let mut group = c.benchmark_group("tiling_overhead/resize_context");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_millis(900));

    for scale in SCALES {
        let output_dimensions = scaled_dimensions(source_dimensions, scale);
        let mut output_rgba = output_buffer(output_dimensions);
        group.throughput(Throughput::Bytes(output_rgba.len() as u64));

        group.bench_function(BenchmarkId::new("nearest/scalar", scale.label), |b| {
            b.iter(|| {
                resize_rgba_nearest_scalar_into(
                    black_box(&source_rgba),
                    black_box(source_dimensions),
                    black_box(output_dimensions),
                    black_box(&mut output_rgba),
                )
                .unwrap();
                black_box(&output_rgba);
            });
        });

        group.bench_function(BenchmarkId::new("nearest/tiled", scale.label), |b| {
            b.iter(|| {
                resize_rgba_nearest_into(
                    black_box(&source_rgba),
                    black_box(source_dimensions),
                    black_box(output_dimensions),
                    black_box(&mut output_rgba),
                )
                .unwrap();
                black_box(&output_rgba);
            });
        });

        group.bench_function(BenchmarkId::new("area/scalar", scale.label), |b| {
            b.iter(|| {
                resize_rgba_area_scalar_into(
                    black_box(&source_rgba),
                    black_box(source_dimensions),
                    black_box(output_dimensions),
                    black_box(&mut output_rgba),
                )
                .unwrap();
                black_box(&output_rgba);
            });
        });

        group.bench_function(BenchmarkId::new("area/tiled", scale.label), |b| {
            b.iter(|| {
                resize_rgba_area_into(
                    black_box(&source_rgba),
                    black_box(source_dimensions),
                    black_box(output_dimensions),
                    black_box(&mut output_rgba),
                )
                .unwrap();
                black_box(&output_rgba);
            });
        });
    }

    group.finish();
}

fn touch_one_byte_per_row(output_width: usize, band: RowBand, rows: &mut [u8]) {
    let row_byte_len = output_width * 4;
    for row in 0..band.output_y_end - band.output_y_start {
        let byte = &mut rows[row * row_byte_len];
        *byte = byte.wrapping_add(1);
    }
}

fn source_image(dimensions: ImageDimensions) -> Vec<u8> {
    let width = dimensions.width_usize().unwrap();
    let height = dimensions.height_usize().unwrap();
    let mut image = vec![0; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let offset = (y * width + x) * 4;
            image[offset] = ((x * 3 + y * 5) & 0xff) as u8;
            image[offset + 1] = ((x * 7 + y * 11) & 0xff) as u8;
            image[offset + 2] = ((x * 13 + y * 17) & 0xff) as u8;
            image[offset + 3] = 255;
        }
    }

    image
}

fn output_buffer(dimensions: ImageDimensions) -> Vec<u8> {
    vec![0; dimensions.pixel_count().unwrap() * 4]
}

fn scalar_plan(output_dimensions: ImageDimensions) -> RowBandPlan {
    RowBandPlan {
        output_width: output_dimensions.width_usize().unwrap(),
        output_height: output_dimensions.height_usize().unwrap(),
        available_logical_threads: 1,
        worker_count: 1,
        band_count: 1,
        band_height: output_dimensions.height_usize().unwrap(),
        min_rows_per_band: output_dimensions.height_usize().unwrap(),
        min_parallel_output_pixels: usize::MAX,
        min_pixels_per_band: usize::MAX,
        max_workers: 1,
    }
}

fn scaled_dimensions(source_dimensions: ImageDimensions, scale: Scale) -> ImageDimensions {
    dimensions(
        scaled_length(source_dimensions.width(), scale.factor),
        scaled_length(source_dimensions.height(), scale.factor),
    )
}

fn scaled_length(length: u32, scale: f64) -> u32 {
    ((f64::from(length) * scale).round() as u32).max(1)
}

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).unwrap()
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = bench_tiling_overhead, bench_resize_context
}
criterion_main!(benches);
