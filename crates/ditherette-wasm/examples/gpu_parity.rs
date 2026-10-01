//! Runs one stage on the CPU and the GPU and reports how far apart they are.
//!
//! ```sh
//! cargo run --release --features gpu --example gpu_parity [WIDTH HEIGHT [RAW_RGBA_FILE]]
//! ```
//!
//! Without a file it uses a synthetic image. Make a raw file from a photo with
//! `ffmpeg -i photo.jpg -f rawvideo -pix_fmt rgba photo.rgba`. Software adapters count here.

use ditherette_wasm::{
    image::ImageDimensions,
    prod::{
        effects::{
            channel::Channel,
            levels::{Levels, Points},
            table::ChannelTables,
        },
        gpu::{Adapters, Backend, ChannelLookup, Gpu},
    },
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |index: usize, default: u32| {
        args.get(index).map_or(default, |arg| {
            arg.parse().expect("width and height are numbers")
        })
    };
    let (width, height) = (number(0, 6000), number(1, 4000));
    let dimensions = ImageDimensions::new(width, height).expect("valid dimensions");
    let rgba = match args.get(2) {
        Some(file) => std::fs::read(file).expect("raw RGBA file is readable"),
        None => synthetic(width, height),
    };
    assert_eq!(
        rgba.len(),
        width as usize * height as usize * 4,
        "raw file size"
    );

    let levels = Levels {
        channel: Channel::Rgb,
        input: Points {
            black: 0.05,
            white: 0.9,
        },
        gamma: 1.3,
        output: Points {
            black: 0.02,
            white: 1.0,
        },
    };
    let stage = ChannelLookup(ChannelTables::new([&levels]).bytes());

    let gpu = match pollster::block_on(Backend::detect_on(&Gpu::instance(), Adapters::Any)) {
        Backend::Gpu(gpu) => gpu,
        Backend::Cpu(reason) => {
            println!("no GPU comparison: the CPU would run ({reason:?})");
            return;
        }
    };
    let info = gpu.info();
    println!(
        "adapter: {} ({:?}, {:?})",
        info.name, info.backend, info.device_type
    );
    match pollster::block_on(gpu.parity(&stage, dimensions, &rgba)) {
        Ok(parity) => println!(
            "channel lookup, {width}×{height}: largest byte difference {}, {} of {} pixels differ",
            parity.max_byte_difference, parity.differing_pixels, parity.pixels
        ),
        Err(reason) => println!("GPU call failed, the CPU would run ({reason:?})"),
    }
}

/// Gradients and a checker, with every byte value in every channel.
fn synthetic(width: u32, height: u32) -> Vec<u8> {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .flat_map(|(x, y)| [x, y, x ^ y, x.wrapping_add(y)].map(|v| v as u8))
        .collect()
}
