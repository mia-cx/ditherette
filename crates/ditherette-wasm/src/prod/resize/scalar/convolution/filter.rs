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

    /// Opt into reordered separable sums for fixed-support shrinking by up to 2x per axis.
    /// Lanczos3 accepts bounded rounding differences; custom kernels retain direct accumulation.
    fn allows_fixed_separable_shrink(&self) -> bool {
        false
    }

    /// Opt into bounded scratch blocks without changing scale-aware accumulation order.
    fn allows_scale_aware_blocks(&self) -> bool {
        false
    }
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
