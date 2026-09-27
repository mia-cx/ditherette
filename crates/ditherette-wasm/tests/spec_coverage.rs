use ditherette_wasm::spec::{
    contract::request::{Anchor, Output, ResizePolicy, ResizeRequest, Source, Support},
    coverage, resize,
};

const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];

fn policies() -> Vec<ResizePolicy> {
    let anchor = Anchor::Center;
    let mut policies = vec![
        ResizePolicy::Area {},
        ResizePolicy::Bilinear { anchor },
        ResizePolicy::Trilinear { anchor },
    ];
    for support in [Support::Fixed, Support::ScaleAware] {
        policies.extend([
            ResizePolicy::Bicubic { anchor, support },
            ResizePolicy::Lanczos2 { anchor, support },
            ResizePolicy::Lanczos3 { anchor, support },
        ]);
    }
    policies
}

fn request(
    data: &[u8],
    width: u32,
    height: u32,
    output: (u32, u32),
    resize: ResizePolicy,
) -> ResizeRequest<'_> {
    ResizeRequest {
        version: 1,
        source: Source {
            width,
            height,
            data,
        },
        output: Output {
            width: output.0,
            height: output.1,
            resize,
        },
    }
}

/// A 3x3 opaque block centred in a 9x9 transparent field whose hidden RGB is `hidden`.
fn sprite(hidden: [u8; 3]) -> Vec<u8> {
    (0..81)
        .flat_map(|index| {
            let (x, y) = (index % 9, index / 9);
            if (3..6).contains(&x) && (3..6).contains(&y) {
                [200, 40, 90, 255]
            } else {
                [hidden[0], hidden[1], hidden[2], 0]
            }
        })
        .collect()
}

#[test]
fn nearest_and_opaque_sources_match_v1_exactly() {
    let opaque: Vec<u8> = (0..36u8)
        .flat_map(|v| [v * 7, 255 - v * 5, v * 3, 255])
        .collect();
    let translucent = sprite([10, 250, 10]);
    for resize_policy in policies() {
        for output in [(3, 3), (13, 11)] {
            let v1 = resize::resize(request(&opaque, 6, 6, output, resize_policy)).unwrap();
            let covered = coverage::resize(request(&opaque, 6, 6, output, resize_policy)).unwrap();
            assert_eq!(covered.data(), v1.data(), "{resize_policy:?} {output:?}");
        }
    }
    let nearest = ResizePolicy::Nearest {
        anchor: Anchor::Center,
    };
    assert_eq!(
        coverage::resize(request(&translucent, 9, 9, (20, 20), nearest))
            .unwrap()
            .data(),
        resize::resize(request(&translucent, 9, 9, (20, 20), nearest))
            .unwrap()
            .data()
    );
}

#[test]
fn hidden_colour_under_transparent_pixels_never_reaches_the_output() {
    let black = sprite([0, 0, 0]);
    let green = sprite([0, 255, 0]);
    for resize_policy in policies() {
        for output in [(4, 4), (20, 17)] {
            let dark = coverage::resize(request(&black, 9, 9, output, resize_policy)).unwrap();
            let light = coverage::resize(request(&green, 9, 9, output, resize_policy)).unwrap();
            assert_eq!(dark.data(), light.data(), "{resize_policy:?} {output:?}");
            // Every visible pixel carries the block's colour, not a blend with hidden black.
            for pixel in dark.data().chunks_exact(4).filter(|pixel| pixel[3] > 0) {
                assert!(
                    pixel[0] >= 190 && pixel[2] >= 80,
                    "{resize_policy:?} {pixel:?}"
                );
            }
        }
    }
}

#[test]
fn ringing_kernels_leave_transparent_neighbourhoods_transparent() {
    let source = sprite([0, 0, 0]);
    let scale = 5;
    for resize_policy in policies() {
        let output = coverage::resize(request(
            &source,
            9,
            9,
            (9 * scale, 9 * scale),
            resize_policy,
        ))
        .unwrap();
        for (index, pixel) in output.data().chunks_exact(4).enumerate() {
            let (x, y) = ((index % 45) as f64, (index / 45) as f64);
            // Centre-anchored source position; the block spans source 3..6.
            let (sx, sy) = (
                (x + 0.5) / scale as f64 - 0.5,
                (y + 0.5) / scale as f64 - 0.5,
            );
            let gap = |p: f64| (2.0 - p).max(p - 6.0).max(0.0);
            if gap(sx).max(gap(sy)) >= 1.0 {
                assert_eq!(pixel, TRANSPARENT, "{resize_policy:?} at ({x}, {y})");
            }
        }
    }
}

#[test]
fn a_transparent_source_resizes_to_transparent() {
    let source = [9, 8, 7, 0].repeat(16);
    for resize_policy in policies() {
        let output = coverage::resize(request(&source, 4, 4, (7, 3), resize_policy)).unwrap();
        assert!(
            output
                .data()
                .chunks_exact(4)
                .all(|pixel| pixel == TRANSPARENT),
            "{resize_policy:?}"
        );
    }
}
