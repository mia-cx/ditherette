use std::{hint::black_box, time::Duration};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ditherette_wasm::{
    image::{rgba, ImageDimensions},
    resize::area::resize_rgba_area_into,
};

#[derive(Clone, Copy)]
struct ResizeCase {
    label: &'static str,
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
}

const CASES: [ResizeCase; 4] = [
    ResizeCase {
        label: "integer_2x_downscale",
        source_width: 2600,
        source_height: 4168,
        output_width: 1300,
        output_height: 2084,
    },
    ResizeCase {
        label: "integer_4x_downscale",
        source_width: 2600,
        source_height: 4168,
        output_width: 650,
        output_height: 1042,
    },
    ResizeCase {
        label: "fractional_downscale",
        source_width: 2600,
        source_height: 4168,
        output_width: 1950,
        output_height: 3126,
    },
    ResizeCase {
        label: "near_identity_downscale",
        source_width: 2600,
        source_height: 4168,
        output_width: 2470,
        output_height: 3959,
    },
];

fn bench_area_resampling(c: &mut Criterion) {
    let mut group = c.benchmark_group("area_resampling");

    for case in CASES {
        let source_dimensions = dimensions(case.source_width, case.source_height);
        let output_dimensions = dimensions(case.output_width, case.output_height);
        let source_rgba = gradient_rgba(source_dimensions);
        let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions).unwrap();
        group.throughput(Throughput::Bytes(output_byte_len as u64));
        group.bench_function(
            BenchmarkId::new(
                case.label,
                format!(
                    "{}x{}-to-{}x{}",
                    source_dimensions.width(),
                    source_dimensions.height(),
                    output_dimensions.width(),
                    output_dimensions.height()
                ),
            ),
            |b| {
                let mut output_rgba = vec![0; output_byte_len];
                b.iter(|| {
                    resize_rgba_area_into(
                        black_box(&source_rgba),
                        source_dimensions,
                        output_dimensions,
                        black_box(&mut output_rgba),
                    )
                    .unwrap();
                    black_box(&output_rgba);
                });
            },
        );
    }

    group.finish();
}

fn gradient_rgba(dimensions: ImageDimensions) -> Vec<u8> {
    let width = dimensions.width_usize().unwrap();
    let height = dimensions.height_usize().unwrap();
    let mut rgba = Vec::with_capacity(width * height * 4);

    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&[
                x.wrapping_mul(17) as u8,
                y.wrapping_mul(31) as u8,
                (x ^ y) as u8,
                255,
            ]);
        }
    }

    rgba
}

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).expect("benchmark dimensions should be non-zero")
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_millis(800));
    targets = bench_area_resampling
}
criterion_main!(benches);
