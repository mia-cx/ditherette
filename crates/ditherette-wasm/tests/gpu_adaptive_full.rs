//! Optional full-photo proof; private host fixtures stay outside production code.
#![cfg(feature = "gpu")]

use ditherette_wasm::{
    image::{ImageDimensions, ImageView},
    prod::{dither::smooth_area_v1, gpu::Backend},
};
use pollster::block_on;

#[test]
#[ignore = "requires a host-provided 2600x4168 RGBA fixture"]
fn full_resolution_adaptive_gpu_probe() {
    let mut backend = block_on(Backend::detect());
    assert!(matches!(backend, Backend::Gpu(_)), "physical GPU required");
    let input =
        std::fs::read(std::env::var("DITHERETTE_GPU_RGBA").expect("RGBA fixture path")).unwrap();
    let dimensions = ImageDimensions::new(2600, 4168).unwrap();
    assert_eq!(input.len(), 2600 * 4168 * 4);
    for sample in [20, 64, 100] {
        let start = std::time::Instant::now();
        let output = block_on(backend.adaptive_coverage(
            dimensions,
            &input,
            sample,
            0.12,
            1536 * 1024 * 1024,
        ))
        .unwrap();
        let elapsed = start.elapsed();
        let mut reference = vec![[0.0; 3]; smooth_area_v1::scratch_len(dimensions)];
        smooth_area_v1::select(
            ImageView::packed(&input, dimensions).unwrap(),
            sample,
            0.12,
            &mut reference,
            || Ok::<_, ()>(()),
        )
        .unwrap();
        let differences = (0..2600 * 4168)
            .filter(|&i| {
                (output[i / 8] & (1 << (i % 8)) != 0)
                    != smooth_area_v1::covers(reference[i][2], i as u32 % 2600, i as u32 / 2600)
            })
            .count();
        eprintln!("full Adaptive sample {sample}: {differences}/10836800 coverage disagreements, GPU call {elapsed:?} (diagnostic shared-host observation)");
        assert!(
            differences < 10836800 / 200,
            "coverage differs at {differences} pixels"
        );
    }
}
