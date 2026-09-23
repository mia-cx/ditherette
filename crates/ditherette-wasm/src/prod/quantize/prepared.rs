//! Fallible prepared ownership and allocation-free indexed execution.

use super::{
    cache::RgbCache,
    matcher::{finite_hue_arc3_squared, PaletteColor, PaletteMatcher},
};
use crate::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageView, PaletteIndex8, Rgba8,
    },
    prod::{
        color::packed::{Converter, OrdinarySpace},
        contract::request::{AlphaPolicy, MatchPolicy},
        palette::{allocation::Budget, PalettePixel, PreparationError, PreparedPalette},
        tiling::{RowBand, RowBandBuffers},
    },
};
use std::mem::size_of;

const HUE_COORDINATE_BOUND: f32 = 65_536.0;

fn hue_coordinates_bounded(coordinates: [f32; 3]) -> bool {
    coordinates
        .iter()
        .all(|value| value.abs() <= HUE_COORDINATE_BOUND)
}

/// Owned palette and matching data. No source buffer or alpha float plane is retained.
pub struct PreparedQuantizer {
    palette: PreparedPalette,
    matcher: PaletteMatcher,
    converter: Converter,
    bounded_hue_palette: bool,
}

impl PreparedQuantizer {
    /// Predicts this record and its heap reservations before constructing tables or allocating.
    pub fn required_capacity_bytes(
        entries: &[PaletteEntry],
        alpha: AlphaPolicy,
        matching: MatchPolicy,
    ) -> Result<u64, PreparationError> {
        let _ = matching; // Every typed recipe is supported; capacity depends on visible entries.
        let palette = PreparedPalette::required_capacity_bytes(entries, alpha)?;
        let visible = entries
            .iter()
            .take(256)
            .filter(|entry| matches!(entry, PaletteEntry::Color { .. }))
            .count();
        Ok(
            size_of::<Self>() as u64 + palette - size_of::<PreparedPalette>() as u64
                + (visible * size_of::<PaletteColor>()) as u64,
        )
    }

    /// Reserves all palette, warning-text, and matcher capacities without infallible heap growth.
    pub fn try_new(
        entries: &[PaletteEntry],
        alpha: AlphaPolicy,
        matching: MatchPolicy,
        memory_limit: u64,
    ) -> Result<Self, PreparationError> {
        let required = Self::required_capacity_bytes(entries, alpha, matching)?;
        if required > memory_limit {
            return Err(PreparationError::memory());
        }
        let mut budget = Budget::new(memory_limit, size_of::<Self>() as u64)?;
        let palette = PreparedPalette::prepare(entries, alpha, &mut budget)?;
        let converter = Converter::new(
            OrdinarySpace::from_matching(matching).expect("every matching tag has coordinates"),
        );
        let matcher = PaletteMatcher::prepare(&palette, &converter, matching, &mut budget)?;
        let bounded_hue_palette = matches!(
            matching,
            MatchPolicy::OklchHueArc | MatchPolicy::CielchHueArc
        ) && matcher
            .colors
            .iter()
            .all(|color| hue_coordinates_bounded(color.coordinates));
        Ok(Self {
            palette,
            matcher,
            converter,
            bounded_hue_palette,
        })
    }

    /// Actual record, tables, Vec capacities and warning-string capacities.
    pub fn capacity_bytes(&self) -> u64 {
        size_of::<Self>() as u64 + self.palette.capacity_bytes()
            - size_of::<PreparedPalette>() as u64
            + (self.matcher.colors.capacity() * size_of::<PaletteColor>()) as u64
    }

    pub fn palette(&self) -> &PreparedPalette {
        &self.palette
    }

    /// RGB matching is unreachable when alpha policy fixes every output index.
    pub(crate) fn can_match_rgb(&self) -> bool {
        !self.palette.visible.is_empty()
            && match self.palette.preserved_alpha() {
                Some((threshold, _)) => threshold < u8::MAX,
                None => true,
            }
    }

    /// Borrow the ordered visible coordinates for palette-mixing kernels.
    pub(crate) fn matcher(&self) -> &PaletteMatcher {
        &self.matcher
    }

    pub(crate) fn converter(&self) -> &Converter {
        &self.converter
    }

    /// Prunes hue work only when every full score is provably finite.
    /// With both coordinates bounded by B, the L/C base is at most 8*B² and
    /// the squared hue term is below 16*B². Their sum cannot overflow f32.
    /// The rounded sum of nonnegative terms cannot fall below the L/C base,
    /// so base >= best cannot improve the first strict minimum.
    pub(crate) fn nearest_finite(&self, coordinates: [f32; 3]) -> Option<PaletteColor> {
        if !self.bounded_hue_palette || !hue_coordinates_bounded(coordinates) {
            return self.matcher.nearest_finite(coordinates);
        }
        let mut best = self.matcher.colors[0];
        let mut best_score = f32::INFINITY;
        for &candidate in &self.matcher.colors {
            let dl = coordinates[0] - candidate.coordinates[0];
            let dc = coordinates[1] - candidate.coordinates[1];
            if dl * dl + dc * dc >= best_score {
                continue;
            }
            let score = finite_hue_arc3_squared(coordinates, candidate.coordinates);
            if score < best_score {
                best = candidate;
                best_score = score;
            }
        }
        Some(best)
    }

    /// Writes one index per source pixel. The caller supplies validated output storage.
    /// Alpha comes from the corresponding source byte; no allocation occurs in this method.
    pub fn quantize_into(&self, source: ImageView<'_, Rgba8>, output: &mut [u8]) {
        self.quantize_with_progress(source, output, |_| Ok(()))
            .expect("disabled progress cannot fail");
    }

    /// Writes a disjoint packed output band from absolute rows of the full source.
    /// All workers share this immutable preparation; no per-band allocation occurs.
    pub fn quantize_rows_into(
        &self,
        source: ImageView<'_, Rgba8>,
        rows: RowBand,
        output: &mut [u8],
    ) {
        let dimensions = source.dimensions();
        assert!(rows.y_end() <= dimensions.height());
        assert_eq!(
            output.len(),
            dimensions.width_usize() * rows.height() as usize
        );
        for (y, output) in
            (rows.y_start()..rows.y_end()).zip(output.chunks_exact_mut(dimensions.width_usize()))
        {
            self.quantize_row_into(source.row(y).expect("valid source row"), output);
        }
    }

    /// Executes preflighted row work with shared preparation and no worker scratch.
    /// Completed-row callbacks run on the caller after each joined batch.
    pub fn quantize_bands_into(
        &self,
        source: ImageView<'_, Rgba8>,
        output: &mut [u8],
        work: &mut RowBandBuffers<()>,
        progress: &mut impl FnMut(u64) -> Result<(), crate::prod::contract::failure::Failure>,
    ) -> Result<(), crate::prod::contract::failure::Failure> {
        work.execute(
            output,
            source.dimensions().width_usize(),
            &|band, output, _| {
                self.quantize_rows_into(source, band, output);
                Ok(u64::from(band.height()))
            },
            progress,
        )
    }

    fn quantize_row_into(&self, source: &[u8], output: &mut [u8]) {
        self.quantize_row_with(source, output, |rgb| {
            self.matcher.nearest(self.converter.coordinates(rgb)).index
        });
    }

    fn quantize_row_with(
        &self,
        source: &[u8],
        output: &mut [u8],
        mut nearest: impl FnMut([u8; 3]) -> u8,
    ) {
        if let Some((threshold, index)) = self.palette.preserved_alpha() {
            if self.palette.visible.is_empty() {
                output.fill(index);
                return;
            }
            for (source, output) in source.chunks_exact(4).zip(output) {
                *output = if source[3] <= threshold {
                    index
                } else {
                    nearest([source[0], source[1], source[2]])
                };
            }
            return;
        }
        for (source, output) in source.chunks_exact(4).zip(output) {
            let rgba = [source[0], source[1], source[2], source[3]];
            *output = match self.palette.prepare_pixel(rgba) {
                PalettePixel::Index(index) => index,
                PalettePixel::Color(rgb) => nearest(rgb),
            };
        }
    }

    /// Runs a fresh exact-byte cache across all rows, using only caller-reserved scratch.
    /// Empty scratch retains the allocation-free direct scan. Rebinding clears all cached entries.
    /// Nonempty scratch must contain a power-of-two number of entries, at most 262,144.
    pub fn quantize_cached_with_progress(
        &self,
        source: ImageView<'_, Rgba8>,
        output: &mut [u8],
        entries: &mut [u64],
        mut progress: impl FnMut(u32) -> Result<(), crate::prod::contract::failure::Failure>,
    ) -> Result<(), crate::prod::contract::failure::Failure> {
        if entries.is_empty() {
            return self.quantize_with_progress(source, output, progress);
        }
        let dimensions = source.dimensions();
        assert_eq!(
            output.len(),
            dimensions.pixel_count().expect("valid dimensions")
        );
        let mut cache = RgbCache::new(entries);
        for (y, output) in output
            .chunks_exact_mut(dimensions.width_usize())
            .enumerate()
        {
            self.quantize_row_with(
                source.row(y as u32).expect("valid source row"),
                output,
                |rgb| {
                    cache.nearest(rgb, || {
                        self.matcher.nearest(self.converter.coordinates(rgb)).index
                    })
                },
            );
            progress(y as u32 + 1)?;
        }
        Ok(())
    }

    /// Shares immutable palette data while each worker owns its exact RGB cache.
    pub(crate) fn quantize_cached_bands_into(
        &self,
        source: ImageView<'_, Rgba8>,
        output: &mut [u8],
        work: &mut RowBandBuffers<u64>,
        progress: &mut impl FnMut(u64) -> Result<(), crate::prod::contract::failure::Failure>,
    ) -> Result<(), crate::prod::contract::failure::Failure> {
        work.execute(
            output,
            source.dimensions().width_usize(),
            &|band, output, entries| {
                if entries.is_empty() {
                    self.quantize_rows_into(source, band, output);
                } else {
                    let mut cache = RgbCache::new(entries);
                    for (y, output) in (band.y_start()..band.y_end())
                        .zip(output.chunks_exact_mut(source.dimensions().width_usize()))
                    {
                        self.quantize_row_with(
                            source.row(y).expect("valid source row"),
                            output,
                            |rgb| {
                                cache.nearest(rgb, || {
                                    self.matcher.nearest(self.converter.coordinates(rgb)).index
                                })
                            },
                        );
                    }
                }
                Ok(u64::from(band.height()))
            },
            progress,
        )
    }

    /// Reports each completed row without changing per-pixel traversal or arithmetic.
    pub(crate) fn quantize_with_progress(
        &self,
        source: ImageView<'_, Rgba8>,
        output: &mut [u8],
        mut progress: impl FnMut(u32) -> Result<(), crate::prod::contract::failure::Failure>,
    ) -> Result<(), crate::prod::contract::failure::Failure> {
        let dimensions = source.dimensions();
        assert_eq!(
            output.len(),
            dimensions.pixel_count().expect("valid dimensions")
        );
        for y in 0..dimensions.height() {
            let source = source.row(y).expect("valid source row");
            let row_start = y as usize * dimensions.width_usize();
            let output = &mut output[row_start..row_start + dimensions.width_usize()];
            self.quantize_row_into(source, output);
            progress(y + 1)?;
        }
        Ok(())
    }

    /// Moves palette metadata into a complete native result; no copies or allocations occur.
    pub fn into_indexed(self, indices: ImageBuf<PaletteIndex8>) -> IndexedImage {
        self.palette.into_indexed(indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::contracts::PaletteEntry;

    #[test]
    fn bounded_hue_selection_matches_full_scan_and_fallback_at_domain_edges() {
        let mut palette: Vec<_> = (0..63_u32)
            .map(|n| PaletteEntry::Color {
                rgb: [(n * 73) as u8, (n * 31 + 17) as u8, (n * 113 + 53) as u8],
            })
            .collect();
        palette.push(palette[0]);
        let outside = f32::from_bits(HUE_COORDINATE_BOUND.to_bits() + 1);
        for matching in [
            MatchPolicy::OklchHueArc,
            MatchPolicy::CielchHueArc,
            MatchPolicy::OklabEuclidean,
        ] {
            let prepared = PreparedQuantizer::try_new(
                &palette,
                AlphaPolicy::Premultiplied {},
                matching,
                u64::MAX,
            )
            .unwrap();
            assert_eq!(
                prepared.bounded_hue_palette,
                matching != MatchPolicy::OklabEuclidean
            );
            let check = |coordinates| {
                assert_eq!(
                    prepared.nearest_finite(coordinates),
                    prepared.matcher.nearest_finite(coordinates),
                    "{matching:?}, {coordinates:?}"
                )
            };
            let mut bits = 0x78ab_c901u32;
            let mut next = || {
                bits ^= bits << 13;
                bits ^= bits >> 17;
                bits ^= bits << 5;
                bits
            };
            for n in 0..10_000 {
                let coordinates = std::array::from_fn(|axis| {
                    let bits = next();
                    match n % 3 {
                        0 => f32::from_bits(bits),
                        1 => bits as i32 as f32 * (if axis == 2 { 2e-8 } else { 1e-9 }),
                        _ => bits as i32 as f32 * 3e-5,
                    }
                });
                check(coordinates);
            }
            for axis in 0..3 {
                for edge in [
                    0.0,
                    -0.0,
                    HUE_COORDINATE_BOUND,
                    -HUE_COORDINATE_BOUND,
                    outside,
                    -outside,
                    f32::MAX,
                    -f32::MAX,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NAN,
                ] {
                    let mut coordinates = [0.5, -0.25, -12.0];
                    coordinates[axis] = edge;
                    check(coordinates);
                }
            }
            assert_eq!(
                prepared
                    .nearest_finite(prepared.matcher.colors[0].coordinates)
                    .unwrap()
                    .index,
                0
            );
        }
    }

    #[test]
    fn bounds_reject_nonfinite_and_outside_coordinates() {
        for axis in 0..3 {
            for value in [
                f32::NAN,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::from_bits(HUE_COORDINATE_BOUND.to_bits() + 1),
                -f32::MAX,
            ] {
                let mut coordinates = [0.0; 3];
                coordinates[axis] = value;
                assert!(!hue_coordinates_bounded(coordinates));
            }
        }
        assert!(hue_coordinates_bounded([
            HUE_COORDINATE_BOUND,
            -HUE_COORDINATE_BOUND,
            -0.0
        ]));
    }

    #[test]
    fn fixed_index_palettes_never_need_rgb_matching() {
        let transparent = [PaletteEntry::Transparent {}];
        let visible = [PaletteEntry::Color { rgb: [1, 2, 3] }];
        let prepare = |palette, alpha| {
            PreparedQuantizer::try_new(palette, alpha, MatchPolicy::SrgbEuclidean, u64::MAX)
                .unwrap()
        };

        assert!(!prepare(&transparent, AlphaPolicy::Premultiplied {}).can_match_rgb());
        assert!(!prepare(&visible, AlphaPolicy::Preserve { threshold: 255.0 }).can_match_rgb());
        assert!(prepare(&visible, AlphaPolicy::Preserve { threshold: 254.9 }).can_match_rgb());
        assert!(prepare(&visible, AlphaPolicy::Premultiplied {}).can_match_rgb());
    }
}
