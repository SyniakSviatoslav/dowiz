//! spherical.rs — Legendre polynomials, spherical harmonics, Lebedev-type
//! quadrature, and the crystal structure factor S(k).
//!
//! Item #8 of screenshot-batch-2 ("AI model for predicting crystal properties
//! in reciprocal space"). The pipeline in that post runs:
//!   Structure → Structure Factor S(k) → reciprocal space → spherical
//!   harmonics / Legendre polynomials → Lebedev-grid sampling → crystal feature.
//! The kernel already had `crystal`/`academia` (the lattice side) but none of
//! the *reciprocal-space* math — this closes that gap.
//!
//! # Why it fits the rewrite law
//! Spherical harmonics are literally rotations on the sphere — the same
//! "geometry over algebra" substrate as `trig::Phase`/`trig::Xyz`, lifted to
//! the angular momentum ladder. Recurrences are computed once into a value,
//! not re-derived (n(0)-style reuse), and every function is deterministic and
//! fail-closed at the boundary.

use crate::spectral::Complex;

// ─── Legendre polynomials ──────────────────────────────────────────────

/// Legendre polynomial P_l(x) by Bonnet's recurrence
/// `(l+1)P_{l+1} = (2l+1)xP_l − lP_{l-1}`. Deterministic, stable for |x| ≤ 1.
pub fn legendre(l: usize, x: f64) -> f64 {
    if l == 0 {
        return 1.0;
    }
    if l == 1 {
        return x;
    }
    let mut p_prev = 1.0; // P_0
    let mut p_curr = x; // P_1
    for k in 1..l {
        let k = k as f64;
        let p_next = ((2.0 * k + 1.0) * x * p_curr - k * p_prev) / (k + 1.0);
        p_prev = p_curr;
        p_curr = p_next;
    }
    p_curr
}

/// Associated Legendre polynomial P_l^m(x) for 0 ≤ m ≤ l, via the standard
/// three-term recurrence on the normalized (geophysical) convention.
/// `x` is cos(θ) ∈ [−1, 1]. Returns `None` for m > l.
pub fn assoc_legendre(l: usize, m: usize, x: f64) -> Option<f64> {
    if m > l {
        return None;
    }
    // P_m^m = (−1)^m (2m−1)!! (1−x²)^{m/2}
    let mut pmm = 1.0;
    if m > 0 {
        let somx2 = (1.0 - x) * (1.0 + x); // 1 − x² (stable near |x|≈1)
        let mut fact = 1.0;
        for _ in 0..m {
            pmm *= -fact * somx2.sqrt();
            fact += 2.0;
        }
    }
    if l == m {
        return Some(pmm);
    }
    let mut pmmp1 = x * (2 * m + 1) as f64 * pmm; // P_{m+1}^m
    if l == m + 1 {
        return Some(pmmp1);
    }
    let mut pll = 0.0;
    for ll in (m + 2)..=l {
        let ll = ll as f64;
        let m = m as f64;
        pll = ((2.0 * ll - 1.0) * x * pmmp1 - (ll + m - 1.0) * pmm) / (ll - m);
        pmm = pmmp1;
        pmmp1 = pll;
    }
    Some(pll)
}

// ─── Spherical harmonics ────────────────────────────────────────────────

/// Real (tesseral) spherical harmonic Y_l^m(θ, φ).
///
/// Conventions: θ = polar angle (colatitude) ∈ [0, π], φ = azimuth ∈ [0, 2π].
/// For m ≥ 0: `Y_l^m = N · P_l^m(cosθ) · cos(mφ)`;
/// for m < 0: `Y_l^m = N · P_l^{|m|}(cosθ) · sin(|m|φ)`.
/// Returns `None` for |m| > l.
pub fn spherical_harmonic(l: usize, m: i64, theta: f64, phi: f64) -> Option<f64> {
    let am = m.unsigned_abs() as usize;
    if am > l {
        return None;
    }
    let p = assoc_legendre(l, am, theta.cos())?;
    // Normalization for real form (Condon–Shortley phase folded into P_l^m).
    let k = l - am;
    let norm = {
        let mut n = ((2 * l + 1) as f64) / (4.0 * core::f64::consts::PI);
        for j in 0..am {
            n /= ((l + j + 1) * (l - j)) as f64;
        }
        // ε_m = 2 for m != 0 (real form), 1 for m == 0.
        let eps = if am == 0 { 1.0 } else { 2.0 };
        (eps * n).sqrt()
    };
    let az = if m < 0 {
        (am as f64 * phi).sin()
    } else {
        (am as f64 * phi).cos()
    };
    // Fold the Condon–Shortley (−1)^m phase (geophysical real form).
    let phase = if am % 2 == 1 { -1.0 } else { 1.0 };
    Some(norm * p * az * phase)
}

// ─── Lebedev-type quadrature ────────────────────────────────────────────

/// A quadrature node (point on the unit sphere + weight).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LebedevNode {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

/// Octahedral (degree-3-exact) quadrature: 6 points at ±e_i, weight 4π/6.
/// This is the minimal exact rule for l ≤ 3 and the correct "seed" for the
/// Lebedev hierarchy. (Higher Lebedev–Laikov orders are larger published
/// tables — out of scope for a zero-dep seed; this rule integrates l ≤ 3
/// spherical harmonics exactly.)
pub fn lebedev_octahedral() -> Vec<LebedevNode> {
    let w = 4.0 * core::f64::consts::PI / 6.0;
    let axes: [(f64, f64, f64); 6] = [
        (1.0, 0.0, 0.0), (-1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0), (0.0, -1.0, 0.0),
        (0.0, 0.0, 1.0), (0.0, 0.0, -1.0),
    ];
    axes.iter()
        .map(|&(x, y, z)| LebedevNode { x, y, z, w })
        .collect()
}

/// Integrate a function over the unit sphere using a Lebedev grid.
pub fn integrate_sphere(f: impl Fn(f64, f64) -> f64, grid: &[LebedevNode]) -> f64 {
    grid.iter()
        .map(|n| {
            let phi = n.y.atan2(n.x);
            let theta = n.z.acos();
            f(theta, phi) * n.w
        })
        .sum()
}

// ─── Structure factor S(k) ──────────────────────────────────────────────

/// Crystal structure factor in reciprocal space:
/// `S(k) = (1/Ω) Σ_j f_j(k) · e^{−2πi k·r_j}`.
///
/// `positions` are fractional/direct-space coordinates (Å); `form_factors`
/// are the atomic scattering factors `f_j(k)` (default 1.0 if empty, i.e.
/// point atoms). Returns the complex structure factor (unnormalized by Ω —
/// the caller supplies the volume).
pub fn structure_factor(
    k: [f64; 3],
    positions: &[[f64; 3]],
    form_factors: &[f64],
) -> Complex {
    let mut acc = Complex::new(0.0, 0.0);
    for (j, r) in positions.iter().enumerate() {
        let f = if form_factors.is_empty() {
            1.0
        } else {
            form_factors[j.min(form_factors.len() - 1)]
        };
        let kdotr = k[0] * r[0] + k[1] * r[1] + k[2] * r[2];
        let phase = -2.0 * core::f64::consts::PI * kdotr;
        acc = acc.add(Complex::new(f * phase.cos(), f * phase.sin()));
    }
    acc
}

/// Intensity |S(k)|² — the directly observable diffraction quantity.
pub fn structure_factor_intensity(k: [f64; 3], positions: &[[f64; 3]], form_factors: &[f64]) -> f64 {
    let s = structure_factor(k, positions, form_factors);
    s.re * s.re + s.im * s.im
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legendre_known_values() {
        assert!((legendre(0, 0.5) - 1.0).abs() < 1e-12);
        assert!((legendre(1, 0.5) - 0.5).abs() < 1e-12);
        assert!((legendre(2, 0.5) - (1.5 * 0.25 - 0.5)).abs() < 1e-12);
        assert!((legendre(3, 0.0)).abs() < 1e-12); // P_3(0) = 0
        assert!((legendre(2, 1.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn assoc_legendre_reduces_to_legendre_at_m0() {
        for l in 0..5 {
            let a = assoc_legendre(l, 0, 0.3).unwrap();
            assert!((a - legendre(l, 0.3)).abs() < 1e-9, "l={l}");
        }
    }

    #[test]
    fn assoc_legendre_rejects_m_gt_l() {
        assert_eq!(assoc_legendre(2, 3, 0.5), None);
    }

    #[test]
    fn y00_is_constant() {
        // Y_0^0 = 1/√(4π).
        let y = spherical_harmonic(0, 0, 0.7, 1.3).unwrap();
        let expect = 1.0 / (4.0 * core::f64::consts::PI).sqrt();
        assert!((y - expect).abs() < 1e-12, "got {y}, want {expect}");
    }

    #[test]
    fn spherical_harmonic_rejects_bad_m() {
        assert_eq!(spherical_harmonic(1, 2, 0.5, 0.0), None);
        assert_eq!(spherical_harmonic(1, -2, 0.5, 0.0), None);
    }

    #[test]
    fn lebedev_integrates_constant_to_4pi() {
        let grid = lebedev_octahedral();
        let s = integrate_sphere(|_, _| 1.0, &grid);
        assert!((s - 4.0 * core::f64::consts::PI).abs() < 1e-12, "got {s}");
    }

    #[test]
    fn lebedev_integrates_l3_harmonic_to_zero() {
        // Odd spherical harmonics integrate to zero over the sphere; the
        // octahedral rule is exact for l ≤ 3, so Y_3^1 must integrate to ~0.
        let grid = lebedev_octahedral();
        let s = integrate_sphere(|t, p| spherical_harmonic(3, 1, t, p).unwrap(), &grid);
        assert!(s.abs() < 1e-9, "got {s}");
    }

    #[test]
    fn structure_factor_body_centered_extinction() {
        // BCC: two atoms at (0,0,0) and (0.5,0.5,0.5), equal form factors.
        // For k = (1,0,0): S = 1 + e^{−πi} = 1 − 1 = 0 (systematic absence).
        let pos = [[0.0, 0.0, 0.0], [0.5, 0.5, 0.5]];
        let s = structure_factor([1.0, 0.0, 0.0], &pos, &[]);
        assert!(s.abs() < 1e-9, "BCC h=1 should be extinct, got {s:?}");

        // For k = (2,0,0): S = 1 + e^{−2πi} = 2 (constructive).
        let s2 = structure_factor([2.0, 0.0, 0.0], &pos, &[]);
        assert!((s2.re - 2.0).abs() < 1e-9 && s2.im.abs() < 1e-9, "got {s2:?}");
    }

    #[test]
    fn structure_factor_intensity_nonnegative() {
        let pos = [[0.0, 0.0, 0.0], [0.5, 0.5, 0.5], [0.25, 0.0, 0.0]];
        let i = structure_factor_intensity([1.0, 2.0, 3.0], &pos, &[]);
        assert!(i >= 0.0);
    }
}
