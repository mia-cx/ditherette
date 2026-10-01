//! How far a GPU result is from the CPU reference.

/// Byte differences between two packed RGBA8 results of the same size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parity {
    /// Largest absolute difference of any one channel byte.
    pub max_byte_difference: u8,
    /// Pixels with at least one differing channel.
    pub differing_pixels: usize,
    pub pixels: usize,
}

impl Parity {
    /// Compares `cpu` with `gpu`, pixel by pixel.
    pub fn between(cpu: &[u8], gpu: &[u8]) -> Self {
        assert_eq!(cpu.len(), gpu.len(), "results differ in size");
        let mut parity = Self {
            max_byte_difference: 0,
            differing_pixels: 0,
            pixels: cpu.len() / 4,
        };
        for (a, b) in cpu.chunks_exact(4).zip(gpu.chunks_exact(4)) {
            let largest = a
                .iter()
                .zip(b)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap_or(0);
            parity.max_byte_difference = parity.max_byte_difference.max(largest);
            parity.differing_pixels += usize::from(largest > 0);
        }
        parity
    }

    pub fn exact(&self) -> bool {
        self.differing_pixels == 0
    }
}
