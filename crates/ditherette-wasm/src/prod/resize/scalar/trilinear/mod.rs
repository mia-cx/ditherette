//! Spec trilinear resize.
//!
//! Trilinear is a mip-level policy: build an area-filtered mip pyramid, resize
//! from the two mip levels surrounding the requested minification with bilinear,
//! then blend those two results by fractional level-of-detail.

use crate::{
    image::{ImageDimensions, ImageFormat, ImageView, ImageViewMut},
    prod::resize::scalar::trilinear::exact::{
        common::{alignment::ResizeAnchor, sample::ResizeSample},
        scalar::{area::resize_area_into, bilinear::resize_bilinear_into},
    },
};

/// Resizes `source` into `output` with mipmapped trilinear filtering.
pub fn resize_trilinear_into<F>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
) where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let minification = minification_factor(source_dimensions, output_dimensions);

    if minification <= 1.0 {
        resize_bilinear_into(source, output, anchor);
        return;
    }

    let lod = minification.log2();
    let lower_lod = lod.floor() as usize;
    let upper_lod = lod.ceil() as usize;
    let blend = lod - lower_lod as f64;
    let lower_level = mip_level(&source, lower_lod);

    if lower_lod == upper_lod
        || lower_level.dimensions.width() == 1 && lower_level.dimensions.height() == 1
    {
        resize_bilinear_into(lower_level.view(), output, anchor);
        return;
    }

    let upper_level = mip_level(&source, upper_lod);
    let output_len = output_dimensions.storage_len::<F>().unwrap();
    let mut lower_output = vec![F::Storage::default(); output_len];
    let mut upper_output = vec![F::Storage::default(); output_len];

    resize_bilinear_into(
        lower_level.view(),
        ImageViewMut::<F>::packed(&mut lower_output, output_dimensions).unwrap(),
        anchor,
    );
    resize_bilinear_into(
        upper_level.view(),
        ImageViewMut::<F>::packed(&mut upper_output, output_dimensions).unwrap(),
        anchor,
    );

    for output_y in 0..output_dimensions.height() {
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");
        let row_start = output_y as usize * output_dimensions.width_usize() * F::CHANNEL_COUNT;
        let row_end = row_start + output_dimensions.width_usize() * F::CHANNEL_COUNT;

        for ((output_channel, lower), upper) in output_row
            .iter_mut()
            .zip(&lower_output[row_start..row_end])
            .zip(&upper_output[row_start..row_end])
        {
            let blended = lower.to_f64() * (1.0 - blend) + upper.to_f64() * blend;
            *output_channel = F::Storage::from_f64(blended);
        }
    }
}

#[derive(Debug, Clone)]
struct MipLevel<F: ImageFormat> {
    dimensions: ImageDimensions,
    data: Vec<F::Storage>,
}

impl<F: ImageFormat> MipLevel<F> {
    fn view(&self) -> ImageView<'_, F> {
        ImageView::<F>::packed(&self.data, self.dimensions)
            .expect("mip levels are produced as packed images")
    }
}

fn mip_level<F>(source: &ImageView<'_, F>, target_level: usize) -> MipLevel<F>
where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    let mut data = Vec::new();
    for y in 0..source.dimensions().height() {
        data.extend_from_slice(source.row(y).expect("source row is within dimensions"));
    }
    let mut current = MipLevel {
        dimensions: source.dimensions(),
        data,
    };

    for _ in 0..target_level {
        if current.dimensions.width() == 1 && current.dimensions.height() == 1 {
            break;
        }

        let next_dimensions = ImageDimensions::new(
            half_rounded_up(current.dimensions.width()),
            half_rounded_up(current.dimensions.height()),
        )
        .expect("halving non-zero dimensions should keep dimensions non-zero");
        let mut next_data =
            vec![F::Storage::default(); next_dimensions.storage_len::<F>().unwrap()];

        resize_area_into(
            current.view(),
            ImageViewMut::<F>::packed(&mut next_data, next_dimensions).unwrap(),
        );

        current = MipLevel {
            dimensions: next_dimensions,
            data: next_data,
        };
    }

    current
}

fn minification_factor(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> f64 {
    let x_factor = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_factor = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());
    x_factor.max(y_factor)
}

fn half_rounded_up(value: u32) -> u32 {
    value.div_ceil(2).max(1)
}

mod exact;
mod prepared;
pub use prepared::PreparedTrilinear;
