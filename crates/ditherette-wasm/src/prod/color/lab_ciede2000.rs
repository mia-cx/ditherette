//! CIELAB plus CIEDE2000 distance.
//!
//! Conversion into Lab uses the same D65 CIELAB coordinates as `cielab`. The
//! distance function implements Sharma et al. CIEDE2000 with unit weighting
//! factors and backs the `CielabCiede2000` palette-matching metric.

pub fn ciede2000(lab1: [f32; 3], lab2: [f32; 3]) -> f32 {
    Ciede2000Pair::new(lab1, unprimed_chroma(lab1), lab2, unprimed_chroma(lab2)).distance()
}

#[inline(always)]
pub(crate) fn unprimed_chroma([_, a, b]: [f32; 3]) -> f32 {
    (a * a + b * b).sqrt()
}

/// The lightness term is a lower bound for the complete CIEDE2000 distance.
#[inline(always)]
pub(crate) fn lightness_lower_bound(l1: f32, l2: f32) -> f32 {
    let delta_l_prime = l2 - l1;
    let l_bar_prime = (l1 + l2) * 0.5;
    let offset = l_bar_prime - 50.0;
    let s_l = 1.0 + (0.015 * offset.powi(2)) / (20.0 + offset.powi(2)).sqrt();
    (delta_l_prime / s_l).abs()
}

/// Pair-dependent terms shared by the lower bound and complete CIEDE2000 score.
pub(crate) struct Ciede2000Pair {
    a1_prime: f32,
    b1: f32,
    a2_prime: f32,
    b2: f32,
    c1_prime: f32,
    c2_prime: f32,
    c_bar_prime: f32,
    l_term: f32,
    c_term: f32,
}

impl Ciede2000Pair {
    pub(crate) fn new(lab1: [f32; 3], c1: f32, lab2: [f32; 3], c2: f32) -> Self {
        let [l1, a1, b1] = lab1;
        let [l2, a2, b2] = lab2;

        let c_bar = (c1 + c2) * 0.5;
        let c_bar7 = c_bar.powi(7);
        let g = 0.5 * (1.0 - (c_bar7 / (c_bar7 + 25_f32.powi(7))).sqrt());

        let a1_prime = (1.0 + g) * a1;
        let a2_prime = (1.0 + g) * a2;
        let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
        let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();

        let delta_l_prime = l2 - l1;
        let delta_c_prime = c2_prime - c1_prime;
        let l_bar_prime = (l1 + l2) * 0.5;
        let c_bar_prime = (c1_prime + c2_prime) * 0.5;
        let s_l = 1.0
            + (0.015 * (l_bar_prime - 50.0).powi(2)) / (20.0 + (l_bar_prime - 50.0).powi(2)).sqrt();
        let s_c = 1.0 + 0.045 * c_bar_prime;

        Self {
            a1_prime,
            b1,
            a2_prime,
            b2,
            c1_prime,
            c2_prime,
            c_bar_prime,
            l_term: delta_l_prime / s_l,
            c_term: delta_c_prime / s_c,
        }
    }

    /// Since `R_T <= 0`, opposite chroma and hue directions make the cross term
    /// nonnegative. Otherwise, `|R_T| <= sqrt(3)` gives a `C² / 4` lower bound.
    #[inline(always)]
    pub(crate) fn lower_bound(&self) -> f32 {
        let chroma = self.c_term.abs();
        let hue_direction =
            self.a1_prime as f64 * self.b2 as f64 - self.b1 as f64 * self.a2_prime as f64;
        let direction_scale = self.c1_prime as f64 * self.c2_prime as f64;
        let reliable_direction = hue_direction.abs() > direction_scale * 1e-5;
        let delta_a_prime = self.a2_prime as f64 - self.a1_prime as f64;
        let delta_b = self.b2 as f64 - self.b1 as f64;
        let delta_c_prime = self.c2_prime as f64 - self.c1_prime as f64;
        let hue_squared = (delta_a_prime * delta_a_prime + delta_b * delta_b
            - delta_c_prime * delta_c_prime)
            .max(0.0);
        // This chord form equals |delta H'|. One percent of slack covers the
        // frozen f32 norm, angle, and sine rounding without weakening useful pruning.
        let hue_term_bound =
            (0.99 * hue_squared.sqrt()) as f32 / (1.0 + 0.015 * self.c_bar_prime * 1.93);
        let chroma_hue_bound = if reliable_direction
            && hue_direction.is_sign_positive() != self.c_term.is_sign_positive()
        {
            chroma * chroma + hue_term_bound * hue_term_bound
        } else {
            let unconstrained_hue = 0.866_025_4 * chroma;
            let minimizing_hue = hue_term_bound.max(unconstrained_hue);
            chroma * chroma + minimizing_hue * minimizing_hue
                - 1.732_050_8 * chroma * minimizing_hue
        };
        // Preserve margin for the frozen formula's independently rounded final sum.
        0.9999 * (self.l_term * self.l_term + chroma_hue_bound).sqrt()
    }

    pub(crate) fn distance(&self) -> f32 {
        self.prepare_hue().distance()
    }

    pub(crate) fn prepare_hue(&self) -> Ciede2000HuePair<'_> {
        let h1_prime = hue_degrees(self.a1_prime, self.b1);
        let h2_prime = hue_degrees(self.a2_prime, self.b2);
        let delta_h_prime = 2.0
            * (self.c1_prime * self.c2_prime).sqrt()
            * (0.5
                * delta_h_prime_degrees(self.c1_prime, self.c2_prime, h1_prime, h2_prime)
                    .to_radians())
            .sin();
        let h_bar_prime = mean_hue_degrees(self.c1_prime, self.c2_prime, h1_prime, h2_prime);
        let delta_theta = 30.0 * (-(((h_bar_prime - 275.0) / 25.0).powi(2))).exp();
        let c_bar_prime7 = self.c_bar_prime.powi(7);
        let r_c = 2.0 * (c_bar_prime7 / (c_bar_prime7 + 25_f32.powi(7))).sqrt();
        let r_t = -r_c * (2.0 * delta_theta).to_radians().sin();

        Ciede2000HuePair {
            pair: self,
            delta_h_prime,
            h_bar_prime,
            r_t,
        }
    }
}

pub(crate) struct Ciede2000HuePair<'a> {
    pair: &'a Ciede2000Pair,
    delta_h_prime: f32,
    h_bar_prime: f32,
    r_t: f32,
}

impl Ciede2000HuePair<'_> {
    /// Avoids the four `T` cosines with conservative hue-scale and rotation bounds.
    pub(crate) fn lower_bound(&self) -> f32 {
        let s_h_bound = 1.0 + 0.015 * self.pair.c_bar_prime * 1.93;
        let h_term_bound = self.delta_h_prime / s_h_bound;
        let c_term = self.pair.c_term;
        let unconstrained_h = -0.5 * self.r_t * c_term;
        let same_direction = unconstrained_h.signum() == h_term_bound.signum();
        let minimizing_h = if same_direction && unconstrained_h.abs() >= h_term_bound.abs() {
            unconstrained_h
        } else {
            h_term_bound
        };
        (self.pair.l_term * self.pair.l_term
            + c_term * c_term
            + minimizing_h * minimizing_h
            + self.r_t * c_term * minimizing_h)
            .sqrt()
    }

    pub(crate) fn distance(&self) -> f32 {
        let pair = self.pair;
        let h_bar_prime = self.h_bar_prime;

        let t = 1.0 - 0.17 * (h_bar_prime - 30.0).to_radians().cos()
            + 0.24 * (2.0 * h_bar_prime).to_radians().cos()
            + 0.32 * (3.0 * h_bar_prime + 6.0).to_radians().cos()
            - 0.20 * (4.0 * h_bar_prime - 63.0).to_radians().cos();
        let s_h = 1.0 + 0.015 * pair.c_bar_prime * t;
        let h_term = self.delta_h_prime / s_h;

        (pair.l_term * pair.l_term
            + pair.c_term * pair.c_term
            + h_term * h_term
            + self.r_t * pair.c_term * h_term)
            .sqrt()
    }
}

fn hue_degrees(a: f32, b: f32) -> f32 {
    if a == 0.0 && b == 0.0 {
        0.0
    } else {
        b.atan2(a).to_degrees().rem_euclid(360.0)
    }
}

fn delta_h_prime_degrees(c1_prime: f32, c2_prime: f32, h1_prime: f32, h2_prime: f32) -> f32 {
    if c1_prime * c2_prime == 0.0 {
        0.0
    } else if (h2_prime - h1_prime).abs() <= 180.0 {
        h2_prime - h1_prime
    } else if h2_prime <= h1_prime {
        h2_prime - h1_prime + 360.0
    } else {
        h2_prime - h1_prime - 360.0
    }
}

fn mean_hue_degrees(c1_prime: f32, c2_prime: f32, h1_prime: f32, h2_prime: f32) -> f32 {
    if c1_prime * c2_prime == 0.0 {
        h1_prime + h2_prime
    } else if (h1_prime - h2_prime).abs() <= 180.0 {
        (h1_prime + h2_prime) * 0.5
    } else if h1_prime + h2_prime < 360.0 {
        (h1_prime + h2_prime + 360.0) * 0.5
    } else {
        (h1_prime + h2_prime - 360.0) * 0.5
    }
}
