//! Rectangular ordered threshold tiles and their separable composition.
//!
//! A tile is a fixed rank table. It thresholds exactly like v1 Bayer: the centred
//! rank feeds the shared v1 perturbation, then the perturbed RGBA8 is quantized.

use serde::{Deserialize, Serialize};

use crate::{
    image::contracts::{IndexedImage, Rgba8Image},
    spec::{
        contract::{
            error::DitheretteError,
            request::{
                BayerSize, DitherPolicy, DitherQuantizeRequest, Field, PerturbPolicy,
                PerturbRequest, Placement, QuantizeRequest, Request, Source, WorkingSpace,
                RECIPE_VERSION,
            },
        },
        dither::perturb::perturb_by_field_rows_into,
        quantize,
        tiling::contract::RowBand,
    },
};

/// Named rectangular tiles. Tags are `<width>x<height>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tile {
    #[serde(rename = "3x1")]
    ThreeByOne,
    #[serde(rename = "4x1")]
    FourByOne,
    #[serde(rename = "4x2")]
    FourByTwo,
    #[serde(rename = "5x3")]
    FiveByThree,
}

impl Tile {
    pub const fn width(self) -> usize {
        match self {
            Self::ThreeByOne => 3,
            Self::FourByOne | Self::FourByTwo => 4,
            Self::FiveByThree => 5,
        }
    }

    pub const fn height(self) -> usize {
        match self {
            Self::ThreeByOne | Self::FourByOne => 1,
            Self::FourByTwo => 2,
            Self::FiveByThree => 3,
        }
    }

    /// Row-major ranks `0..width*height`, each exactly once. See `ordered.md` for sources.
    pub const fn ranks(self) -> &'static [u8] {
        match self {
            Self::ThreeByOne => &[0, 2, 1],
            Self::FourByOne => &[0, 2, 1, 3],
            Self::FourByTwo => &[
                0, 4, 2, 6, //
                3, 7, 1, 5,
            ],
            Self::FiveByThree => &[
                0, 12, 7, 3, 9, //
                14, 8, 1, 5, 11, //
                6, 4, 10, 13, 2,
            ],
        }
    }
}

/// Centred threshold at global image coordinates, using the v1 Bayer formula.
/// For n cells and rank r it is `(r+0.5)/n-0.5`, so no cell reaches either end of [-0.5,0.5].
pub fn tile_noise_at(x: u32, y: u32, tile: Tile) -> f32 {
    let (width, height) = (tile.width(), tile.height());
    let rank = tile.ranks()[(y as usize % height) * width + x as usize % width];
    (f32::from(rank) + 0.5) / (width * height) as f32 - 0.5
}

/// The v1 separable controls with a rectangular tile in place of the field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TilePerturbPolicy {
    pub tile: Tile,
    pub space: WorkingSpace,
    pub strength: f32,
    pub placement: Placement,
}

impl TilePerturbPolicy {
    /// A v1 policy with identical controls, used only for validation.
    /// v1 validation never reads the field, so any v1 field checks the other controls.
    fn as_v1(self) -> PerturbPolicy {
        PerturbPolicy {
            field: Field::Bayer {
                size: BayerSize::Two,
            },
            space: self.space,
            strength: self.strength,
            placement: self.placement,
        }
    }
}

/// Validates like v1 `perturb` and returns durable perturbed RGBA8; alpha passes through.
pub fn perturb(
    source: Source<'_>,
    policy: TilePerturbPolicy,
) -> Result<Rgba8Image, DitheretteError> {
    let layout = Request::Perturb(PerturbRequest {
        version: RECIPE_VERSION,
        source,
        perturb: policy.as_v1(),
    })
    .validate()?;
    let mut output = Rgba8Image::new_packed(layout.output)
        .expect("validated output dimensions fit RGBA8 storage");
    let rows = RowBand::new(0, layout.output.height()).expect("image height is nonzero");
    perturb_by_field_rows_into(
        layout.source,
        output.as_view_mut(),
        policy.space,
        policy.strength,
        policy.placement,
        rows,
        |x, y, _| tile_noise_at(x, y, policy.tile),
    );
    Ok(output)
}

/// Perturbs into RGBA8, then runs v1 `quantize` on those bytes, exactly like a v1 separable field.
pub fn dither_and_quantize(
    request: QuantizeRequest<'_>,
    policy: TilePerturbPolicy,
) -> Result<IndexedImage, DitheretteError> {
    Request::DitherAndQuantize(DitherQuantizeRequest {
        quantize: request,
        dither: DitherPolicy::Separable {
            perturb: policy.as_v1(),
        },
    })
    .validate()?;
    let perturbed = perturb(request.source, policy)?;
    quantize::quantize(QuantizeRequest {
        source: Source {
            width: perturbed.dimensions().width(),
            height: perturbed.dimensions().height(),
            data: perturbed.data(),
        },
        ..request
    })
}
