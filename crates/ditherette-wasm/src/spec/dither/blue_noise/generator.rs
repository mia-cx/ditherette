//! Offline naive void-and-cluster reference. See ../blue_noise.md for provenance.
//! Recompute every density and every Fourier coefficient directly; no cached filter state.

use serde::Serialize;
use std::f64::consts::{PI, TAU};

pub const SIDE: usize = 32;
pub const COUNT: usize = SIDE * SIDE;
pub const INITIAL_ONES: usize = 128;
pub const SEED: u32 = 0xd17e_e77e;
pub const SIGMA: f64 = 1.5;

/// Reproducible construction evidence, separate from performance measurements.
#[derive(Debug, Serialize)]
pub struct Construction {
    pub side: usize,
    pub seed: u32,
    pub sigma: f64,
    pub initial_ones: usize,
    pub relaxation_moves: usize,
    pub ranks: Vec<u16>,
    pub spectra: Vec<Spectrum>,
    pub passes: bool,
}

#[derive(Debug, Serialize)]
pub struct Spectrum {
    pub occupied: usize,
    pub low_mean_over_white: f64,
    pub high_mean_over_low: f64,
    pub peak_fraction: f64,
    pub angular_coefficient_of_variation: f64,
    pub parseval_relative_error: f64,
    pub passes: bool,
}

/// Fixed Fisher-Yates input: Numerical Recipes LCG, high-product mapping into 0..=i.
/// This defines site order only. The subsequent spectral checks establish field quality.
fn initial_pattern() -> [bool; COUNT] {
    let mut order: Vec<_> = (0..COUNT).collect();
    let mut state = SEED;
    for i in (1..COUNT).rev() {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let j = ((u64::from(state) * (i + 1) as u64) >> 32) as usize;
        order.swap(i, j);
    }
    let mut pattern = [false; COUNT];
    for &site in &order[..INITIAL_ONES] {
        pattern[site] = true;
    }
    pattern
}

/// Periodic Gaussian density, summed in row-major order over the chosen population.
pub fn density(pattern: &[bool; COUNT], site: usize, population: bool) -> f64 {
    let mut sum = 0.0;
    for (other, &occupied) in pattern.iter().enumerate() {
        if occupied != population {
            continue;
        }
        let dx = (site % SIDE).abs_diff(other % SIDE);
        let dy = (site / SIDE).abs_diff(other / SIDE);
        let dx = dx.min(SIDE - dx) as f64;
        let dy = dy.min(SIDE - dy) as f64;
        sum += (-(dx * dx + dy * dy) / (2.0 * SIGMA * SIGMA)).exp();
    }
    sum
}

/// Strict comparison makes equal-density ties retain the first row-major site.
fn extremum(pattern: &[bool; COUNT], candidates: bool, population: bool, maximum: bool) -> usize {
    let mut chosen = None;
    let mut best = if maximum {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    for (site, &occupied) in pattern.iter().enumerate() {
        if occupied != candidates {
            continue;
        }
        let value = density(pattern, site, population);
        if (maximum && value > best) || (!maximum && value < best) {
            chosen = Some(site);
            best = value;
        }
    }
    chosen.expect("ranking phase has a candidate site")
}

/// Relax the seeded pattern, then assign every rank using the three reference phases.
pub fn generate() -> Result<Construction, &'static str> {
    let mut pattern = initial_pattern();
    let mut moves = 0;
    loop {
        let cluster = extremum(&pattern, true, true, true);
        pattern[cluster] = false;
        let void = extremum(&pattern, false, true, false);
        pattern[void] = true;
        if cluster == void {
            break;
        }
        moves += 1;
        if moves == COUNT * COUNT {
            return Err("void-and-cluster relaxation did not converge");
        }
    }
    let initial = pattern;
    let mut ranks = vec![u16::MAX; COUNT];
    // Phase I removes the tightest occupied cluster, assigning descending low ranks.
    for rank in (0..INITIAL_ONES).rev() {
        let site = extremum(&pattern, true, true, true);
        pattern[site] = false;
        ranks[site] = rank as u16;
    }
    // Phase II restores the initial pattern and fills the largest voids up to half occupancy.
    pattern = initial;
    for rank in INITIAL_ONES..COUNT / 2 {
        let site = extremum(&pattern, false, true, false);
        pattern[site] = true;
        ranks[site] = rank as u16;
    }
    // Phase III removes the tightest clusters of the now-minority empty sites.
    for rank in COUNT / 2..COUNT {
        let site = extremum(&pattern, false, false, true);
        pattern[site] = true;
        ranks[site] = rank as u16;
    }
    let spectra = [
        COUNT / 8,
        COUNT / 4,
        COUNT / 2,
        3 * COUNT / 4,
        7 * COUNT / 8,
    ]
    .map(|occupied| spectrum(&ranks, occupied));
    let mut sorted = ranks.clone();
    sorted.sort_unstable();
    let permutation = sorted.iter().copied().eq(0..COUNT as u16);
    let passes = permutation && spectra.iter().all(|report| report.passes);
    Ok(Construction {
        side: SIDE,
        seed: SEED,
        sigma: SIGMA,
        initial_ones: INITIAL_ONES,
        relaxation_moves: moves,
        ranks,
        spectra: spectra.into(),
        passes,
    })
}

/// Direct DFT of a mean-centered binary threshold pattern. Frequencies use cycles per tile.
pub fn spectrum(ranks: &[u16], occupied: usize) -> Spectrum {
    assert_eq!(ranks.len(), COUNT);
    assert!((1..COUNT).contains(&occupied));
    let p = occupied as f64 / COUNT as f64;
    let white = COUNT as f64 * p * (1.0 - p);
    let mut low = (0.0, 0);
    let mut high = (0.0, 0);
    let mut sectors = [(0.0, 0); 4];
    let mut total = 0.0;
    let mut peak: f64 = 0.0;
    for ky in -(SIDE as i32) / 2..SIDE as i32 / 2 {
        for kx in -(SIDE as i32) / 2..SIDE as i32 / 2 {
            if kx == 0 && ky == 0 {
                continue;
            }
            let mut real = 0.0;
            let mut imaginary = 0.0;
            for (site, &rank) in ranks.iter().enumerate() {
                let sample = f64::from(u8::from(usize::from(rank) < occupied)) - p;
                let phase = TAU * (kx * (site % SIDE) as i32 + ky * (site / SIDE) as i32) as f64
                    / SIDE as f64;
                real += sample * phase.cos();
                imaginary -= sample * phase.sin();
            }
            let power = real * real + imaginary * imaginary;
            let radius = f64::from(kx * kx + ky * ky).sqrt();
            total += power;
            peak = peak.max(power);
            if radius <= 3.2 {
                low.0 += power;
                low.1 += 1;
            }
            if (8.0..=16.0).contains(&radius) {
                high.0 += power;
                high.1 += 1;
            }
            let angle = f64::from(ky).atan2(f64::from(kx)).rem_euclid(PI);
            let sector = ((angle / (PI / 4.0)) as usize).min(3);
            sectors[sector].0 += power;
            sectors[sector].1 += 1;
        }
    }
    let low_mean = low.0 / f64::from(low.1);
    let high_mean = high.0 / f64::from(high.1);
    let sector_means = sectors.map(|(power, count)| power / f64::from(count));
    let angular_mean = sector_means.iter().sum::<f64>() / 4.0;
    let angular_variance = sector_means
        .iter()
        .map(|value| (value - angular_mean).powi(2))
        .sum::<f64>()
        / 4.0;
    let low_mean_over_white = low_mean / white;
    let high_mean_over_low = high_mean / low_mean;
    let peak_fraction = peak / total;
    let angular_coefficient_of_variation = angular_variance.sqrt() / angular_mean;
    let parseval_relative_error = (total - COUNT as f64 * white).abs() / (COUNT as f64 * white);
    Spectrum {
        occupied,
        low_mean_over_white,
        high_mean_over_low,
        peak_fraction,
        angular_coefficient_of_variation,
        parseval_relative_error,
        passes: low_mean_over_white <= 0.2
            && high_mean_over_low >= 4.0
            && peak_fraction <= 0.1
            && angular_coefficient_of_variation <= 0.5
            && parseval_relative_error < 1e-10,
    }
}
