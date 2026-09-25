//! Public effect operations: standalone `apply_effects` and recipe-v2 `process`.
//!
//! `process` is exactly `apply_effects` followed by the frozen v1 `process`
//! with the recipe's terminal settings. No other composition exists.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    image::{
        contracts::{IndexedImage, PaletteEntry, Rgba8Image},
        ImageView,
    },
    spec::{
        contract::{
            error::{DitheretteError, ErrorCode},
            request::{
                validate_dimensions, AlphaPolicy, DitherPolicy, MatchPolicy, Output,
                ProcessRequest, RecipeV1, Request, Source, MAX_SOURCE_SIDE, RECIPE_VERSION,
            },
        },
        pipeline,
    },
};

use super::{
    chain::{apply_chain, validate_chain, EffectContext},
    image::EffectImage,
    recipe::{decode_steps, EffectStep},
};

/// Version of the standalone `apply_effects` request shape.
pub const EFFECTS_VERSION: u32 = 1;
/// The recipe version that adds `effects` to the v1 terminal settings.
pub const RECIPE_V2: u32 = 2;

/// Standalone chain request. The source is borrowed and never modified.
#[derive(Debug, Clone, Copy)]
pub struct EffectsRequest<'a> {
    pub version: u32,
    pub source: Source<'a>,
    pub effects: &'a [EffectStep],
    pub context: EffectContext<'a>,
}

/// Applies the ordered chain and returns owned full-colour RGBA8 at the source size.
pub fn apply_effects(request: EffectsRequest<'_>) -> Result<Rgba8Image, DitheretteError> {
    if request.version != EFFECTS_VERSION {
        return Err(unsupported("version"));
    }
    validate_chain(request.effects, &request.context)?;
    let source = source_view(request.source)?;
    let mut image = EffectImage::from_rgba8(source);
    apply_chain(&mut image, request.effects, &request.context)?;
    Ok(image.to_rgba8())
}

/// Recipe v1's terminal settings plus the ordered effects that precede them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeV2 {
    pub version: u32,
    pub effects: Vec<EffectStep>,
    pub output: Output,
    pub alpha: AlphaPolicy,
    #[serde(rename = "match")]
    pub matching: MatchPolicy,
    pub dither: DitherPolicy,
}

impl RecipeV2 {
    /// The terminal resize, alpha, matching, and dither settings as a v1 recipe.
    pub fn terminal(&self) -> RecipeV1 {
        RecipeV1 {
            version: RECIPE_VERSION,
            output: self.output,
            alpha: self.alpha,
            matching: self.matching,
            dither: self.dither,
        }
    }
}

/// End-to-end request: effects, then resize, then dither and quantize.
#[derive(Debug, Clone, Copy)]
pub struct ProcessRequestV2<'a> {
    pub source: Source<'a>,
    pub palette: &'a [PaletteEntry],
    pub recipe: &'a RecipeV2,
}

/// Validates everything, applies effects, then runs the frozen v1 `process`.
/// The effects context is this request's palette and the matching working space.
pub fn process(request: ProcessRequestV2<'_>) -> Result<IndexedImage, DitheretteError> {
    let recipe = request.recipe;
    if recipe.version != RECIPE_V2 {
        return Err(unsupported("recipe.version"));
    }
    let context = EffectContext {
        palette: request.palette,
        space: Some(recipe.matching.space()),
    };
    validate_chain(&recipe.effects, &context).map_err(in_recipe)?;
    let terminal = ProcessRequest {
        source: request.source,
        palette: request.palette,
        recipe: recipe.terminal(),
    };
    Request::Process(terminal).validate()?;
    let effected = apply_effects(EffectsRequest {
        version: EFFECTS_VERSION,
        source: request.source,
        effects: &recipe.effects,
        context,
    })
    .map_err(in_recipe)?;
    pipeline::process(ProcessRequest {
        source: Source {
            width: effected.dimensions().width(),
            height: effected.dimensions().height(),
            data: effected.data(),
        },
        ..terminal
    })
}

/// Decodes a recipe-v2 JSON object. Effect decoding errors name `recipe.effects.i`.
pub fn decode_recipe_v2(json: &str) -> Result<RecipeV2, DitheretteError> {
    let value: Value = serde_json::from_str(json).map_err(|_| malformed())?;
    if let Some(effects) = value.get("effects") {
        decode_steps(effects.clone(), "recipe.effects")?;
    }
    serde_json::from_value(value).map_err(|_| malformed())
}

fn source_view(source: Source<'_>) -> Result<ImageView<'_, crate::image::Rgba8>, DitheretteError> {
    let dimensions = validate_dimensions(
        source.width,
        source.height,
        MAX_SOURCE_SIDE,
        "source",
        ErrorCode::InvalidImage,
    )?;
    ImageView::packed(source.data, dimensions).map_err(|_| {
        DitheretteError::new(
            ErrorCode::InvalidImage,
            "source.data",
            "RGBA8 byte length does not match dimensions.",
        )
    })
}

/// Effect paths inside `process` live under `recipe`; context comes from the palette and match.
fn in_recipe(mut error: DitheretteError) -> DitheretteError {
    error.path = match error.path.as_str() {
        "context.palette" => "palette".into(),
        "context.space" => "recipe.match".into(),
        path if path.starts_with("effects") => format!("recipe.{path}"),
        path => path.into(),
    };
    error
}

fn unsupported(path: &str) -> DitheretteError {
    DitheretteError::new(
        ErrorCode::InvalidRequest,
        path,
        "Unsupported request version.",
    )
}

fn malformed() -> DitheretteError {
    DitheretteError::new(
        ErrorCode::InvalidSettings,
        "recipe",
        "Malformed recipe or unsupported setting tag.",
    )
}
