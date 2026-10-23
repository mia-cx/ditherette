//! Continuous pointwise effects. Only bounded control data is prepared on CPU;
//! the shader evaluates every image pixel, including masks and recipe curves.

use super::{Backend, Fallback, GpuTiling, LookupMemory, Stage};
use crate::prod::contract::request::WorkingSpace;
use crate::{
    image::ImageDimensions,
    prod::effects::{
        chain::{validate_chain, Effect},
        channel::Channel,
        curves::{ChannelKind, ColourChannel, Curve, CurveGrid, CurveKind},
        mask::MaskCurve,
        model::ColourModel,
        operation::{apply_in_place, enabled_steps, tabulates},
        recipe::BuiltinEffect,
        table::ChannelTables,
        EffectContext, EffectStep,
    },
};

mod shader;

/// Control-data bytecode plus its CPU reference. Image colours are never tabulated
/// beyond the same 768 leading channel values used by the CPU executor.
pub struct Pointwise {
    words: Vec<u32>,
    steps: Vec<EffectStep>,
    space: Option<WorkingSpace>,
    mask: Option<Vec<MaskCurve>>,
}

impl Pointwise {
    /// Automatic analyses are separate image operations, not pointwise maps.
    pub fn supported(steps: &[EffectStep]) -> bool {
        steps
            .iter()
            .filter(|step| step.enabled)
            .all(|step| step.effect.pointwise() && !step.effect.needs().palette)
    }

    /// Must be admitted before decoding or preparing any control vectors.
    /// The extra reserve covers bytecode, its wire/GPU/staging copies and the
    /// retained decoded recipe. No additional full-image carrier is allocated.
    pub fn memory(dimensions: ImageDimensions, json_bytes: u64) -> LookupMemory {
        let mut memory = LookupMemory::new(dimensions);
        // Leading float channel tables exist even for an empty effect prefix.
        memory.required_bytes +=
            32 * 1024 + (LookupMemory::DECODER_BYTES_PER_JSON_BYTE + 32) * json_bytes;
        memory
    }

    /// Validates and owns a resolved chain; `space` supplies explicit recolour's
    /// context. Preparation visits control data only, never source pixels.
    pub fn new(steps: Vec<EffectStep>, space: Option<WorkingSpace>) -> Result<Self, Fallback> {
        validate_chain(
            &steps,
            &EffectContext {
                space,
                ..Default::default()
            },
        )
        .map_err(|error| Fallback::Failed(error.to_string()))?;
        if !Self::supported(&steps) {
            return Err(Fallback::UnsupportedStage);
        }
        let mut enabled = Vec::new();
        enabled
            .try_reserve_exact(steps.len())
            .map_err(|_| Fallback::MemoryLimit)?;
        enabled.extend(enabled_steps(&steps));
        let leading = enabled.iter().take_while(|step| tabulates(step)).count();
        let tables = ChannelTables::new(enabled[..leading].iter().map(|step| step.effect));
        let tail = &enabled[leading..];
        let linear = tail.first().is_some_and(|step| {
            matches!(step.effect, BuiltinEffect::HueSaturation(effect)
                if effect.hue != 0.0 || effect.saturation != 0.0 || effect.lightness != 0.0)
                && crate::prod::effects::mask::PreparedMask::new(step.mask).is_full()
        });
        // Header: count, first step, linear-leading flag, 768 float values.
        let mut code = Code(Vec::new());
        code.0
            .try_reserve_exact(Code::required(&steps))
            .map_err(|_| Fallback::MemoryLimit)?;
        code.0.extend([tail.len() as u32, 0, u32::from(linear)]);
        if linear {
            code.0
                .extend(tables.linear().iter().flatten().map(|x| x.to_bits()));
        } else {
            for channel in 0..3 {
                for byte in 0..=255 {
                    code.0.push(tables.unit(channel, byte).to_bits());
                }
            }
        }
        let mut previous = 1;
        for step in tail {
            let record = code.0.len();
            code.0[previous] = record as u32;
            code.0.extend([0; 5]); // next, operation, argument offset, mask count, mask offsets
            let masks = code.masks(step.mask);
            code.0[record + 3] = masks.0;
            code.0[record + 4] = masks.1;
            let (tag, args) = code.effect(step.effect);
            code.0[record + 1] = tag;
            code.0[record + 2] = args;
            previous = record;
        }
        Ok(Self {
            words: code.0,
            steps,
            space,
            mask: None,
        })
    }

    /// Evaluate a following step's mask from the continuous carrier, before the
    /// effect-chain byte boundary. Empty masks produce white, preserving alpha.
    pub fn with_mask(mut self, mask: Vec<MaskCurve>) -> Result<Self, Fallback> {
        crate::prod::effects::mask::validate(&mask, "")
            .map_err(|e| Fallback::Failed(e.to_string()))?;
        let extra = 9 + mask
            .iter()
            .map(|m| match m {
                MaskCurve::OneInput(c) => 16 + 2 * c.points.len(),
                MaskCurve::TwoInput(c) => {
                    16 + c.grid.columns.len()
                        + c.grid.rows.len()
                        + c.grid.values.iter().map(Vec::len).sum::<usize>()
                }
            })
            .sum::<usize>();
        let mut code = Code(std::mem::take(&mut self.words));
        code.0
            .try_reserve_exact(extra)
            .map_err(|_| Fallback::MemoryLimit)?;
        let mut previous = 1;
        while code.0[previous] != 0 {
            previous = code.0[previous] as usize;
        }
        let (count, masks) = code.masks(&mask);
        let record = code.0.len() as u32;
        code.0.extend([0, 8, 0, count, masks]);
        code.0[previous] = record;
        code.0[0] += 1;
        self.words = code.0;
        self.mask = Some(mask);
        Ok(self)
    }
}

impl Backend {
    /// Maps a resolved pointwise chain on GPU without intermediate byte rounding.
    pub async fn pointwise(
        &mut self,
        stage: &Pointwise,
        dimensions: ImageDimensions,
        rgba: &[u8],
        memory_limit: u64,
    ) -> Result<Vec<u8>, Fallback> {
        self.pointwise_tiled(stage, dimensions, rgba, GpuTiling::Disabled, memory_limit)
            .await
    }

    /// Tiling changes transport only; each shader pixel retains the full chain.
    pub async fn pointwise_tiled(
        &mut self,
        stage: &Pointwise,
        dimensions: ImageDimensions,
        rgba: &[u8],
        tiling: GpuTiling,
        memory_limit: u64,
    ) -> Result<Vec<u8>, Fallback> {
        let mut memory = LookupMemory::with_tiling(dimensions, tiling);
        // Retained words, wire bytes, GPU storage and queue staging coexist.
        memory.required_bytes += 4 * stage.words.capacity() as u64 * 4 + stage.recipe_bytes();
        memory.check(memory_limit)?;
        let result = match self {
            Self::Cpu(reason) => Err(reason.clone()),
            Self::Gpu(gpu) => gpu.process_tiled(stage, dimensions, rgba, tiling).await,
        };
        self.retire_failure(&result);
        result
    }
}

impl Stage for Pointwise {
    const WGSL: &'static str = shader::WGSL;
    fn params(&self) -> Vec<u8> {
        self.words.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
    fn cpu(&self, rgba: &mut [u8]) {
        let dimensions = ImageDimensions::new((rgba.len() / 4) as u32, 1).unwrap();
        if let Some(mask) = &self.mask {
            crate::prod::effects::write_mask(
                &rgba.to_vec(),
                dimensions,
                &self.steps,
                mask,
                &EffectContext {
                    space: self.space,
                    ..Default::default()
                },
                rgba,
            )
            .unwrap();
            return;
        }
        apply_in_place(
            rgba,
            dimensions,
            &self.steps,
            &EffectContext {
                space: self.space,
                ..Default::default()
            },
        )
        .unwrap();
    }
}

impl Pointwise {
    fn recipe_bytes(&self) -> u64 {
        fn grid(grid: &CurveGrid) -> usize {
            (grid.columns.capacity() + grid.rows.capacity()) * 4
                + grid.values.capacity() * std::mem::size_of::<Vec<f32>>()
                + grid
                    .values
                    .iter()
                    .map(|row| row.capacity() * 4)
                    .sum::<usize>()
        }
        fn curves(curves: &Vec<Curve>) -> usize {
            curves.capacity() * std::mem::size_of::<Curve>()
                + curves
                    .iter()
                    .map(|curve| match curve {
                        Curve::OneInput(c) => c.points.capacity() * 8,
                        Curve::TwoInput(c) => grid(&c.grid),
                    })
                    .sum::<usize>()
        }
        (self
            .mask
            .as_ref()
            .map(|mask| {
                mask.capacity() * std::mem::size_of::<MaskCurve>()
                    + mask
                        .iter()
                        .map(|m| match m {
                            MaskCurve::OneInput(c) => c.points.capacity() * 8,
                            MaskCurve::TwoInput(c) => grid(&c.grid),
                        })
                        .sum::<usize>()
            })
            .unwrap_or(0)
            + self.steps.capacity() * std::mem::size_of::<EffectStep>()
            + self
                .steps
                .iter()
                .map(|step| {
                    step.mask.capacity() * std::mem::size_of::<MaskCurve>()
                        + step
                            .mask
                            .iter()
                            .map(|mask| match mask {
                                MaskCurve::OneInput(c) => c.points.capacity() * 8,
                                MaskCurve::TwoInput(c) => grid(&c.grid),
                            })
                            .sum::<usize>()
                        + match &step.effect {
                            BuiltinEffect::Curves(c) => curves(&c.curves),
                            BuiltinEffect::PaletteFit(c) => {
                                c.curves.as_ref().map(curves).unwrap_or(0)
                            }
                            BuiltinEffect::Recolour(c) => c
                                .recipe
                                .as_ref()
                                .map(|r| {
                                    r.tone.capacity() * 8
                                        + r.groups.capacity()
                                            * std::mem::size_of::<
                                                crate::prod::effects::recolour::Group,
                                            >()
                                })
                                .unwrap_or(0),
                            _ => 0,
                        }
                })
                .sum::<usize>()) as u64
    }
}

struct Code(Vec<u32>);
impl Code {
    /// Reserve control storage once, including disabled/neutral curves as a
    /// conservative bound. No shader-plan push may grow the vector afterward.
    fn required(steps: &[EffectStep]) -> usize {
        fn grid(grid: &CurveGrid) -> usize {
            16 + grid.columns.len()
                + grid.rows.len()
                + grid.values.iter().map(Vec::len).sum::<usize>()
        }
        fn curves(curves: &[Curve]) -> usize {
            curves
                .iter()
                .map(|c| {
                    6 + match c {
                        Curve::OneInput(c) => 16 + 2 * c.points.len(),
                        Curve::TwoInput(c) => grid(&c.grid),
                    }
                })
                .sum()
        }
        771 + steps
            .iter()
            .filter(|step| step.enabled)
            .map(|step| {
                32 + step
                    .mask
                    .iter()
                    .map(|mask| {
                        5 + match mask {
                            MaskCurve::OneInput(c) => 16 + 2 * c.points.len(),
                            MaskCurve::TwoInput(c) => grid(&c.grid),
                        }
                    })
                    .sum::<usize>()
                    + match &step.effect {
                        BuiltinEffect::Curves(c) => curves(&c.curves),
                        BuiltinEffect::PaletteFit(c) => curves(c.curves.as_deref().unwrap_or(&[])),
                        BuiltinEffect::Recolour(c) => c
                            .recipe
                            .as_ref()
                            .map(|r| 16 + 2 * r.tone.len() + 4 * r.groups.len())
                            .unwrap_or(0),
                        _ => 0,
                    }
            })
            .sum::<usize>()
    }
    fn floats(&mut self, values: impl IntoIterator<Item = f32>) -> u32 {
        let start = self.0.len() as u32;
        self.0.extend(values.into_iter().map(f32::to_bits));
        start
    }
    fn channel(channel: ColourChannel) -> u32 {
        let resolved = channel.resolve().expect("validated colour channel");
        let model = match resolved.model {
            ColourModel::Srgb => 0,
            ColourModel::LinearRgb => 1,
            ColourModel::Hsl => 2,
            ColourModel::Hsv => 3,
            ColourModel::Oklab => 4,
            ColourModel::Oklch => 5,
            ColourModel::Cielab => 6,
            ColourModel::Cielch => 7,
            ColourModel::Ycbcr => 8,
        };
        let kind = match resolved.kind {
            ChannelKind::Other => 0,
            ChannelKind::Hue => 1,
            ChannelKind::Chroma => 2,
        };
        model | ((resolved.index as u32) << 4) | (kind << 6)
    }
    fn points(&mut self, points: &[[f32; 2]], periodic: bool) -> u32 {
        let count = points.len() - usize::from(periodic);
        let positions = self.floats(points[..count].iter().map(|p| p[0]));
        let values = self.floats(points[..count].iter().map(|p| p[1]));
        let start = self.0.len() as u32;
        self.0.extend([
            positions,
            values,
            count as u32,
            u32::from(periodic),
            0,
            0,
            0,
            0,
        ]);
        start
    }
    fn grid(&mut self, grid: &CurveGrid, cyclic_x: bool, cyclic_y: bool) -> u32 {
        let x = self.floats(grid.columns.iter().copied());
        let y = self.floats(grid.rows.iter().copied());
        let values = self.floats(grid.values.iter().flatten().copied());
        let start = self.0.len() as u32;
        self.0.extend([
            x,
            values,
            grid.columns.len() as u32,
            u32::from(cyclic_x),
            y,
            grid.rows.len() as u32,
            u32::from(cyclic_y),
            1,
        ]);
        start
    }
    fn curve(&mut self, curve: &Curve) -> Option<u32> {
        let (flags, x, y, x2, data) = match curve {
            Curve::OneInput(c) => {
                if c.kind == CurveKind::Adjust && c.points.iter().all(|p| p[1] == 0.5) {
                    return None;
                }
                let x = Self::channel(c.x);
                let y = Self::channel(c.y);
                let adjust = c.kind == CurveKind::Adjust;
                let identity = !adjust && c.points == [[0.0, 0.0], [1.0, 1.0]];
                let data = self.points(&c.points, adjust && x >> 6 == 1);
                (
                    u32::from(adjust) | (u32::from(identity) << 2),
                    x,
                    y,
                    0,
                    data,
                )
            }
            Curve::TwoInput(c) => {
                if c.grid.values.iter().flatten().all(|x| *x == 0.5) {
                    return None;
                }
                let x = Self::channel(c.x);
                let x2 = Self::channel(c.x2);
                let data = self.grid(&c.grid, x >> 6 == 1, x2 >> 6 == 1);
                (3, x, Self::channel(c.y), x2, data)
            }
        };
        let start = self.0.len() as u32;
        self.0.extend([flags, x, y, x2, data]);
        Some(start)
    }
    fn masks(&mut self, masks: &[MaskCurve]) -> (u32, u32) {
        let mut offsets = [0; crate::prod::effects::mask::MAX_MASK_CURVES];
        let mut count = 0;
        for mask in masks {
            let (x, x2, data, grid) = match mask {
                MaskCurve::OneInput(c) => {
                    if c.points.iter().all(|p| p[1] == 1.0) {
                        continue;
                    }
                    let x = Self::channel(c.x);
                    (x, 0, self.points(&c.points, x >> 6 == 1), 0)
                }
                MaskCurve::TwoInput(c) => {
                    if c.grid.values.iter().flatten().all(|x| *x == 1.0) {
                        continue;
                    }
                    let x = Self::channel(c.x);
                    let x2 = Self::channel(c.x2);
                    (x, x2, self.grid(&c.grid, x >> 6 == 1, x2 >> 6 == 1), 1)
                }
            };
            offsets[count] = self.0.len() as u32;
            count += 1;
            self.0.extend([x, x2, data, grid]);
        }
        let start = self.0.len() as u32;
        self.0.extend_from_slice(&offsets[..count]);
        (count as u32, start)
    }
    fn curve_list(&mut self, curves: &[Curve], strength: f32) -> u32 {
        let mut offsets = [0; crate::prod::effects::curves::MAX_CURVES];
        let mut count = 0;
        for curve in curves {
            if let Some(offset) = self.curve(curve) {
                offsets[count] = offset;
                count += 1;
            }
        }
        let start = self.0.len() as u32;
        self.0.extend([strength.to_bits(), count as u32]);
        self.0.extend_from_slice(&offsets[..count]);
        start
    }
    fn effect(&mut self, effect: &BuiltinEffect) -> (u32, u32) {
        match effect {
            BuiltinEffect::Levels(c) => {
                let channels = match c.channel {
                    Channel::Rgb => 7,
                    Channel::Red => 1,
                    Channel::Green => 2,
                    Channel::Blue => 4,
                };
                let ptr = self.floats([
                    if c.input.black == 0.0 {
                        0.0
                    } else {
                        c.input.black
                    },
                    c.input.white,
                    c.gamma,
                    c.output.black,
                    c.output.white,
                ]);
                self.0.push(channels);
                // Normalize subnormal-width intervals before shader subtraction.
                // Endpoint bits stay separate so denormal flushing cannot turn
                // black into white or make an out-of-range quotient overflow.
                let scale = if c.input.white - c.input.black < f32::MIN_POSITIVE {
                    f32::from_bits(0x7e800000) // 2^126
                } else {
                    1.0
                };
                self.floats([
                    (f64::from(c.input.black) * f64::from(scale)) as f32,
                    (f64::from(c.input.white) * f64::from(scale)) as f32,
                    scale,
                ]);
                (0, ptr)
            }
            BuiltinEffect::BrightnessContrast(c) => {
                (1, self.floats([c.brightness, 4f32.powf(c.contrast)]))
            }
            BuiltinEffect::Exposure(c) => (2, self.floats([2f32.powf(c.stops)])),
            BuiltinEffect::WhiteBalance(c) => (3, self.floats(c.gains())),
            BuiltinEffect::HueSaturation(c) => {
                let (sin, cos) = c.hue.to_radians().sin_cos();
                (4, self.floats([sin, cos, 1.0 + c.saturation, c.lightness]))
            }
            BuiltinEffect::Curves(c) => (5, self.curve_list(&c.curves, 1.0)),
            BuiltinEffect::PaletteFit(c) => (
                5,
                self.curve_list(c.curves.as_deref().unwrap_or(&[]), c.strength),
            ),
            BuiltinEffect::Recolour(c) => {
                let Some(recipe) = c
                    .recipe
                    .as_ref()
                    .filter(|r| c.strength != 0.0 && !r.is_identity())
                else {
                    return (7, 0);
                };
                let tone = self.points(&recipe.tone, false);
                let groups = self.floats(
                    recipe
                        .groups
                        .iter()
                        .flat_map(|g| [g.hue, g.width, g.turn, g.chroma]),
                );
                let args = self.0.len() as u32;
                self.0.extend([
                    c.strength.to_bits(),
                    recipe.chroma.to_bits(),
                    recipe.shift[0].to_bits(),
                    recipe.shift[1].to_bits(),
                    tone,
                    groups,
                    recipe.groups.len() as u32,
                    match recipe.space {
                        WorkingSpace::Srgb | WorkingSpace::Ycbcr => 0,
                        WorkingSpace::LinearRgb => 1,
                        WorkingSpace::Oklab | WorkingSpace::Oklch => 4,
                        WorkingSpace::Cielab | WorkingSpace::Cielch => 6,
                    },
                ]);
                (6, args)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pollster::block_on;
    use serde_json::{json, Value};

    /// Proves masks, continuous step sequencing, every curve model and explicit
    /// palette recipes reach the real GPU while retaining source alpha exactly.
    #[test]
    fn pointwise_effects_match_continuous_cpu_chains() {
        let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
            return;
        };
        let dimensions = ImageDimensions::new(37, 19).unwrap();
        let rgba: Vec<_> = (0..37 * 19u32)
            .flat_map(|i| {
                [
                    (i * 73) as u8,
                    (i * 29 + 11) as u8,
                    (i * 13 + 37) as u8,
                    (i * 97) as u8,
                ]
            })
            .collect();
        let hue = json!({"effect":"hue-saturation","enabled":true,"hue":37,"saturation":0.17,"lightness":-0.12});
        let exposure = json!({"effect":"exposure","enabled":true,"stops":1.3});
        let levels = json!({"effect":"levels","enabled":true,"channel":"blue","input":{"black":0.1,"white":0.8},"gamma":1.7,"output":{"black":0.9,"white":0.2}});
        let mask = json!([{"x":{"model":"srgb","channel":"red"},"points":[[0,0],[0.4,0.2],[1,1]]}]);
        let mut masked = levels.clone();
        masked["mask"] = mask;
        let mut cases = vec![
            vec![hue.clone()],
            vec![exposure.clone(), hue.clone(), levels.clone()],
            vec![hue.clone(), masked],
            vec![
                hue.clone(),
                json!({"effect":"brightness-contrast","enabled":true,"brightness":-0.1,"contrast":0.6}),
                json!({"effect":"white-balance","enabled":true,"temperature":0.2,"tint":-0.3}),
                exposure.clone(),
            ],
        ];
        for (model, channel) in [
            ("srgb", "blue"),
            ("linear-rgb", "red"),
            ("hsl", "saturation"),
            ("hsv", "value"),
            ("oklab", "a"),
            ("oklch", "chroma"),
            ("cielab", "lightness"),
            ("cielch", "chroma"),
            ("ycbcr", "cb"),
        ] {
            let selected = json!({"model":model,"channel":channel});
            cases.push(vec![hue.clone(),json!({"effect":"curves","enabled":true,"curves":[
                {"kind":"remap","x":selected,"y":selected,"points":[[0,0.1],[0.2,0.35],[0.7,0.55],[1,0.9]]}]})]);
            cases.push(vec![json!({"effect":"palette-fit","enabled":true,"look":"vivid","space":"oklab","strength":1.5,
                "curves":[{"kind":"adjust","x":{"model":"hsl","channel":"hue"},
                "x2":{"model":"srgb","channel":"green"},"y":selected,
                "grid":{"columns":[0.1,0.4,0.8],"rows":[0,0.3,1],"values":[[0.2,0.4,0.6],[0.7,0.4,0.2],[0.6,0.6,0.8]]}}]})]);
        }
        cases.push(vec![json!({"effect":"curves","enabled":true,"curves":[{"kind":"adjust",
            "x":{"model":"hsl","channel":"hue"},"y":{"model":"oklch","channel":"hue"},
            "points":[[0,0.6],[0.3,0.4],[0.7,0.55],[1,0.6]]}],"mask":[{"x":{"model":"hsl","channel":"hue"},
            "x2":{"model":"hsv","channel":"saturation"},"grid":{"columns":[0,0.5],"rows":[0,1],"values":[[0.2,0.8],[0.9,0.3]]}}]})]);
        for space in [
            "srgb",
            "linear-rgb",
            "oklab",
            "oklch",
            "cielab",
            "cielch",
            "ycbcr",
        ] {
            cases.push(vec![
                hue.clone(),
                json!({"effect":"recolour","enabled":true,"strength":0.7,
                "recipe":{"space":space,"tone":[[0,0],[0.4,0.45],[1,0.95]],"chroma":0.85,
                    "shift":[0.01,-0.02],"groups":[{"hue":90,"width":80,"turn":23,"chroma":1.1},
                    {"hue":185,"width":140,"turn":-15,"chroma":0.9}]},
                "mask":[{"x":{"model":"srgb","channel":"blue"},"points":[[0,0.1],[1,0.8]]}]}),
            ]);
        }
        let mut differences = 0;
        let mut maximum = 0;
        for (index, case) in cases.into_iter().enumerate() {
            let space = case
                .iter()
                .find_map(|step| step.get("recipe").and_then(|r| r.get("space")))
                .map(|space| serde_json::from_value(space.clone()).unwrap())
                .unwrap_or(WorkingSpace::Oklab);
            let steps: Vec<EffectStep> = serde_json::from_value(Value::Array(case)).unwrap();
            let stage = Pointwise::new(steps, Some(space)).unwrap();
            let mut expected = rgba.clone();
            stage.cpu(&mut expected);
            let actual = block_on(backend.pointwise(&stage, dimensions, &rgba, u64::MAX)).unwrap();
            if index % 7 == 0 {
                let tiled = block_on(backend.pointwise_tiled(
                    &stage,
                    dimensions,
                    &rgba,
                    GpuTiling::Rows(3),
                    u64::MAX,
                ))
                .unwrap();
                assert_eq!(
                    tiled, actual,
                    "tile boundaries preserve the continuous chain, case {index}"
                );
            }
            let parity = super::super::Parity::between(&expected, &actual);
            assert!(parity.max_byte_difference <= 1, "case {index}: {parity:?}");
            assert!(rgba
                .chunks_exact(4)
                .zip(actual.chunks_exact(4))
                .all(|(a, b)| a[3] == b[3]));
            differences += parity.differing_pixels;
            maximum = maximum.max(parity.max_byte_difference);
        }
        eprintln!("pointwise differing pixels {differences}, max byte {maximum}");
    }

    /// Proves valid narrow Levels ranges saturate endpoints without overflow,
    /// including subnormal widths after an unrounded mixed-effect carrier.
    #[test]
    fn mixed_levels_preserve_narrow_and_subnormal_endpoints() {
        let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
            return;
        };
        let dimensions = ImageDimensions::new(3, 1).unwrap();
        let rgba = [0, 0, 0, 0, 1, 2, 3, 127, 255, 255, 255, 255];
        for (black, white) in [(0.0, 1e-37), (0.0, 1e-44), (1e-37, 1.000001e-37)] {
            let mut steps = vec![json!({"effect":"exposure","enabled":true,"stops":4}); 3];
            steps.push(json!({"effect":"hue-saturation","enabled":true,"hue":0,"saturation":0,"lightness":0}));
            steps.push(json!({"effect":"levels","enabled":true,"channel":"rgb",
                "input":{"black":black,"white":white},"gamma":1,"output":{"black":0,"white":1}}));
            let stage =
                Pointwise::new(serde_json::from_value(Value::Array(steps)).unwrap(), None).unwrap();
            let mut expected = rgba.to_vec();
            stage.cpu(&mut expected);
            let actual = block_on(backend.pointwise(&stage, dimensions, &rgba, u64::MAX)).unwrap();
            assert_eq!(actual, expected, "{black}..{white}");
            assert_eq!(&actual[0..3], &[0; 3]);
            assert_eq!(&actual[4..7], &[255; 3]);
        }
    }

    /// Proves an over-budget control upload is rejected before GPU execution,
    /// leaving the same device usable for a later admitted request.
    #[test]
    fn control_capacity_rejection_preserves_device() {
        let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
            return;
        };
        let dimensions = ImageDimensions::new(1, 1).unwrap();
        let steps = serde_json::from_value(json!([{"effect":"hue-saturation","enabled":true,
            "hue":0,"saturation":0,"lightness":0}]))
        .unwrap();
        let stage = Pointwise::new(steps, None).unwrap();
        let required = LookupMemory::new(dimensions).required_bytes
            + 16 * stage.words.capacity() as u64
            + stage.recipe_bytes();
        assert_eq!(
            block_on(backend.pointwise(&stage, dimensions, &[93, 93, 93, 0], required - 1)),
            Err(Fallback::MemoryLimit)
        );
        assert_eq!(
            block_on(backend.pointwise(&stage, dimensions, &[93, 93, 93, 0], required)).unwrap(),
            [93, 93, 93, 0]
        );
    }

    /// Proves mask preview reads the unrounded effect carrier, including an
    /// empty prefix/full mask, hue-keyed grids and unchanged hidden alpha.
    #[test]
    fn mask_preview_uses_continuous_carrier() {
        let Some((_serial, mut backend)) = super::super::tests::any_gpu() else {
            return;
        };
        let dimensions = ImageDimensions::new(37, 19).unwrap();
        let source: Vec<_> = (0..37 * 19)
            .flat_map(|i| {
                [
                    (i * 71) as u8,
                    (i * 31) as u8,
                    (i * 23) as u8,
                    (i * 97) as u8,
                ]
            })
            .collect();
        let masks:Vec<Vec<MaskCurve>>=serde_json::from_value(json!([
            [],[{"x":{"model":"srgb","channel":"red"},"points":[[0,0],[0.4,0.1],[1,1]]}],
            [{"x":{"model":"hsl","channel":"hue"},"x2":{"model":"cielch","channel":"chroma"},
                "grid":{"columns":[0.1,0.4,0.8],"rows":[0,0.3,1],"values":[[0.2,0.4,0.6],[0.7,0.4,0.2],[0.6,0.6,0.8]]}}]
        ])).unwrap();
        let prefixes:Vec<Vec<EffectStep>>=serde_json::from_value(json!([
            [],[{"effect":"exposure","enabled":true,"stops":0.33}],
            [{"effect":"exposure","enabled":true,"stops":3},
                {"effect":"hue-saturation","enabled":true,"hue":-50,"saturation":0.5,"lightness":0.1},
                {"effect":"brightness-contrast","enabled":true,"brightness":-0.4,"contrast":0.3}]
        ])).unwrap();
        for prefix in prefixes {
            for mask in &masks {
                let text = serde_json::to_string(&prefix).unwrap().len()
                    + serde_json::to_string(mask).unwrap().len();
                let stage = Pointwise::new(prefix.clone(), None)
                    .unwrap()
                    .with_mask(mask.clone())
                    .unwrap();
                let mut expected = source.clone();
                stage.cpu(&mut expected);
                let memory = Pointwise::memory(dimensions, text as u64).required_bytes
                    + 8 * source.len() as u64;
                let actual = block_on(backend.pointwise_tiled(
                    &stage,
                    dimensions,
                    &source,
                    GpuTiling::Rows(3),
                    memory,
                ))
                .unwrap();
                let parity = super::super::Parity::between(&expected, &actual);
                assert!(parity.max_byte_difference <= 1, "{parity:?}");
                for (a, b) in actual.chunks_exact(4).zip(source.chunks_exact(4)) {
                    assert_eq!(a[0], a[1]);
                    assert_eq!(a[1], a[2]);
                    assert_eq!(a[3], b[3]);
                    if mask.is_empty() {
                        assert_eq!(&a[..3], &[255; 3]);
                    }
                }
            }
        }
    }
}
