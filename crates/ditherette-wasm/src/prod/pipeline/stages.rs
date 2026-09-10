//! Owned materialized stages. Their records and heap capacities have separate ledger owners.

use std::mem::size_of;

use crate::{
    image::{
        contracts::{NormalizedPalette, ProcessWarning},
        ImageDimensions,
    },
    prod::{contract::failure::Failure, palette::allocation::Budget},
};

use super::quantize::preparation_failure;

/// Metadata survives independent eviction of any prepared matcher.
pub(super) struct IndexedMetadata {
    pub palette: NormalizedPalette,
    pub warnings: Vec<ProcessWarning>,
}

impl IndexedMetadata {
    pub fn required_bytes(palette: &NormalizedPalette, warnings: &[ProcessWarning]) -> u64 {
        (palette.rgba.len()
            + warnings.len() * size_of::<ProcessWarning>()
            + warnings
                .iter()
                .map(|warning| warning.message.len())
                .sum::<usize>()) as u64
    }

    pub fn try_copy(
        palette: &NormalizedPalette,
        warnings: &[ProcessWarning],
        limit: u64,
    ) -> Result<Self, Failure> {
        let mut budget = Budget::new(limit, 0).map_err(preparation_failure)?;
        let mut rgba = Vec::new();
        budget
            .reserve(&mut rgba, palette.rgba.len())
            .map_err(preparation_failure)?;
        rgba.extend_from_slice(&palette.rgba);
        let mut copied = Vec::new();
        budget
            .reserve(&mut copied, warnings.len())
            .map_err(preparation_failure)?;
        for warning in warnings {
            copied.push(ProcessWarning {
                code: warning.code,
                message: budget
                    .string(&warning.message)
                    .map_err(preparation_failure)?,
            });
        }
        Ok(Self {
            palette: NormalizedPalette {
                rgba,
                transparent_index: palette.transparent_index,
            },
            warnings: copied,
        })
    }

    pub fn capacity_bytes(&self) -> u64 {
        (self.palette.rgba.capacity()
            + self.warnings.capacity() * size_of::<ProcessWarning>()
            + self
                .warnings
                .iter()
                .map(|warning| warning.message.capacity())
                .sum::<usize>()) as u64
    }
}

pub(super) enum Metadata {
    Rgba,
    Indexed(IndexedMetadata),
}

/// Takes an already-materialized buffer by ownership; no source snapshot becomes an entry.
pub(super) struct ImageStage {
    pub dimensions: ImageDimensions,
    pub bytes: Vec<u8>,
    pub metadata: Metadata,
}

impl ImageStage {
    /// Heap capacity only. The store separately owns and counts the containing record.
    pub fn capacity_bytes(&self) -> u64 {
        self.bytes.capacity() as u64
            + match &self.metadata {
                Metadata::Rgba => 0,
                Metadata::Indexed(metadata) => metadata.capacity_bytes(),
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::contracts::WarningCode;
    use crate::prod::contract::error::ErrorCode;

    #[test]
    fn owned_metadata_counts_reserved_capacity_and_survives_source_mutation() {
        let mut palette = NormalizedPalette {
            rgba: vec![1, 2, 3, 255, 0, 0, 0, 0],
            transparent_index: Some(1),
        };
        let warnings = [ProcessWarning {
            code: WarningCode::PaletteTruncated,
            message: String::from("retained warning"),
        }];
        let needed = IndexedMetadata::required_bytes(&palette, &warnings);
        assert_eq!(
            IndexedMetadata::try_copy(&palette, &warnings, needed - 1)
                .err()
                .unwrap()
                .code,
            ErrorCode::MemoryLimit
        );
        let metadata = IndexedMetadata::try_copy(&palette, &warnings, needed).unwrap();
        palette.rgba.fill(9);
        assert_eq!(metadata.palette.rgba, [1, 2, 3, 255, 0, 0, 0, 0]);
        assert_eq!(metadata.palette.transparent_index, Some(1));
        assert_eq!(metadata.warnings, warnings);
        assert_eq!(metadata.capacity_bytes(), needed);
        let mut bytes = Vec::with_capacity(32);
        bytes.push(1);
        let image = ImageStage {
            dimensions: ImageDimensions::new(1, 1).unwrap(),
            bytes,
            metadata: Metadata::Indexed(metadata),
        };
        assert_eq!(image.capacity_bytes(), 32 + needed);
    }
}
