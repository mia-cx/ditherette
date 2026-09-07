//! Shared finite-support reconstruction-filter contracts.

/// Support policy for convolution kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportPolicy {
    /// Use the kernel's native support radius for every scale.
    Fixed,
    /// Widen support during minification to preserve more source information.
    ScaleAware,
}

/// Finite-support one-dimensional reconstruction kernel.
pub trait ReconstructionKernel {
    /// Native support radius in source-pixel units before scale-aware widening.
    fn radius(&self) -> f64;

    /// Return the unnormalized kernel weight at `distance` source pixels.
    fn weight(&self, distance: f64) -> f64;
}

pub(super) fn axis_kernel_scale(
    source_len: u32,
    output_len: u32,
    support_policy: SupportPolicy,
) -> f64 {
    match support_policy {
        SupportPolicy::Fixed => 1.0,
        SupportPolicy::ScaleAware => (f64::from(source_len) / f64::from(output_len)).max(1.0),
    }
}
