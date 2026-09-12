use std::{env, fs, path::PathBuf, time::Instant};

use ditherette_wasm_old::{
    image::{rgba, ImageDimensions},
    resize::{
        bilinear::resize_rgba_bilinear_2_into, resize_rgba_area_into, resize_rgba_bilinear_into,
        resize_rgba_nearest_into,
    },
};
use image::ImageReader;

fn main() {
    let args = Args::parse();
    let fixture = load_fixture(args.fixture.as_deref());
    let output_dimensions = ImageDimensions::new(
        ((fixture.dimensions.width() as f64) * 0.5).round().max(1.0) as u32,
        ((fixture.dimensions.height() as f64) * 0.5)
            .round()
            .max(1.0) as u32,
    )
    .unwrap();
    let mut output = vec![0; rgba::checked_rgba_byte_len(output_dimensions).unwrap()];

    for _ in 0..args.warmup {
        run_once(&args, &fixture, output_dimensions, &mut output);
    }

    let start = Instant::now();
    for _ in 0..args.iterations {
        run_once(&args, &fixture, output_dimensions, &mut output);
    }
    let elapsed = start.elapsed();
    let nanos_per_iter = elapsed.as_nanos() as f64 / args.iterations as f64;

    println!(
        "{}:{} {:.3} ms/iter checksum={}",
        args.filter,
        args.implementation,
        nanos_per_iter / 1_000_000.0,
        checksum(&output)
    );

    if let Some(path) = args.save_baseline_path() {
        fs::write(path, format!("{nanos_per_iter}\n")).unwrap();
    }
    if let Some(path) = args.baseline_path() {
        let baseline: f64 = fs::read_to_string(path).unwrap().trim().parse().unwrap();
        println!(
            "comparison: {:.2}x baseline ({:+.1}%)",
            nanos_per_iter / baseline,
            ((nanos_per_iter / baseline) - 1.0) * 100.0
        );
    }
}

fn run_once(args: &Args, fixture: &Fixture, output_dimensions: ImageDimensions, output: &mut [u8]) {
    match (args.filter.as_str(), args.implementation.as_str()) {
        ("nearest", "scalar") => {
            resize_rgba_nearest_into(&fixture.rgba, fixture.dimensions, output_dimensions, output)
        }
        ("area", "scalar") => {
            resize_rgba_area_into(&fixture.rgba, fixture.dimensions, output_dimensions, output)
        }
        ("bilinear", "scalar") => {
            resize_rgba_bilinear_into(&fixture.rgba, fixture.dimensions, output_dimensions, output)
        }
        ("bilinear", "scalar_2") | ("bilinear_2", "scalar") => resize_rgba_bilinear_2_into(
            &fixture.rgba,
            fixture.dimensions,
            output_dimensions,
            output,
        ),
        _ => panic!(
            "unsupported quick benchmark target {}:{}",
            args.filter, args.implementation
        ),
    }
    .unwrap();
}

fn load_fixture(requested: Option<&str>) -> Fixture {
    let path = requested
        .map(PathBuf::from)
        .unwrap_or_else(default_fixture_path);
    let image = ImageReader::open(&path)
        .unwrap_or_else(|error| panic!("failed to open {}: {error}", path.display()))
        .decode()
        .unwrap_or_else(|error| panic!("failed to decode {}: {error}", path.display()))
        .to_rgba8();
    Fixture {
        dimensions: ImageDimensions::new(image.width(), image.height()).unwrap(),
        rgba: image.into_raw(),
    }
}

fn default_fixture_path() -> PathBuf {
    repo_root().join("benchmark-fixtures/Celeste_box_art.png")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .unwrap()
        .to_path_buf()
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().map(|byte| u64::from(*byte)).sum()
}

struct Fixture {
    dimensions: ImageDimensions,
    rgba: Vec<u8>,
}

struct Args {
    filter: String,
    implementation: String,
    iterations: usize,
    warmup: usize,
    fixture: Option<String>,
    save_baseline: Option<String>,
    baseline: Option<String>,
    compare_id: String,
}

impl Args {
    fn parse() -> Self {
        let mut args = env::args().skip(1);
        let mut parsed = Self {
            filter: "bilinear".to_owned(),
            implementation: "scalar".to_owned(),
            iterations: 30,
            warmup: 3,
            fixture: None,
            save_baseline: None,
            baseline: None,
            compare_id: "quick".to_owned(),
        };
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--filter" => parsed.filter = args.next().unwrap(),
                "--implementation" => parsed.implementation = args.next().unwrap(),
                "--iterations" => parsed.iterations = args.next().unwrap().parse().unwrap(),
                "--warmup" => parsed.warmup = args.next().unwrap().parse().unwrap(),
                "--fixture" => parsed.fixture = Some(args.next().unwrap()),
                "--save-baseline" => parsed.save_baseline = Some(args.next().unwrap()),
                "--baseline" => parsed.baseline = Some(args.next().unwrap()),
                "--compare-id" => parsed.compare_id = args.next().unwrap(),
                _ => panic!("unknown argument {arg}"),
            }
        }
        parsed
    }

    fn save_baseline_path(&self) -> Option<PathBuf> {
        self.save_baseline.as_ref().map(|name| baseline_path(name))
    }

    fn baseline_path(&self) -> Option<PathBuf> {
        self.baseline.as_ref().map(|name| baseline_path(name))
    }
}

fn baseline_path(name: &str) -> PathBuf {
    let path = repo_root().join("benchmark-results/quick-baselines");
    fs::create_dir_all(&path).unwrap();
    path.join(format!("{name}.txt"))
}
