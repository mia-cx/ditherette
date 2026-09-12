//! Packed image format definitions.
//!
//! These marker types describe the layout of flat channel arrays such as
//! `[r, g, b, a, r, g, b, a, ...]`. They intentionally do not destructure data
//! into per-pixel structs and do not implement color conversion; algorithms use
//! the constants here to index packed storage directly.

use super::pixel::StorageElement;

/// Describes a packed image format over a scalar storage type.
pub trait ImageFormat: 'static {
    type Storage: StorageElement;

    /// Number of scalar storage elements per logical pixel.
    const CHANNEL_COUNT: usize;

    /// Human-readable format name for diagnostics and benchmark IDs.
    const NAME: &'static str;
}

/// Packed RGBA with u8 channels in browser `ImageData` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rgba8 {}

impl ImageFormat for Rgba8 {
    type Storage = u8;

    const CHANNEL_COUNT: usize = 4;
    const NAME: &'static str = "rgba8";
}

impl Rgba8 {
    pub const R: usize = 0;
    pub const G: usize = 1;
    pub const B: usize = 2;
    pub const A: usize = 3;
}

/// Packed RGB with u8 channels in red, green, blue order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rgb8 {}

impl ImageFormat for Rgb8 {
    type Storage = u8;

    const CHANNEL_COUNT: usize = 3;
    const NAME: &'static str = "rgb8";
}

impl Rgb8 {
    pub const R: usize = 0;
    pub const G: usize = 1;
    pub const B: usize = 2;
}

/// Packed gamma-encoded sRGB with f32 channels normalized to `0..=1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Srgb32 {}

impl ImageFormat for Srgb32 {
    type Storage = f32;

    const CHANNEL_COUNT: usize = 3;
    const NAME: &'static str = "srgb32";
}

impl Srgb32 {
    pub const R: usize = 0;
    pub const G: usize = 1;
    pub const B: usize = 2;
}

/// Packed linear RGB with f32 channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinearRgb32 {}

impl ImageFormat for LinearRgb32 {
    type Storage = f32;

    const CHANNEL_COUNT: usize = 3;
    const NAME: &'static str = "linear-rgb32";
}

impl LinearRgb32 {
    pub const R: usize = 0;
    pub const G: usize = 1;
    pub const B: usize = 2;
}

/// Packed linear RGBA with f32 channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinearRgba32 {}

impl ImageFormat for LinearRgba32 {
    type Storage = f32;

    const CHANNEL_COUNT: usize = 4;
    const NAME: &'static str = "linear-rgba32";
}

impl LinearRgba32 {
    pub const R: usize = 0;
    pub const G: usize = 1;
    pub const B: usize = 2;
    pub const A: usize = 3;
}

/// Packed Oklab with f32 channels in L, a, b order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Oklab32 {}

impl ImageFormat for Oklab32 {
    type Storage = f32;

    const CHANNEL_COUNT: usize = 3;
    const NAME: &'static str = "oklab32";
}

impl Oklab32 {
    pub const L: usize = 0;
    pub const A: usize = 1;
    pub const B: usize = 2;
}

/// Packed Oklab with f32 L, a, b and alpha channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Oklaba32 {}

impl ImageFormat for Oklaba32 {
    type Storage = f32;

    const CHANNEL_COUNT: usize = 4;
    const NAME: &'static str = "oklaba32";
}

impl Oklaba32 {
    pub const L: usize = 0;
    pub const A: usize = 1;
    pub const B: usize = 2;
    pub const ALPHA: usize = 3;
}

/// Packed OKLCH with f32 lightness, chroma, and hue-radians channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Oklch32 {}

impl ImageFormat for Oklch32 {
    type Storage = f32;

    const CHANNEL_COUNT: usize = 3;
    const NAME: &'static str = "oklch32";
}

impl Oklch32 {
    pub const L: usize = 0;
    pub const C: usize = 1;
    pub const H: usize = 2;
}

/// Packed CIELAB with f32 L*, a*, b* channels using D65 white.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cielab32 {}

impl ImageFormat for Cielab32 {
    type Storage = f32;

    const CHANNEL_COUNT: usize = 3;
    const NAME: &'static str = "cielab32";
}

impl Cielab32 {
    pub const L: usize = 0;
    pub const A: usize = 1;
    pub const B: usize = 2;
}

/// Packed CIELCH with f32 lightness, chroma, and hue-radians channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cielch32 {}

impl ImageFormat for Cielch32 {
    type Storage = f32;

    const CHANNEL_COUNT: usize = 3;
    const NAME: &'static str = "cielch32";
}

impl Cielch32 {
    pub const L: usize = 0;
    pub const C: usize = 1;
    pub const H: usize = 2;
}

/// Packed full-range BT.601 YCbCr with f32 Y, Cb, Cr channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YCbCr32 {}

impl ImageFormat for YCbCr32 {
    type Storage = f32;

    const CHANNEL_COUNT: usize = 3;
    const NAME: &'static str = "ycbcr32";
}

impl YCbCr32 {
    pub const Y: usize = 0;
    pub const CB: usize = 1;
    pub const CR: usize = 2;
}

/// Packed single-channel palette index image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaletteIndex8 {}

impl ImageFormat for PaletteIndex8 {
    type Storage = u8;

    const CHANNEL_COUNT: usize = 1;
    const NAME: &'static str = "palette-index8";
}

impl PaletteIndex8 {
    pub const INDEX: usize = 0;
}
