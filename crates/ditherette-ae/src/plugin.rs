//! The After Effects effect: controls, and the render command.
//!
//! Renders 8-bit frames. In 16- and 32-bit projects After Effects converts
//! frames to 8 bits for this effect, which suits a palette-limited result.

use after_effects as ae;

use crate::{
    look::{self, defaults, labels, position, Look, DITHERS, PLACEMENTS, RESIZES, SPACES},
    palette,
    render::{Engine, Image, ImageMut, Ran},
};

/// Control order in the Effect Controls panel. Saved projects depend on it: only append.
#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
enum Params {
    PaletteLayer,
    PixelSize,
    Resize,
    Dither,
    Strength,
    Serpentine,
    ColourSpace,
    DitherInSpace,
    Placement,
    SampleSize,
    ContrastTolerance,
}

#[derive(Default)]
struct Plugin {
    engine: Engine,
}

ae::define_effect!(Plugin, (), Params);

fn popup<T: PartialEq>(rows: &'static [look::Row<T>], default: T) -> ae::PopupDef<'static> {
    ae::PopupDef::setup(|f| {
        f.set_options(&labels(rows));
        f.set_default(position(rows, default));
    })
}

fn slider(min: f32, max: f32, default: f64, percent: bool) -> ae::FloatSliderDef<'static> {
    ae::FloatSliderDef::setup(|f| {
        f.set_valid_min(min);
        f.set_slider_min(min);
        f.set_valid_max(max);
        f.set_slider_max(max);
        f.set_default(default);
        f.set_precision(0);
        if percent {
            f.set_display_flags(ae::ValueDisplayFlag::PERCENT);
        }
    })
}

fn checkbox(label: &str, default: bool) -> ae::CheckBoxDef<'static> {
    ae::CheckBoxDef::setup(|f| {
        f.set_label(label);
        f.set_default(default);
    })
}

impl AdobePluginGlobal for Plugin {
    fn params_setup(
        &self,
        params: &mut ae::Parameters<Params>,
        _: ae::InData,
        _: ae::OutData,
    ) -> Result<(), ae::Error> {
        params.add(Params::PaletteLayer, "Palette layer", ae::LayerDef::new())?;
        params.add(
            Params::PixelSize,
            "Pixel size",
            slider(
                1.0,
                defaults::MAX_PIXEL_SIZE as f32,
                defaults::PIXEL_SIZE.into(),
                false,
            ),
        )?;
        params.add(Params::Resize, "Resize", popup(RESIZES, defaults::RESIZE))?;
        params.add(Params::Dither, "Dither", popup(DITHERS, defaults::DITHER))?;
        params.add(
            Params::Strength,
            "Strength",
            slider(0.0, 100.0, defaults::STRENGTH, true),
        )?;
        params.add(
            Params::Serpentine,
            "",
            checkbox("Serpentine scan", defaults::SERPENTINE),
        )?;
        params.add(
            Params::ColourSpace,
            "Colour space",
            popup(SPACES, look::choice(SPACES, 1)),
        )?;
        params.add(
            Params::DitherInSpace,
            "",
            checkbox("Dither in selected space", defaults::DITHER_IN_SPACE),
        )?;
        params.add(
            Params::Placement,
            "Placement",
            popup(PLACEMENTS, defaults::PLACEMENT),
        )?;
        params.add(
            Params::SampleSize,
            "Sample size",
            slider(
                defaults::MIN_SAMPLE_SIZE as f32,
                defaults::MAX_SAMPLE_SIZE as f32,
                defaults::SAMPLE_SIZE.into(),
                false,
            ),
        )?;
        params.add(
            Params::ContrastTolerance,
            "Contrast tolerance",
            slider(0.0, 100.0, defaults::CONTRAST_TOLERANCE, true),
        )
    }

    fn handle_command(
        &mut self,
        command: ae::Command,
        in_data: ae::InData,
        mut out_data: ae::OutData,
        params: &mut ae::Parameters<Params>,
    ) -> Result<(), ae::Error> {
        match command {
            ae::Command::About => out_data.set_return_msg(concat!(
                "Ditherette ",
                env!("CARGO_PKG_VERSION"),
                "\rPalette-limited pixel art.\rhttps://ditherette.mia.cx"
            )),
            ae::Command::Render {
                in_layer,
                mut out_layer,
            } => self.render(&in_data, params, &in_layer, &mut out_layer)?,
            _ => {}
        }
        Ok(())
    }
}

impl Plugin {
    fn render(
        &self,
        in_data: &ae::InData,
        params: &mut ae::Parameters<Params>,
        in_layer: &ae::Layer,
        out_layer: &mut ae::Layer,
    ) -> Result<(), ae::Error> {
        let same_size =
            (in_layer.width(), in_layer.height()) == (out_layer.width(), out_layer.height());
        if !same_size || in_layer.bit_depth() != 8 || out_layer.bit_depth() != 8 {
            return Err(ae::Error::BadCallbackParameter);
        }
        let float =
            |param| -> Result<f64, ae::Error> { Ok(params.get(param)?.as_float_slider()?.value()) };
        let popup =
            |param| -> Result<i32, ae::Error> { Ok(params.get(param)?.as_popup()?.value()) };
        let checked =
            |param| -> Result<bool, ae::Error> { Ok(params.get(param)?.as_checkbox()?.value()) };
        // Reduced preview resolution shrinks frames; shrink the cells with them.
        let downsample = f64::from(in_data.downsample_x());
        let look = Look {
            pixel_size: (float(Params::PixelSize)? * downsample).round().max(1.0) as u32,
            resize: look::choice(RESIZES, popup(Params::Resize)?),
            dither: look::choice(DITHERS, popup(Params::Dither)?),
            strength: float(Params::Strength)? as f32,
            serpentine: checked(Params::Serpentine)?,
            space: look::choice(SPACES, popup(Params::ColourSpace)?),
            dither_in_space: checked(Params::DitherInSpace)?,
            placement: look::choice(PLACEMENTS, popup(Params::Placement)?),
            sample_size: float(Params::SampleSize)?.round() as u32,
            contrast_tolerance: float(Params::ContrastTolerance)? as f32,
        };
        let palette = match params.get(Params::PaletteLayer)?.as_layer()?.value() {
            Some(layer) if layer.bit_depth() == 8 => palette::from_layer(image(&layer)),
            Some(_) => return Err(ae::Error::BadCallbackParameter),
            None => palette::wplace(),
        };
        let (width, height, stride) =
            (out_layer.width(), out_layer.height(), out_layer.row_bytes());
        let ran = self
            .engine
            .render(
                &look,
                &palette,
                image(in_layer),
                &mut ImageMut {
                    width,
                    height,
                    stride,
                    argb: out_layer.buffer_mut(),
                },
            )
            .map_err(|failure| {
                log::error!("Ditherette could not render this frame: {failure:?}");
                ae::Error::Generic
            })?;
        if let Ran::Cpu(reason) = ran {
            log::debug!("Ditherette rendered on the CPU: {reason:?}");
        }
        Ok(())
    }
}

fn image(layer: &ae::Layer) -> Image<'_> {
    Image {
        width: layer.width(),
        height: layer.height(),
        stride: layer.row_bytes(),
        argb: layer.buffer(),
    }
}
