//! Treated-versus-untreated evaluation of automatic recolouring. Quality only; never times calls.
//!
//! Usage: `cargo run --release --example recolour_evaluation [sheet-directory]`.
//! Each benchmark fixture is area-resized to 320 px wide, then processed with and without a
//! leading `recolour` step, for several palettes and dither families, matching in Oklab.
//! With a directory argument, it also writes side-by-side PNG sheets there.

use std::{env, path::Path};

use ditherette_wasm::{
    image::contracts::{IndexedImage, PaletteEntry},
    spec::{
        color::oklab::rgb8_to_oklab,
        contract::request::Source,
        effects::{process, ProcessRequestV2, RecipeV2},
    },
};
use serde_json::json;

const FIXTURES: &[&str] = &[
    "Celeste_box_art.png",
    "Celeste_Insta_selfie.png",
    "Picking_at_thread.jpg",
    "The_Quilt_banner.png",
];
const WIDTH: u32 = 320;

fn palettes() -> Vec<(&'static str, Vec<[u8; 3]>)> {
    let hex = |values: &[u32]| {
        values
            .iter()
            .map(|v| [(v >> 16) as u8, (v >> 8) as u8, *v as u8])
            .collect::<Vec<_>>()
    };
    let wplace_free = hex(&[
        0x000000, 0x3C3C3C, 0x787878, 0xD2D2D2, 0xFFFFFF, 0x600018, 0xED1C24, 0xFF7F27, 0xF6AA09,
        0xF9DD3B, 0xFFFABC, 0x0EB968, 0x13E67B, 0x87FF5E, 0x0C816E, 0x10AE82, 0x13E1BE, 0x60F7F2,
        0x28509E, 0x4093E4, 0x6B50F6, 0x99B1FB, 0x780C99, 0xAA38B9, 0xE09FF9, 0xCB007A, 0xEC1F80,
        0xF38DA9, 0x684634, 0x95682A, 0xF8B277,
    ]);
    vec![
        ("wplace-free", wplace_free),
        (
            "pico-8",
            hex(&[
                0x000000, 0x1D2B53, 0x7E2553, 0x008751, 0xAB5236, 0x5F574F, 0xC2C3C7, 0xFFF1E8,
                0xFF004D, 0xFFA300, 0xFFEC27, 0x00E436, 0x29ADFF, 0x83769C, 0xFF77A8, 0xFFCCAA,
            ]),
        ),
        ("gameboy", hex(&[0x0F380F, 0x306230, 0x8BAC0F, 0x9BBC0F])),
        ("warm-4", hex(&[0x600018, 0xED1C24, 0xFF7F27, 0xF9DD3B])),
        (
            "cool-5",
            hex(&[0x0C816E, 0x13E1BE, 0x28509E, 0x4093E4, 0x99B1FB]),
        ),
        ("grey-4", hex(&[0x000000, 0x555555, 0xAAAAAA, 0xFFFFFF])),
    ]
}

fn dithers() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("none", json!({ "family": "none" })),
        (
            "bayer-8",
            json!({ "family": "separable", "perturb": { "field": { "algorithm": "bayer", "size": "8" },
                "space": "oklab", "strength": 0.5, "placement": { "mode": "everywhere" } } }),
        ),
        (
            "floyd-steinberg",
            json!({ "family": "diffusion", "kernel": "floyd-steinberg", "strength": 1.0,
                "placement": { "mode": "everywhere" }, "serpentine": true, "feedback": "matching" }),
        ),
    ]
}

struct Image {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// Resize with the reference itself (area), so every compared image shares one source.
fn load(name: &str) -> Image {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../benchmark-fixtures")
        .join(name);
    let image = image::open(&path).expect("fixture decodes").to_rgba8();
    let (width, height) = (image.width(), image.height());
    let output_height = (height as f64 * WIDTH as f64 / width as f64).round() as u32;
    let resized = ditherette_wasm::spec::resize::resize(
        ditherette_wasm::spec::contract::request::ResizeRequest {
            version: 1,
            source: Source {
                width,
                height,
                data: image.as_raw(),
            },
            output: serde_json::from_value(json!({ "width": WIDTH, "height": output_height,
            "resize": { "algorithm": "area" } }))
            .unwrap(),
        },
    )
    .expect("resize succeeds");
    Image {
        width: WIDTH,
        height: output_height,
        rgba: resized.into_vec(),
    }
}

fn run(
    image: &Image,
    palette: &[PaletteEntry],
    effects: serde_json::Value,
    dither: &serde_json::Value,
) -> IndexedImage {
    let recipe: RecipeV2 = serde_json::from_value(json!({
        "version": 2, "effects": effects,
        "output": { "width": image.width, "height": image.height, "resize": { "algorithm": "area" } },
        "alpha": { "mode": "preserve", "threshold": 127.5 }, "match": "oklab-euclidean", "dither": dither,
    }))
    .unwrap();
    process(ProcessRequestV2 {
        source: Source {
            width: image.width,
            height: image.height,
            data: &image.rgba,
        },
        palette,
        recipe: &recipe,
    })
    .expect("process succeeds")
}

/// Oklab of every pixel.
fn lab(rgba: &[u8]) -> Vec<[f32; 3]> {
    rgba.chunks(4)
        .map(|p| rgb8_to_oklab([p[0], p[1], p[2]]))
        .collect()
}

/// 5×5 box blur, roughly what the eye averages dithering into at viewing distance.
fn blur(values: &[[f32; 3]], width: usize, height: usize) -> Vec<[f32; 3]> {
    let mut out = vec![[0.0; 3]; values.len()];
    for y in 0..height {
        for x in 0..width {
            let mut sum = [0.0; 3];
            let mut count = 0.0;
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let (sx, sy) = (x as i32 + dx, y as i32 + dy);
                    if sx >= 0 && sy >= 0 && (sx as usize) < width && (sy as usize) < height {
                        let v = values[sy as usize * width + sx as usize];
                        for c in 0..3 {
                            sum[c] += v[c];
                        }
                        count += 1.0;
                    }
                }
            }
            out[y * width + x] = sum.map(|s| s / count);
        }
    }
    out
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn gradients(values: &[[f32; 3]], width: usize, height: usize) -> Vec<f32> {
    let mut out = Vec::new();
    for y in 0..height - 1 {
        for x in 0..width - 1 {
            let at = |x: usize, y: usize| values[y * width + x][0];
            out.push((at(x + 1, y) - at(x, y)).hypot(at(x, y + 1) - at(x, y)));
        }
    }
    out
}

fn correlation(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len() as f32;
    let (ma, mb) = (a.iter().sum::<f32>() / n, b.iter().sum::<f32>() / n);
    let (mut cov, mut va, mut vb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        cov += (x - ma) * (y - mb);
        va += (x - ma).powi(2);
        vb += (y - mb).powi(2);
    }
    cov / (va.sqrt() * vb.sqrt()).max(1e-12)
}

#[derive(Default, Clone, Copy)]
struct Scores {
    shift: f32,
    detail: f32,
    loss: f32,
    colors: f32,
    texture: f32,
}

/// Scores one indexed output against the continuous source it came from.
fn score(source: &Image, output: &IndexedImage) -> Scores {
    let (w, h) = (source.width as usize, source.height as usize);
    let rgba: Vec<u8> = output
        .indices
        .data()
        .iter()
        .flat_map(|&i| output.palette.rgba[i as usize * 4..i as usize * 4 + 4].to_vec())
        .collect();
    let (src, out) = (lab(&source.rgba), lab(&rgba));
    let (src_blur, out_blur) = (blur(&src, w, h), blur(&out, w, h));
    let n = src.len() as f32;
    let shift = src_blur
        .iter()
        .zip(&out_blur)
        .map(|(a, b)| distance(*a, *b))
        .sum::<f32>()
        / n;
    let detail = correlation(&gradients(&src_blur, w, h), &gradients(&out_blur, w, h));
    let chromatic: Vec<usize> = (0..src.len())
        .filter(|&i| src_blur[i][1].hypot(src_blur[i][2]) > 0.04)
        .collect();
    let loss = if chromatic.is_empty() {
        0.0
    } else {
        chromatic
            .iter()
            .filter(|&&i| out_blur[i][1].hypot(out_blur[i][2]) < 0.02)
            .count() as f32
            / chromatic.len() as f32
    };
    let mut used = output.indices.data().to_vec();
    used.sort_unstable();
    used.dedup();
    let texture = out
        .iter()
        .zip(&out_blur)
        .map(|(a, b)| (a[0] - b[0]).abs())
        .sum::<f32>()
        / n;
    Scores {
        shift,
        detail,
        loss,
        colors: used.len() as f32,
        texture,
    }
}

fn write_sheet(directory: &Path, name: &str, source: &Image, outputs: [&IndexedImage; 2]) {
    let (w, h) = (source.width, source.height);
    let mut sheet = image::RgbaImage::new(w * 3, h);
    let pixel = |x: u32, y: u32, rgba: &[u8]| {
        let i = ((y * w + x) * 4) as usize;
        image::Rgba([rgba[i], rgba[i + 1], rgba[i + 2], 255])
    };
    for (column, rgba) in [
        source.rgba.clone(),
        indexed_rgba(outputs[0]),
        indexed_rgba(outputs[1]),
    ]
    .iter()
    .enumerate()
    {
        for y in 0..h {
            for x in 0..w {
                sheet.put_pixel(column as u32 * w + x, y, pixel(x, y, rgba));
            }
        }
    }
    sheet
        .save(directory.join(format!("{name}.png")))
        .expect("sheet saves");
}

fn indexed_rgba(output: &IndexedImage) -> Vec<u8> {
    output
        .indices
        .data()
        .iter()
        .flat_map(|&i| output.palette.rgba[i as usize * 4..i as usize * 4 + 4].to_vec())
        .collect()
}

fn main() {
    let sheets = env::args().nth(1);
    let images: Vec<(&str, Image)> = FIXTURES.iter().map(|name| (*name, load(name))).collect();
    println!("| Palette | Dither | Shift ΔE (−) | Detail r (+) | Colour loss (−) | Colours used (+) | Texture (−) |");
    println!("| --- | --- | --- | --- | --- | --- | --- |");
    for (palette_name, colors) in palettes() {
        let palette: Vec<PaletteEntry> = colors
            .iter()
            .map(|&rgb| PaletteEntry::Color { rgb })
            .collect();
        for (dither_name, dither) in dithers() {
            let mut totals = [Scores::default(); 2];
            for (fixture, image) in &images {
                let untreated = run(image, &palette, json!([]), &dither);
                let treated = run(
                    image,
                    &palette,
                    json!([{ "effect": "recolour", "enabled": true, "strength": 1, "recipe": null }]),
                    &dither,
                );
                for (total, output) in totals.iter_mut().zip([&untreated, &treated]) {
                    let s = score(image, output);
                    total.shift += s.shift;
                    total.detail += s.detail;
                    total.loss += s.loss;
                    total.colors += s.colors;
                    total.texture += s.texture;
                }
                if let Some(directory) = &sheets {
                    let stem = fixture.rsplit_once('.').map_or(*fixture, |(stem, _)| stem);
                    write_sheet(
                        Path::new(directory),
                        &format!("{palette_name}-{dither_name}-{stem}"),
                        image,
                        [&untreated, &treated],
                    );
                }
            }
            let n = images.len() as f32;
            let [u, t] = totals.map(|s| Scores {
                shift: s.shift / n,
                detail: s.detail / n,
                loss: s.loss / n,
                colors: s.colors / n,
                texture: s.texture / n,
            });
            println!(
                "| {palette_name} | {dither_name} | {:.4} → {:.4} | {:.3} → {:.3} | {:.1}% → {:.1}% | {:.1} → {:.1} | {:.4} → {:.4} |",
                u.shift, t.shift, u.detail, t.detail, u.loss * 100.0, t.loss * 100.0, u.colors, t.colors, u.texture, t.texture
            );
        }
    }
}
