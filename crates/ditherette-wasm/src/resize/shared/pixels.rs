use crate::image::rgba;

pub(crate) fn copy_pixel_bytes(
    source_rgba: &[u8],
    source_offset: usize,
    output_rgba: &mut [u8],
    output_offset: usize,
) {
    output_rgba[output_offset..output_offset + rgba::RGBA_CHANNEL_COUNT]
        .copy_from_slice(&source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT]);
}
