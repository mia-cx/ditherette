//! CIELAB plus CIEDE2000 distance spec.
//!
//! Conversion into Lab uses the same D65 CIELAB coordinates as `cielab`. The
//! distance function implements Sharma et al. CIEDE2000 with unit weighting
//! factors, suitable for later nearest-palette matching.

pub fn ciede2000(lab1: [f32; 3], lab2: [f32; 3]) -> f32 {
    let [l1, a1, b1] = lab1;
    let [l2, a2, b2] = lab2;

    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_bar = (c1 + c2) * 0.5;
    let c_bar7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar7 / (c_bar7 + 25_f32.powi(7))).sqrt());

    let a1_prime = (1.0 + g) * a1;
    let a2_prime = (1.0 + g) * a2;
    let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
    let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();
    let h1_prime = hue_degrees(a1_prime, b1);
    let h2_prime = hue_degrees(a2_prime, b2);

    let delta_l_prime = l2 - l1;
    let delta_c_prime = c2_prime - c1_prime;
    let delta_h_prime = 2.0
        * (c1_prime * c2_prime).sqrt()
        * (0.5 * delta_h_prime_degrees(c1_prime, c2_prime, h1_prime, h2_prime).to_radians()).sin();

    let l_bar_prime = (l1 + l2) * 0.5;
    let c_bar_prime = (c1_prime + c2_prime) * 0.5;
    let h_bar_prime = mean_hue_degrees(c1_prime, c2_prime, h1_prime, h2_prime);

    let t = 1.0 - 0.17 * (h_bar_prime - 30.0).to_radians().cos()
        + 0.24 * (2.0 * h_bar_prime).to_radians().cos()
        + 0.32 * (3.0 * h_bar_prime + 6.0).to_radians().cos()
        - 0.20 * (4.0 * h_bar_prime - 63.0).to_radians().cos();
    let delta_theta = 30.0 * (-(((h_bar_prime - 275.0) / 25.0).powi(2))).exp();
    let c_bar_prime7 = c_bar_prime.powi(7);
    let r_c = 2.0 * (c_bar_prime7 / (c_bar_prime7 + 25_f32.powi(7))).sqrt();
    let s_l =
        1.0 + (0.015 * (l_bar_prime - 50.0).powi(2)) / (20.0 + (l_bar_prime - 50.0).powi(2)).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_prime;
    let s_h = 1.0 + 0.015 * c_bar_prime * t;
    let r_t = -r_c * (2.0 * delta_theta).to_radians().sin();

    let l_term = delta_l_prime / s_l;
    let c_term = delta_c_prime / s_c;
    let h_term = delta_h_prime / s_h;

    (l_term * l_term + c_term * c_term + h_term * h_term + r_t * c_term * h_term).sqrt()
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
