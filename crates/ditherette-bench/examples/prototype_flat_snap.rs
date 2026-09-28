//! PROTOTYPE, throwaway. Question: can we keep flat fills clean (one palette colour) while wide
//! gradients keep dithering smoothly?
//!
//! Usage: `cargo run --release --example prototype_flat_snap [out-dir]` (default /tmp/flat-snap).
//! Fills are found spatially: split the image into regions at colour edges, then call a region a
//! fill when its colours stay close to its mean. A wide gradient is one region with a wide spread.
//! Colour statistics alone can't tell them apart: a histogram rated the logo face and the sky
//! gradient equally compact, and the sky is locally flatter than the logo.
//! Panels, all after the current palette fit, Wplace 64, Oklab, Atkinson:
//! 1: Mia's adaptive settings. 2: everywhere. 3: everywhere, fills undithered. 4: fill mask.

use std::{env, path::Path};

use ditherette_wasm::{
    image::{contracts::PaletteEntry, dimensions::ImageDimensions, owned::ImageBuf},
    spec::{
        color::oklab::{oklab_to_rgb8, rgb8_to_oklab},
        contract::request::{Source, WorkingSpace},
        effects::{
            process, recolour_analysis::analyze, EffectContext, EffectImage, ProcessRequestV2,
            RecipeV2,
        },
    },
};
use serde_json::json;

const WPLACE: &[u32] = &[
    0x000000, 0x3C3C3C, 0x787878, 0xD2D2D2, 0xFFFFFF, 0x600018, 0xED1C24, 0xFF7F27, 0xF6AA09,
    0xF9DD3B, 0xFFFABC, 0x0EB968, 0x13E67B, 0x87FF5E, 0x0C816E, 0x10AE82, 0x13E1BE, 0x60F7F2,
    0x28509E, 0x4093E4, 0x6B50F6, 0x99B1FB, 0x780C99, 0xAA38B9, 0xE09FF9, 0xCB007A, 0xEC1F80,
    0xF38DA9, 0x684634, 0x95682A, 0xF8B277, 0xAAAAAA, 0xA50E1E, 0xFA8072, 0xE45C1A, 0x9C8431,
    0xC5AD31, 0xE8D45F, 0x4A6B3A, 0x5A944A, 0x84C573, 0x0F799F, 0xBBFAF2, 0x7DC7FF, 0x4D31B8,
    0x4A4284, 0x7A71C4, 0xB5AEF1, 0x9B5249, 0xD18078, 0xFAB6A4, 0xDBA463, 0x7B6352, 0x9C846B,
    0xD6B594, 0xD18051, 0xFFC5A5, 0x6D643F, 0x948C6B, 0xCDC59E, 0x333941, 0x6D758D, 0xB3B9D1,
];

// Output size of Mia's screenshot, and the crop: mountain, sky, and logo.
const WIDTH: u32 = 1610;
const HEIGHT: u32 = 2581;
const CROP: (u32, u32, u32, u32) = (240, 160, 1120, 700);

// Knobs, in Oklab units.
const EDGE: f32 = 0.006; // neighbours closer than this join one region
const MIN_PIXELS: usize = 400; // smaller regions are detail, not fills
const FILL_SPREAD: f32 = 0.025; // RMS distance from the region mean; above it, a gradient

type Lab = [f32; 3];

fn sub(a: Lab, b: Lab) -> Lab {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn len(a: Lab) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

fn find(parent: &mut [u32], mut i: u32) -> u32 {
    while parent[i as usize] != i {
        parent[i as usize] = parent[parent[i as usize] as usize];
        i = parent[i as usize];
    }
    i
}

/// Region label per pixel, and whether each region is a fill. Never reads the palette.
fn find_fills(labs: &[Lab], width: usize) -> (Vec<u32>, Vec<bool>) {
    let n = labs.len();
    let mut parent: Vec<u32> = (0..n as u32).collect();
    for i in 0..n {
        for j in [i + 1, i + width] {
            if (j == i + 1 && j % width == 0) || j >= n || len(sub(labs[i], labs[j])) >= EDGE {
                continue;
            }
            let (a, b) = (find(&mut parent, i as u32), find(&mut parent, j as u32));
            if a != b {
                parent[a.max(b) as usize] = a.min(b);
            }
        }
    }
    let labels: Vec<u32> = (0..n as u32).map(|i| find(&mut parent, i)).collect();

    let mut count = vec![0usize; n];
    let mut sum = vec![[0.0f64; 3]; n];
    for (i, &l) in labels.iter().enumerate() {
        count[l as usize] += 1;
        for c in 0..3 {
            sum[l as usize][c] += labs[i][c] as f64;
        }
    }
    let mean: Vec<Lab> = (0..n)
        .map(|l| sum[l].map(|s| (s / count[l].max(1) as f64) as f32))
        .collect();
    let mut sq = vec![0.0f64; n];
    for (i, &l) in labels.iter().enumerate() {
        sq[l as usize] += (len(sub(labs[i], mean[l as usize])) as f64).powi(2);
    }

    let mut report = Vec::new();
    let fills: Vec<bool> = (0..n)
        .map(|l| {
            if count[l] < MIN_PIXELS {
                return false;
            }
            let spread = (sq[l] / count[l] as f64).sqrt() as f32;
            report.push((count[l], mean[l], spread));
            spread <= FILL_SPREAD
        })
        .collect();
    report.sort_by(|a, b| b.0.cmp(&a.0));
    println!(
        "{} regions ≥ {MIN_PIXELS} px, {} fills",
        report.len(),
        fills.iter().filter(|&&f| f).count()
    );
    for (count, mean, spread) in report.iter().take(15) {
        let c = oklab_to_rgb8(*mean);
        println!(
            "{:5.2}% #{:02X}{:02X}{:02X} spread {:.3} {}",
            *count as f32 / n as f32 * 100.0,
            c[0],
            c[1],
            c[2],
            spread,
            if *spread <= FILL_SPREAD {
                "fill"
            } else {
                "gradient"
            }
        );
    }
    (labels, fills)
}

/// Mia's dither settings, and the same kernel placed everywhere.
fn yours() -> serde_json::Value {
    json!({ "family": "diffusion", "kernel": "atkinson", "strength": 0.3, "serpentine": true, "feedback": "srgb-bytes",
        "placement": { "mode": "adaptive", "radius": 6, "threshold": 12.0, "softness": 50.0 } })
}
fn everywhere() -> serde_json::Value {
    json!({ "family": "diffusion", "kernel": "atkinson", "strength": 0.6, "serpentine": true, "feedback": "srgb-bytes",
        "placement": { "mode": "everywhere" } })
}

fn run(
    rgba: &[u8],
    w: u32,
    h: u32,
    palette: &[PaletteEntry],
    dither: serde_json::Value,
) -> Vec<u8> {
    let recipe: RecipeV2 = serde_json::from_value(json!({
        "version": 2, "effects": [],
        "output": { "width": w, "height": h, "resize": { "algorithm": "area" } },
        "alpha": { "mode": "preserve", "threshold": 127.5 }, "match": "oklab-euclidean", "dither": dither,
    }))
    .unwrap();
    let indexed = process(ProcessRequestV2 {
        source: Source {
            width: w,
            height: h,
            data: rgba,
        },
        palette,
        recipe: &recipe,
    })
    .expect("process succeeds");
    let rgba = &indexed.palette.rgba;
    indexed
        .indices
        .into_vec()
        .into_iter()
        .flat_map(|i| rgba[i as usize * 4..i as usize * 4 + 4].to_vec())
        .collect()
}

fn save(dir: &Path, name: &str, rgba: &[u8], w: u32, h: u32) {
    image::RgbaImage::from_raw(w, h, rgba.to_vec())
        .unwrap()
        .save(dir.join(name))
        .unwrap();
}

fn crop<T: Clone>(values: &[T], per_pixel: u32) -> Vec<T> {
    let (x0, y0, w, h) = CROP;
    (y0..y0 + h)
        .flat_map(|y| {
            let start = ((y * WIDTH + x0) * per_pixel) as usize;
            values[start..start + (w * per_pixel) as usize].to_vec()
        })
        .collect()
}

fn main() {
    let dir = env::args().nth(1).unwrap_or("/tmp/flat-snap".into());
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();

    let full = image::open(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmark-fixtures/Celeste_box_art.png"),
    )
    .unwrap()
    .to_rgba8();
    let resized = ditherette_wasm::spec::resize::resize(
        ditherette_wasm::spec::contract::request::ResizeRequest {
            version: 1,
            source: Source {
                width: full.width(),
                height: full.height(),
                data: full.as_raw(),
            },
            output: serde_json::from_value(
                json!({ "width": WIDTH, "height": HEIGHT, "resize": { "algorithm": "area" } }),
            )
            .unwrap(),
        },
    )
    .unwrap()
    .into_vec();

    let colours: Vec<[u8; 3]> = WPLACE
        .iter()
        .map(|v| [(v >> 16) as u8, (v >> 8) as u8, *v as u8])
        .collect();
    let palette: Vec<PaletteEntry> = colours
        .iter()
        .map(|&rgb| PaletteEntry::Color { rgb })
        .collect();

    // Current palette fit over the whole image, as the app runs it.
    let dims = ImageDimensions::new(WIDTH, HEIGHT).unwrap();
    let mut image = EffectImage::from_rgba8(
        ImageBuf::from_vec_packed(resized.clone(), dims)
            .unwrap()
            .as_view(),
    );
    let context = EffectContext {
        palette: &palette,
        space: Some(WorkingSpace::Oklab),
    };
    analyze(&image, &context).apply(&mut image, 1.0);
    let fitted = image.to_rgba8().into_vec();

    let labs: Vec<Lab> = fitted
        .chunks(4)
        .map(|p| rgb8_to_oklab([p[0], p[1], p[2]]))
        .collect();
    let (labels, fills) = find_fills(&labs, WIDTH as usize);
    let is_fill: Vec<bool> = labels.iter().map(|&l| fills[l as usize]).collect();
    let mask: Vec<u8> = fitted
        .chunks(4)
        .zip(&is_fill)
        .flat_map(|(p, &f)| {
            if f {
                [p[0], p[1], p[2], 255]
            } else {
                [40, 40, 40, 255]
            }
        })
        .collect();

    let (_, _, w, h) = CROP;
    let source = crop(&fitted, 4);
    let is_fill = crop(&is_fill, 1);
    let spread = run(&source, w, h, &palette, everywhere());
    let flat = run(&source, w, h, &palette, json!({ "family": "none" }));
    // Stand-in for a fill-aware placement mask: fills take plain nearest colour, the rest dithers.
    let fills_off: Vec<u8> = spread
        .chunks(4)
        .zip(flat.chunks(4))
        .zip(&is_fill)
        .flat_map(|((d, n), &f)| if f { n.to_vec() } else { d.to_vec() })
        .collect();
    let panels: [(&str, Vec<u8>); 4] = [
        ("1-yours", run(&source, w, h, &palette, yours())),
        ("2-everywhere", spread),
        ("3-fills-undithered", fills_off),
        ("4-fill-mask", crop(&mask, 4)),
    ];
    // Zoom: sky, mountain foot, and "CEL", 3× nearest, 2×2.
    let (zx, zy, zw, zh, scale) = (20u32, 300u32, 480u32, 340u32, 3u32);
    let mut sheet = image::RgbaImage::new(zw * scale * 2, zh * scale * 2);
    for (index, (name, rgba)) in panels.into_iter().enumerate() {
        save(dir, &format!("{name}.png"), &rgba, w, h);
        let (ox, oy) = (
            (index as u32 % 2) * zw * scale,
            (index as u32 / 2) * zh * scale,
        );
        for y in 0..zh * scale {
            for x in 0..zw * scale {
                let i = (((zy + y / scale) * w + zx + x / scale) * 4) as usize;
                sheet.put_pixel(
                    ox + x,
                    oy + y,
                    image::Rgba(rgba[i..i + 4].try_into().unwrap()),
                );
            }
        }
    }
    sheet.save(dir.join("zoom.png")).unwrap();
}
