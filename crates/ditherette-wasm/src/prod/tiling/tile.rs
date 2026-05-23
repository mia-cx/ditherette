//! Output rectangle tile geometry.

/// A half-open output rectangle: `x_start..x_end, y_start..y_end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    x_start: u32,
    y_start: u32,
    x_end: u32,
    y_end: u32,
}

impl Tile {
    /// Creates a non-empty half-open output tile.
    pub const fn new(x_start: u32, y_start: u32, x_end: u32, y_end: u32) -> Option<Self> {
        if x_start < x_end && y_start < y_end {
            Some(Self {
                x_start,
                y_start,
                x_end,
                y_end,
            })
        } else {
            None
        }
    }

    pub const fn x_start(self) -> u32 {
        self.x_start
    }

    pub const fn y_start(self) -> u32 {
        self.y_start
    }

    pub const fn x_end(self) -> u32 {
        self.x_end
    }

    pub const fn y_end(self) -> u32 {
        self.y_end
    }

    pub const fn width(self) -> u32 {
        self.x_end - self.x_start
    }

    pub const fn height(self) -> u32 {
        self.y_end - self.y_start
    }
}
