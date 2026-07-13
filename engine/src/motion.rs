//! FE-08 — Motion field (critically-damped spring easing).
//!
//! RED→GREEN GATE (per blueprint): ζ=1 → monotone, no overshoot; ζ<1 → bounces;
//! heat-kernel stagger ripples outward with delay ∝ graph distance.
//!
//! Per-property spring integrator: `ẍ + 2ζω ẋ + ω²x = ω²·x_target`.
//! (ω,ζ) derived from `motion.ts` tension/friction:
//!   ω = √(k/m),  ζ = d / (2√(k·m))   (with m=1: ω=√k, ζ=d/2√k).
//! Regions: snappy ζ=1 ω≈30 (τ_s≈130ms), fluid ζ≈0.65–0.8, playful ζ≈0.35.
//! Semi-implicit Euler (stable). Money is NEVER a field channel (see FE-09).

/// A single critically-damped-capable spring on one scalar property.
#[derive(Debug, Clone)]
pub struct Spring {
    /// Current value.
    pub x: f32,
    /// Current velocity.
    pub v: f32,
    /// Target value.
    pub target: f32,
    /// Angular frequency ω (rad/s).
    omega: f32,
    /// Damping ratio ζ.
    zeta: f32,
}

impl Spring {
    /// `tension`→k, `friction`→d (mass m=1, so ω=√k, ζ=d/(2√k)).
    pub fn new(tension: f32, friction: f32, initial: f32) -> Self {
        let k = tension.max(0.0);
        let d = friction.max(0.0);
        let omega = k.sqrt();
        let zeta = if omega > 0.0 {
            (d / (2.0 * omega)).clamp(0.0, 4.0)
        } else {
            1.0
        };
        Spring {
            x: initial,
            v: 0.0,
            target: initial,
            omega,
            zeta,
        }
    }

    /// Step the integrator by fixed `dt` (semi-implicit Euler — stable for
    /// stiff springs when dt ≤ ~1/ω). Internally substeps so ω·dt_sub ≤ 0.1
    /// (keeps the discrete ζ=1 solution monotone, overshoot < 1e-3).
    pub fn step(&mut self, dt: f32) {
        let w = self.omega;
        let sub = if w > 0.0 {
            ((w * dt) / 0.1).ceil().max(1.0) as usize
        } else {
            1
        };
        let h = dt / sub as f32;
        for _ in 0..sub {
            let accel = w * w * (self.target - self.x) - 2.0 * self.zeta * w * self.v;
            self.v += accel * h;
            self.x += self.v * h;
        }
    }

    /// Snappy preset (ζ=1, ω≈30) — used for press/enter emphasis.
    pub fn snappy(initial: f32) -> Self {
        Spring::new(900.0, 60.0, initial) // friction = 2·ζ·√k = 60 → exact ζ=1
    }

    /// Fluid preset (ζ≈0.7).
    pub fn fluid(initial: f32) -> Self {
        Spring::new(700.0, 2.0 * 0.7 * 700.0_f32.sqrt(), initial)
    }

    /// Playful preset (ζ≈0.35, bounces).
    pub fn playful(initial: f32) -> Self {
        Spring::new(700.0, 2.0 * 0.35 * 700.0_f32.sqrt(), initial)
    }

    pub fn zeta(&self) -> f32 {
        self.zeta
    }
}

/// Heat-kernel stagger delay (FE-08 global transitions): delay of node `j`
/// relative to a source node, ∝ graph distance / √α. Returns seconds.
pub fn heat_kernel_delay(graph_distance: f32, alpha: f32) -> f32 {
    if alpha <= 0.0 {
        return f32::INFINITY;
    }
    graph_distance / alpha.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    // RED→GREEN: ζ=1 is monotone — no overshoot past the target.
    #[test]
    fn zeta_one_no_overshoot() {
        let mut s = Spring::snappy(0.0);
        s.target = 1.0;
        let mut max_x = 0.0f32;
        for _ in 0..600 {
            // 60Hz, 10s of sim
            s.step(1.0 / 60.0);
            max_x = max_x.max(s.x);
        }
        assert!(
            max_x <= 1.0 + 1e-3,
            "ζ=1 must NOT overshoot target; max_x={max_x}"
        );
        assert!(
            (s.x - 1.0).abs() < 1e-2,
            "ζ=1 settles exactly at target; x={}",
            s.x
        );
        assert_eq!(s.zeta(), 1.0);
    }

    // ζ<1 overshoots (bounces).
    #[test]
    fn zeta_below_one_overshoots() {
        let mut s = Spring::playful(0.0);
        s.target = 1.0;
        let mut max_x = 0.0f32;
        for _ in 0..600 {
            s.step(1.0 / 60.0);
            max_x = max_x.max(s.x);
        }
        assert!(
            max_x > 1.0,
            "ζ<1 must overshoot target (bounce); max_x={max_x}"
        );
    }

    // Settles: velocity and distance-to-target both decay to ~0.
    #[test]
    fn snappy_settles_quickly() {
        let mut s = Spring::snappy(0.0);
        s.target = 1.0;
        // τ_s ≈ 2/(ζω) ≈ 130ms → settle well within 1s.
        for _ in 0..60 {
            s.step(1.0 / 60.0);
        }
        assert!((s.x - 1.0).abs() < 0.05, "settled within ~1s; x={}", s.x);
    }

    // Heat-kernel delay grows with graph distance, shrinks with α.
    #[test]
    fn heat_kernel_delay_monotone() {
        let near = heat_kernel_delay(1.0, 4.0);
        let far = heat_kernel_delay(4.0, 4.0);
        assert!(far > near, "farther node delays more");
        let faster = heat_kernel_delay(4.0, 16.0);
        assert!(faster < far, "higher α (diffusion) → shorter delay");
    }

    // Money guard: a Spring is a FIELD channel only — never feed it money.
    // (compile-time proof: Spring::step takes no money type; this documents
    //  the invariant. The FE-09 module enforces the type-level boundary.)
    #[test]
    fn spring_is_field_channel_not_money() {
        let mut s = Spring::snappy(10.0);
        s.target = 20.0; // a screen coordinate, NOT a monetary amount
        s.step(1.0 / 60.0);
        assert!(s.x > 10.0, "field channel interpolates freely");
    }
}
