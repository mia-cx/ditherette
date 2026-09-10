//! Bounded exact RGB memoization in caller-owned, capacity-accounted scratch.

/// At most 2 MiB per active scalar call or worker, independent of source entropy.
pub(crate) const MAX_ENTRIES: usize = 1 << 18;
const MIN_ENTRIES: usize = 1 << 10;
const VALID: u64 = 1 << 32;
const RGB_MASK: u64 = 0x00ff_ffff;

/// Selects a power-of-two table within spare heap bytes; small calls use the direct scan.
/// The caller accounts for the owning Vec record separately from these heap entries.
pub(crate) fn recommended_entries(pixel_count: usize, available_bytes: u64) -> usize {
    if pixel_count < MIN_ENTRIES {
        return 0;
    }
    let ceiling = (available_bytes / std::mem::size_of::<u64>() as u64)
        .min(MAX_ENTRIES as u64)
        .min(pixel_count.saturating_mul(2) as u64) as usize;
    if ceiling < MIN_ENTRIES {
        return 0;
    }
    1 << ceiling.ilog2()
}

/// A fresh binding to one prepared palette/alpha/metric. No float coordinates enter this cache.
pub(super) struct RgbCache<'a> {
    entries: &'a mut [u64],
}

impl<'a> RgbCache<'a> {
    pub fn new(entries: &'a mut [u64]) -> Self {
        assert!(entries.len().is_power_of_two() && entries.len() <= MAX_ENTRIES);
        // Scratch may have belonged to any earlier prepared quantizer, including after cancellation.
        entries.fill(0);
        Self { entries }
    }

    #[inline]
    pub fn nearest(&mut self, rgb: [u8; 3], miss: impl FnOnce() -> u8) -> u8 {
        let key = u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2]);
        // Mix all three byte channels before masking the bounded direct-mapped table.
        let slot = (key.wrapping_mul(0x9e37_79b1) >> 8) as usize & (self.entries.len() - 1);
        let entry = &mut self.entries[slot];
        if *entry & (VALID | RGB_MASK) == VALID | u64::from(key) {
            return (*entry >> 24) as u8;
        }
        let index = miss();
        *entry = VALID | u64::from(key) | u64::from(index) << 24;
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        image::{contracts::PaletteEntry, ImageDimensions, ImageView, Rgba8},
        prod::{
            contract::request::{AlphaPolicy, MatchPolicy},
            quantize::PreparedQuantizer,
            tiling::{RowBandBuffers, WorkerBudget},
        },
        spec,
    };

    #[test]
    fn capacity_is_bounded_and_low_budgets_disable_the_cache() {
        assert_eq!(recommended_entries(1023, u64::MAX), 0);
        assert_eq!(recommended_entries(usize::MAX, 8191), 0);
        assert_eq!(recommended_entries(usize::MAX, 8192), 1024);
        assert_eq!(recommended_entries(usize::MAX, 16383), 1024);
        assert_eq!(recommended_entries(usize::MAX, u64::MAX), MAX_ENTRIES);
        assert_eq!(recommended_entries(1024, u64::MAX), 2048);
    }

    #[test]
    fn exact_hits_preserve_black_white_and_index_255_and_collisions_recompute() {
        let mut entries = [0];
        let mut cache = RgbCache::new(&mut entries);
        for (rgb, index) in [([0; 3], 0), ([255; 3], 255), ([1, 2, 3], 42), ([0; 3], 7)] {
            assert_eq!(cache.nearest(rgb, || index), index);
            assert_eq!(
                cache.nearest(rgb, || panic!("exact hit must skip matching")),
                index
            );
        }
    }

    #[test]
    fn reused_scratch_matches_all_metrics_alpha_and_palette_changes() {
        use MatchPolicy::*;
        let policies = [
            SrgbEuclidean,
            LinearRgbEuclidean,
            OklabEuclidean,
            CielabEuclidean,
            YcbcrEuclidean,
            SrgbCompuphase,
            SrgbRec601,
            SrgbRec709,
            OklchEuclidean,
            OklchCircularHue,
            OklchHueArc,
            CielabCiede2000,
            CielchEuclidean,
            CielchCircularHue,
            CielchHueArc,
        ];
        let dimensions = ImageDimensions::new(64, 32).unwrap();
        let source: Vec<u8> = (0..2048u32)
            .flat_map(|n| {
                let color = n % 1024;
                [
                    (color * 73) as u8,
                    (color * 31 + color / 256) as u8,
                    (color * 17) as u8,
                    (n / 8) as u8,
                ]
            })
            .collect();
        let view = ImageView::<Rgba8>::packed(&source, dimensions).unwrap();
        let color = |rgb| PaletteEntry::Color { rgb };
        let palettes = [
            vec![
                PaletteEntry::Transparent {},
                color([0; 3]),
                color([2, 0, 0]),
                color([255; 3]),
                color([255; 3]),
            ],
            vec![color([255; 3]), color([0; 3]), color([73; 3])],
            vec![PaletteEntry::Transparent {}],
        ];
        // Deliberately undersized so high-entropy source colors must collide.
        let mut entries = [u64::MAX; 128];
        let mut output = vec![0; 2048];
        for matching in policies {
            for palette in &palettes {
                for alpha in [
                    AlphaPolicy::Preserve {
                        threshold: 127.9999999,
                    },
                    AlphaPolicy::Matte { rgb: [17, 73, 211] },
                    AlphaPolicy::Premultiplied {},
                ] {
                    let prepared =
                        PreparedQuantizer::try_new(palette, alpha, matching, u64::MAX).unwrap();
                    let expected =
                        spec::quantize::quantize(spec::contract::request::QuantizeRequest {
                            version: 1,
                            source: spec::contract::request::Source {
                                width: 64,
                                height: 32,
                                data: &source,
                            },
                            palette,
                            alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap())
                                .unwrap(),
                            matching: serde_json::from_value(
                                serde_json::to_value(matching).unwrap(),
                            )
                            .unwrap(),
                        })
                        .unwrap();
                    let mut completed = 0;
                    prepared
                        .quantize_cached_with_progress(view, &mut output, &mut entries, |row| {
                            assert_eq!(row, completed + 1);
                            completed = row;
                            Ok(())
                        })
                        .unwrap();
                    assert_eq!(
                        output,
                        expected.indices.data(),
                        "{matching:?}, {alpha:?}, {palette:?}"
                    );
                    assert_eq!(completed, 32);
                    prepared
                        .quantize_cached_with_progress(view, &mut output, &mut [], |_| Ok(()))
                        .unwrap();
                    assert_eq!(output, expected.indices.data());
                }
            }
        }
    }

    #[test]
    fn band_caches_share_immutable_preparation_and_count_owned_capacity() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<PreparedQuantizer>();
        let palette = [
            PaletteEntry::Color { rgb: [0; 3] },
            PaletteEntry::Color { rgb: [255; 3] },
        ];
        let prepared = PreparedQuantizer::try_new(
            &palette,
            AlphaPolicy::Premultiplied {},
            MatchPolicy::SrgbRec709,
            u64::MAX,
        )
        .unwrap();
        let dimensions = ImageDimensions::new(64, 32).unwrap();
        let source: Vec<u8> = (0..2048u32)
            .flat_map(|n| [(n * 73) as u8, (n * 31) as u8, (n * 17) as u8, 255])
            .collect();
        let view = ImageView::<Rgba8>::packed(&source, dimensions).unwrap();
        let mut expected = vec![0; 2048];
        prepared.quantize_into(view, &mut expected);
        for entries in [0, 128] {
            let workers = WorkerBudget::new(4);
            let required =
                RowBandBuffers::<u64>::required_bytes(dimensions, 7, workers, 4, &|_| Ok(entries))
                    .unwrap();
            let mut bands =
                RowBandBuffers::try_new(dimensions, 7, workers, 4, required, &|_| Ok(entries))
                    .unwrap();
            assert_eq!(bands.capacity_bytes(), required);
            let mut actual = vec![0; 2048];
            let mut completed = 0;
            prepared
                .quantize_cached_bands_into(view, &mut actual, &mut bands, &mut |rows| {
                    completed = rows;
                    Ok(())
                })
                .unwrap();
            assert_eq!(actual, expected);
            assert_eq!(completed, 32);
        }
    }
}
