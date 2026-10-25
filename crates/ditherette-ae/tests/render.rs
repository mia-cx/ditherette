use ditherette_ae::{
    look::{Dither, Look, Row, DITHERS},
    palette,
    render::{Engine, Image, ImageMut, Ran},
};
use ditherette_wasm::{
    image::contracts::PaletteEntry,
    prod::{
        contract::request::{Anchor, BayerSize, Field, ResizePolicy},
        gpu::Fallback,
    },
};

const BLACK: PaletteEntry = PaletteEntry::Color { rgb: [0, 0, 0] };
const WHITE: PaletteEntry = PaletteEntry::Color {
    rgb: [255, 255, 255],
};

/// Rows of ARGB pixels, each row followed by `padding` bytes.
fn frame(rows: &[&[[u8; 4]]], padding: usize) -> Vec<u8> {
    rows.iter()
        .flat_map(|row| {
            row.iter()
                .flatten()
                .copied()
                .chain(std::iter::repeat_n(0xAB, padding))
        })
        .collect()
}

#[test]
fn renders_palette_cells_at_the_pixel_size_and_keeps_transparency() {
    let dark = [255, 20, 20, 20];
    let light = [255, 230, 230, 230];
    let clear = [0, 90, 90, 90];
    let padding = 8;
    let input = frame(
        &[
            &[dark, dark, light, light],
            &[dark, dark, light, light],
            &[clear, clear, light, light],
        ],
        padding,
    );
    let mut output = vec![0xAB; input.len()];
    let stride = (4 * 4 + padding) as isize;
    let look = Look {
        pixel_size: 2,
        resize: ResizePolicy::Nearest {
            anchor: Anchor::Center,
        },
        ..Look::default()
    };

    Engine::default()
        .render(
            &look,
            &[BLACK, WHITE, PaletteEntry::Transparent {}],
            Image {
                width: 4,
                height: 3,
                stride,
                argb: &input,
            },
            &mut ImageMut {
                width: 4,
                height: 3,
                stride,
                argb: &mut output,
            },
        )
        .unwrap();

    let black = [255, 0, 0, 0];
    let white = [255, 255, 255, 255];
    let none = [0, 0, 0, 0];
    assert_eq!(
        output,
        frame(
            &[
                &[black, black, white, white],
                &[black, black, white, white],
                &[none, none, white, white]
            ],
            padding,
        ),
        "row padding stays untouched"
    );
}

#[test]
fn every_dither_choice_renders_only_palette_colours() {
    let (width, height) = (24, 16);
    let gradient: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).map(move |x| [255, (x * 10) as u8, (y * 15) as u8, 128]))
        .flatten()
        .collect();
    let palette = [BLACK, WHITE, PaletteEntry::Color { rgb: [200, 40, 40] }];
    let engine = Engine::default();
    for row in DITHERS {
        let Row::Choice(label, dither) = row else {
            continue;
        };
        for pixel_size in [1, 3] {
            let look = Look {
                dither: *dither,
                pixel_size,
                ..Look::default()
            };
            let mut output = vec![0; gradient.len()];
            engine
                .render(
                    &look,
                    &palette,
                    Image {
                        width,
                        height,
                        stride: width as isize * 4,
                        argb: &gradient,
                    },
                    &mut ImageMut {
                        width,
                        height,
                        stride: width as isize * 4,
                        argb: &mut output,
                    },
                )
                .unwrap_or_else(|failure| panic!("{label} at {pixel_size} px: {failure:?}"));
            for pixel in output.chunks_exact(4) {
                assert!(
                    [[255, 0, 0, 0], [255, 255, 255, 255], [255, 200, 40, 40]]
                        .contains(&pixel.try_into().unwrap()),
                    "{label} at {pixel_size} px painted {pixel:?}"
                );
            }
        }
    }
}

/// Exact stages only: GPU diffusion is banded, so it is not index-identical to the CPU scan.
#[test]
fn the_cpu_fallback_paints_the_same_frame_as_the_gpu() {
    let (width, height) = (40, 30);
    let photo: Vec<u8> = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                [
                    if x < 4 { 0 } else { 255 },
                    (x * 6) as u8,
                    (y * 8) as u8,
                    (x * y) as u8,
                ]
            })
        })
        .flatten()
        .collect();
    let gpu = Engine::default();
    let cpu = Engine::cpu();
    let looks = [
        Look {
            pixel_size: 3,
            resize: ResizePolicy::Area {},
            ..Look::default()
        },
        Look {
            pixel_size: 1,
            dither: Dither::Field(Field::Bayer {
                size: BayerSize::Four,
            }),
            ..Look::default()
        },
        Look {
            pixel_size: 4,
            dither: Dither::Yliluoma(BayerSize::Four),
            ..Look::default()
        },
    ];
    for look in looks {
        let render = |engine: &Engine| {
            let mut output = vec![0; photo.len()];
            let ran = engine
                .render(
                    &look,
                    &palette::wplace(),
                    Image {
                        width,
                        height,
                        stride: width as isize * 4,
                        argb: &photo,
                    },
                    &mut ImageMut {
                        width,
                        height,
                        stride: width as isize * 4,
                        argb: &mut output,
                    },
                )
                .unwrap();
            (ran, output)
        };
        let (ran, on_gpu) = render(&gpu);
        if ran != Ran::Gpu {
            eprintln!("skipped: no GPU ({ran:?})");
            return;
        }
        assert_eq!(
            render(&cpu),
            (Ran::Cpu(Fallback::NoAdapter), on_gpu),
            "{look:?}"
        );
    }
}

#[test]
fn a_swatch_layer_gives_its_opaque_colours_in_reading_order() {
    let red = [255, 255, 0, 0];
    let blue = [255, 0, 0, 255];
    let edge = [128, 0, 255, 0];
    // Stored bottom-up, as After Effects may hand over a layer.
    let stored = frame(&[&[blue, edge], &[red, blue]], 0);
    let layer = Image {
        width: 2,
        height: 2,
        stride: -8,
        argb: &stored,
    };

    assert_eq!(
        palette::from_layer(layer),
        [
            PaletteEntry::Color { rgb: [255, 0, 0] },
            PaletteEntry::Color { rgb: [0, 0, 255] },
            PaletteEntry::Transparent {},
        ]
    );
}
