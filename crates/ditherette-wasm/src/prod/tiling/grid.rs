//! Output tile-grid planning.

use crate::image::ImageDimensions;

use super::Tile;

/// A row-major rectangular tile grid covering an output image exactly once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileGrid {
    output_dimensions: ImageDimensions,
    tiles: Vec<Tile>,
}

impl TileGrid {
    /// Splits output space into row-major tiles capped by `target_tile_width`
    /// and `target_tile_height`.
    pub fn new(
        output_dimensions: ImageDimensions,
        target_tile_width: u32,
        target_tile_height: u32,
    ) -> Option<Self> {
        if target_tile_width == 0 || target_tile_height == 0 {
            return None;
        }

        let mut tiles = Vec::new();
        let mut y_start = 0;
        while y_start < output_dimensions.height() {
            let y_end = y_start
                .saturating_add(target_tile_height)
                .min(output_dimensions.height());
            let mut x_start = 0;
            while x_start < output_dimensions.width() {
                let x_end = x_start
                    .saturating_add(target_tile_width)
                    .min(output_dimensions.width());
                tiles.push(Tile::new(x_start, y_start, x_end, y_end)?);
                x_start = x_end;
            }
            y_start = y_end;
        }

        Some(Self {
            output_dimensions,
            tiles,
        })
    }

    pub const fn output_dimensions(&self) -> ImageDimensions {
        self.output_dimensions
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
}
