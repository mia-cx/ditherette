//! Real and synthetic RGBA8 fixture loading.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    cli::Flags,
    error::BenchError,
    util::{checksum, RGBA_CHANNELS},
};

const FIXTURE_DIR: &str = "benchmark-fixtures";

#[derive(Debug, Clone)]
pub(crate) struct Fixture {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) fingerprint: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

pub(crate) fn fixtures_from_flags(flags: &Flags) -> Result<Vec<Fixture>, BenchError> {
    let (width, height) = flags
        .optional("--size")
        .map(parse_size)
        .transpose()?
        .unwrap_or((1024, 768));

    if let Some(names) = flags.optional("--fixtures") {
        return names
            .split(',')
            .filter(|name| !name.is_empty())
            .map(|name| load_fixture(name, width, height))
            .collect();
    }

    discover_image_fixtures()?
        .into_iter()
        .map(|path| load_image_fixture(&fixture_id(&path), &path))
        .collect()
}

fn load_fixture(
    name: &str,
    synthetic_width: u32,
    synthetic_height: u32,
) -> Result<Fixture, BenchError> {
    match name {
        "solid" => Ok(synthetic_fixture(
            name,
            synthetic_width,
            synthetic_height,
            |_, _, _| [80, 120, 160, 255],
        )),
        "gradient" | "horizontal-gradient" => Ok(synthetic_fixture(
            name,
            synthetic_width,
            synthetic_height,
            |x, _, width| {
                let value = ((x * 255) / width.max(1)) as u8;
                [value, 255u8.wrapping_sub(value), value / 2, 255]
            },
        )),
        "checkerboard" => Ok(synthetic_fixture(
            name,
            synthetic_width,
            synthetic_height,
            |x, y, _| {
                if (x / 16 + y / 16) % 2 == 0 {
                    [20, 20, 20, 255]
                } else {
                    [230, 230, 230, 255]
                }
            },
        )),
        "alpha-gradient" => Ok(synthetic_fixture(
            name,
            synthetic_width,
            synthetic_height,
            |x, _, width| [200, 80, 120, ((x * 255) / width.max(1)) as u8],
        )),
        "impulse" => Ok(synthetic_fixture(
            name,
            synthetic_width,
            synthetic_height,
            |x, y, width| {
                let center_x = width / 2;
                let center_y = synthetic_height / 2;
                if x == center_x && y == center_y {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                }
            },
        )),
        "noise-seeded" => Ok(noise_fixture(
            name,
            synthetic_width,
            synthetic_height,
            0xD17E_E77E,
        )),
        image => {
            let path = resolve_fixture_path(image)?;
            load_image_fixture(&fixture_id(&path), &path)
        }
    }
}

fn discover_image_fixtures() -> Result<Vec<PathBuf>, BenchError> {
    let mut paths = fs::read_dir(FIXTURE_DIR)
        .map_err(|error| BenchError::Runtime(format!("failed to read {FIXTURE_DIR:?}: {error}")))?
        .map(|entry| entry.map(|entry| entry.path()).map_err(BenchError::io))
        .collect::<Result<Vec<_>, _>>()?;

    paths.retain(|path| path.is_file() && is_supported_image(path));
    paths.sort();

    if paths.is_empty() {
        return Err(BenchError::Runtime(format!(
            "no supported image fixtures found in {FIXTURE_DIR:?}"
        )));
    }

    Ok(paths)
}

fn resolve_fixture_path(value: &str) -> Result<PathBuf, BenchError> {
    let direct = Path::new(value);
    if direct.exists() {
        return Ok(direct.to_owned());
    }

    let in_fixture_dir = fixture_path(value);
    if in_fixture_dir.exists() {
        return Ok(in_fixture_dir);
    }

    let matches = discover_image_fixtures()?
        .into_iter()
        .filter(|path| fixture_matches(path, value))
        .collect::<Vec<_>>();

    match matches.len() {
        0 => Err(BenchError::Config(format!(
            "unknown fixture {value:?}; pass a file path or one of the files in {FIXTURE_DIR}/"
        ))),
        1 => Ok(matches[0].clone()),
        _ => Err(BenchError::Config(format!(
            "fixture name {value:?} is ambiguous: {}",
            matches
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn fixture_matches(path: &Path, value: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == value)
        || path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem == value)
}

fn fixture_path(file_name: &str) -> PathBuf {
    Path::new(FIXTURE_DIR).join(file_name)
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg"
            )
        })
        .unwrap_or(false)
}

fn fixture_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("fixture")
        .to_owned()
}

fn synthetic_fixture(
    id: &str,
    width: u32,
    height: u32,
    mut pixel: impl FnMut(u32, u32, u32) -> [u8; 4],
) -> Fixture {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * RGBA_CHANNELS);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&pixel(x, y, width));
        }
    }
    Fixture {
        id: id.to_owned(),
        kind: "synthetic".to_owned(),
        fingerprint: format!("synthetic:{id}:{width}x{height}:rgba8"),
        width,
        height,
        rgba,
    }
}

fn noise_fixture(id: &str, width: u32, height: u32, seed: u64) -> Fixture {
    let mut state = seed;
    synthetic_fixture(id, width, height, |_, _, _| {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let bytes = state.to_le_bytes();
        [bytes[0], bytes[1], bytes[2], 255]
    })
}

fn load_image_fixture(id: &str, path: &Path) -> Result<Fixture, BenchError> {
    let image = image::ImageReader::open(path)
        .map_err(|error| BenchError::Runtime(format!("failed to open fixture {path:?}: {error}")))?
        .decode()
        .map_err(|error| {
            BenchError::Runtime(format!("failed to decode fixture {path:?}: {error}"))
        })?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    Ok(Fixture {
        id: id.to_owned(),
        kind: "image".to_owned(),
        fingerprint: format!(
            "image:{}:{}:{}x{}:rgba8",
            path.display(),
            checksum(&rgba),
            width,
            height
        ),
        width,
        height,
        rgba,
    })
}

fn parse_size(value: &str) -> Result<(u32, u32), BenchError> {
    let Some((width, height)) = value.split_once('x') else {
        return Err(BenchError::Config(format!(
            "invalid size {value:?}, expected WxH"
        )));
    };
    Ok((
        width
            .parse()
            .map_err(|error| BenchError::Config(format!("invalid width {width:?}: {error}")))?,
        height
            .parse()
            .map_err(|error| BenchError::Config(format!("invalid height {height:?}: {error}")))?,
    ))
}
