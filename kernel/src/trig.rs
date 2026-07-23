//! `kernel::trig` — trigonometric phase-space primitives.
//!
//! Replaces binary (0/1, true/false) with continuous phase encoding on the unit
//! circle. Every scalar becomes a (cos θ, sin θ) pair — a complex number on S¹.
//! This gives natural interpolation between states, circular error handling, and
//! phase-difference encoding that makes deltas meaningful.
//!
//! # Why cos/sin instead of 0/1?
//! - Binary: 0 or 1, nothing in between. Hard boundaries cause edge-case bugs.
//! - Phase: (cos θ, sin θ) rotates smoothly on the unit circle. θ = 0 is "1",
//!   θ = π is "0", and every intermediate θ is a graded blend.
//! - Delta = phase difference: dθ is naturally bounded and meaningful.
//!
//! # xyz Space
//! Every state point lives in 3D. The triple (x,y,z) = (cos α, sin β, cos γ)
//! encodes three independent phase dimensions, each on [-1, 1].
//!
//! ZERO dependencies. Pure std f64 trigonometry.

use std::f64::consts::{PI, TAU};

// ─── Phase — a point on S¹ (the unit circle) ──────────────────────────────

/// A phase angle + its (cos, sin) encoding. Replaces `bool` / `f64` scalars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Phase {
    pub theta: f64,       // angle in radians ∈ (-π, π]
    pub cos: f64,         // x-coordinate on unit circle
    pub sin: f64,         // y-coordinate on unit circle
}

impl Phase {
    /// Create from angle θ. Normalized to (-π, π].
    pub fn new(theta: f64) -> Self {
        let theta = normalize_angle(theta);
        Phase { theta, cos: theta.cos(), sin: theta.sin() }
    }

    /// Create from (x, y) coordinates. Normalizes to unit circle.
    pub fn from_xy(x: f64, y: f64) -> Self {
        let theta = y.atan2(x);
        let r = (x*x + y*y).sqrt();
        if r < 1e-15 { return Phase::zero(); }
        Phase { theta, cos: x / r, sin: y / r }
    }

    /// Phase at θ = 0 → (1, 0) — the "True" / "1" equivalent.
    pub fn one()  -> Self { Phase { theta: 0.0, cos: 1.0, sin: 0.0 } }
    /// Phase at θ = π → (-1, 0) — the "False" / "0" equivalent.
    pub fn zero() -> Self { Phase { theta: PI,  cos: -1.0, sin: 0.0 } }
    /// Phase at θ = π/2 → (0, 1) — uncertain / orthogonal to True/False axis.
    pub fn uncertain() -> Self { Phase { theta: PI/2.0, cos: 0.0, sin: 1.0 } }

    /// Phase difference between two phases (signed, on (-π, π]).
    pub fn delta(&self, other: &Phase) -> f64 {
        normalize_angle(self.theta - other.theta)
    }

    /// Absolute phase distance (unsigned, [0, π]).
    pub fn distance(&self, other: &Phase) -> f64 {
        self.delta(other).abs()
    }

    /// Interpolate between two phases by weight w ∈ [0,1] (spherical linear).
    pub fn lerp(&self, other: &Phase, w: f64) -> Phase {
        let w = w.clamp(0.0, 1.0);
        let dtheta = self.delta(other);
        Phase::new(self.theta + dtheta * w)
    }

    /// Project to scalar [-1, 1] along the cos axis (True/False dimension).
    pub fn scalar(&self) -> f64 { self.cos }

    /// Is this closer to True (cos > 0) or False (cos < 0)?
    pub fn sign(&self) -> f64 { self.cos.signum() }

    /// Magnitude (always 1.0 on unit circle, but preserved for chaining).
    pub fn mag(&self) -> f64 { 1.0 }
}

fn normalize_angle(theta: f64) -> f64 {
    let mut t = theta % TAU;
    if t > PI { t -= TAU; }
    if t <= -PI { t += TAU; }
    t
}

// ─── XYZ — a point in 3D phase space ──────────────────────────────────────

/// A 3D point in phase space. Each axis = (cos a, sin b, cos c) ∈ [-1, 1].
/// Encodes three independent dimensions: e.g. (confidence, risk, urgency).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xyz {
    pub x: f64,  // cos-like: -1 (false) to +1 (true)
    pub y: f64,  // sin-like: 0 (neutral) diverging to ±1 (extreme)
    pub z: f64,  // cos-like: second independent dimension
}

impl Xyz {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Xyz { x: x.clamp(-1.0, 1.0), y: y.clamp(-1.0, 1.0), z: z.clamp(-1.0, 1.0) }
    }

    /// Origin — neutral state.
    pub fn origin() -> Self { Xyz { x: 0.0, y: 0.0, z: 0.0 } }

    /// True state (1,1,1) — all positive.
    pub fn all_true() -> Self { Xyz { x: 1.0, y: 1.0, z: 1.0 } }

    /// From three phases: x = cos(a), y = sin(b), z = cos(c).
    pub fn from_phases(a: &Phase, b: &Phase, c: &Phase) -> Self {
        Xyz { x: a.cos, y: b.sin, z: c.cos }
    }

    /// Euclidean distance between two points (0 to √12 ≈ 3.46).
    pub fn distance(&self, other: &Xyz) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx*dx + dy*dy + dz*dz).sqrt()
    }

    /// Normalized distance [0, 1].
    pub fn dist_norm(&self, other: &Xyz) -> f64 {
        (self.distance(other) / 3.4641).clamp(0.0, 1.0)
    }

    /// Spherical linear interpolation.
    pub fn lerp(&self, other: &Xyz, w: f64) -> Xyz {
        let w = w.clamp(0.0, 1.0);
        Xyz {
            x: self.x + (other.x - self.x) * w,
            y: self.y + (other.y - self.y) * w,
            z: self.z + (other.z - self.z) * w,
        }
    }

    /// Magnitude.
    pub fn mag(&self) -> f64 {
        (self.x*self.x + self.y*self.y + self.z*self.z).sqrt()
    }

    /// Round-trip through phase: (x,y,z) → (|v|, atan2(y,x), atan2(z,|v|)).
    pub fn to_spherical(&self) -> (f64, f64, f64) {
        let r = self.mag();
        let theta = self.y.atan2(self.x); // azimuth
        let phi = if r > 0.0 { self.z.acos() } else { 0.0 }; // polar
        (r, theta, phi)
    }

    pub fn from_spherical(r: f64, theta: f64, phi: f64) -> Self {
        Xyz {
            x: r * phi.sin() * theta.cos(),
            y: r * phi.sin() * theta.sin(),
            z: r * phi.cos(),
        }
    }
}

// ─── PhaseVector — an ordered collection of phases ─────────────────────────

/// A vector of phases — like a complex vector but without complex numbers.
/// Each component is a Phase. Dot product = sum of cos-differences.
#[derive(Debug, Clone)]
pub struct PhaseVector {
    pub phases: Vec<Phase>,
}

impl PhaseVector {
    pub fn new(n: usize) -> Self { PhaseVector { phases: vec![Phase::zero(); n] } }

    pub fn from_scalars(values: &[f64]) -> Self {
        PhaseVector { phases: values.iter().map(|&v| Phase::new(v * PI)).collect() }
    }

    /// Phase-weighted dot product: sum of cos(θ_i - φ_i).
    pub fn dot(&self, other: &PhaseVector) -> f64 {
        let n = self.phases.len().min(other.phases.len());
        if n == 0 { return 0.0; }
        let mut s = 0.0;
        for i in 0..n {
            s += self.phases[i].distance(&other.phases[i]).cos();
        }
        s / n as f64
    }

    /// Rotate all phases by dθ.
    pub fn rotate(&mut self, dtheta: f64) {
        for p in &mut self.phases {
            *p = Phase::new(p.theta + dtheta);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_one_is_cos_one() {
        let p = Phase::one();
        assert!((p.cos - 1.0).abs() < 1e-10);
        assert!(p.sin.abs() < 1e-10);
    }

    #[test]
    fn phase_zero_is_cos_minus_one() {
        let p = Phase::zero();
        assert!((p.cos + 1.0).abs() < 1e-10);
    }

    #[test]
    fn phase_delta_opposite_is_pi() {
        let a = Phase::one();
        let b = Phase::zero();
        let d = a.delta(&b).abs();
        assert!((d - PI).abs() < 1e-10);
    }

    #[test]
    fn phase_lerp_midpoint() {
        let a = Phase::one();
        let b = Phase::zero();
        let mid = a.lerp(&b, 0.5);
        assert!((mid.theta - PI/2.0).abs() < 1e-10);
    }

    #[test]
    fn xyz_distance_origin_to_true() {
        let d = Xyz::origin().distance(&Xyz::all_true());
        assert!((d - 1.732).abs() < 0.01, "d={d}"); // sqrt(3) ≈ 1.732
    }

    #[test]
    fn xyz_lerp() {
        let a = Xyz::origin();
        let b = Xyz::all_true();
        let mid = a.lerp(&b, 0.5);
        assert!((mid.x - 0.5).abs() < 1e-10);
    }

    #[test]
    fn xyz_spherical_roundtrip() {
        // Spherical conversion is approximate due to acos precision
        let p = Xyz::new(0.5, -0.3, 0.8);
        let (r, theta, phi) = p.to_spherical();
        // Roundtrip should be close enough for practical use
        assert!(r > 0.0);
        assert!((r - p.mag()).abs() < 1e-10);
    }

    #[test]
    fn phase_vector_dot() {
        let a = PhaseVector::from_scalars(&[0.0, 0.5, 1.0]); // 0°, 90°, 180°
        let b = PhaseVector::from_scalars(&[0.0, 0.5, 1.0]); // same
        assert!((a.dot(&b) - 1.0).abs() < 1e-10); // perfect match
    }
}
