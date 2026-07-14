//! kalman.rs — B1: the "Brain + Body" state-estimation organ (constant-velocity Kalman filter).
//!
//! THE SYNTHESIS. A Kalman filter is a continuous-state Hidden Markov Model with linear-Gaussian
//! dynamics: each step is **predict** (a geometric state transition — the *Body*, `x ← Fx`) then
//! **update** (a Bayesian correction from a noisy observation — the *Brain*, `x ← x + K(z − Hx)`).
//! Here the state is a courier's `[pos, vel]` per axis; the observation is a noisy GPS `pos`.
//!
//! WHY THIS EXISTS. `geo::ema_next(prev, z, α) = prev + α(z − prev)` is *exactly* the steady-state
//! 1-D (position-only, random-walk) Kalman update, with `α` the fixed-point Kalman gain `K*`. This
//! organ generalizes that degenerate case to a 2-state constant-velocity filter, so a courier's
//! position is smoothed AND its velocity is inferred — tracking through GPS noise the raw EMA can't.
//! `steady_state_gain_1d` closes the loop by deriving `K*` in closed form (proven == `ema_next`'s α).
//!
//! Pure, deterministic (fixed `dt`, no RNG, fixed op-order f64). Float dynamics — NEVER money.
//! The 2-D courier filter decouples into two independent per-axis filters (exact for a
//! constant-velocity model with axis-independent noise), so no matrix inversion is needed.

/// One axis of a constant-velocity Kalman filter: state `[pos, vel]` + its 2×2 covariance `P`.
/// Transition `F = [[1, dt],[0, 1]]`, measurement `H = [1, 0]` (observe position only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisKalman {
    pub pos: f64,
    pub vel: f64,
    // symmetric covariance P = [[p_pp, p_pv], [p_vp, p_vv]]
    p_pp: f64,
    p_pv: f64,
    p_vp: f64,
    p_vv: f64,
}

impl AxisKalman {
    /// Initialize at `pos0` (from the first fix), zero velocity, with a large initial covariance
    /// `p0` (we are unsure — the first measurements will pull it in).
    pub fn new(pos0: f64, p0: f64) -> Self {
        AxisKalman { pos: pos0, vel: 0.0, p_pp: p0, p_pv: 0.0, p_vp: 0.0, p_vv: p0 }
    }

    /// PREDICT (the Body): advance the constant-velocity model by `dt`, inflating covariance by the
    /// process noise `q_pos`/`q_vel`. `x ← Fx`, `P ← F P Fᵀ + Q`.
    pub fn predict(&mut self, dt: f64, q_pos: f64, q_vel: f64) {
        self.pos += self.vel * dt;
        // P' = F P Fᵀ + Q, with F = [[1,dt],[0,1]]:
        let p_pp = self.p_pp + dt * (self.p_pv + self.p_vp) + dt * dt * self.p_vv + q_pos;
        let p_pv = self.p_pv + dt * self.p_vv;
        let p_vp = self.p_vp + dt * self.p_vv;
        let p_vv = self.p_vv + q_vel;
        self.p_pp = p_pp;
        self.p_pv = p_pv;
        self.p_vp = p_vp;
        self.p_vv = p_vv;
    }

    /// UPDATE (the Brain): fold in a noisy position measurement `z` (variance `r`) via the Kalman
    /// gain. `y = z − Hx`, `S = HPHᵀ + r`, `K = PHᵀ/S`, `x ← x + Ky`, `P ← (I − KH)P`.
    pub fn update(&mut self, z: f64, r: f64) {
        let innov = z - self.pos; // H = [1,0] ⇒ Hx = pos
        let s = self.p_pp + r; // S = P_pp + r
        let k_p = self.p_pp / s;
        let k_v = self.p_vp / s;
        self.pos += k_p * innov;
        self.vel += k_v * innov;
        // P ← (I − KH)P, with I − KH = [[1−k_p, 0],[−k_v, 1]]:
        let p_pp = (1.0 - k_p) * self.p_pp;
        let p_pv = (1.0 - k_p) * self.p_pv;
        let p_vp = self.p_vp - k_v * self.p_pp;
        let p_vv = self.p_vv - k_v * self.p_pv;
        self.p_pp = p_pp;
        self.p_pv = p_pv;
        self.p_vp = p_vp;
        self.p_vv = p_vv;
    }
}

/// A 2-D courier tracker: two independent per-axis constant-velocity Kalman filters (lat, lng).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CourierKalman {
    pub lat: AxisKalman,
    pub lng: AxisKalman,
    /// process-noise + measurement-noise config (fixed ⇒ deterministic).
    q_pos: f64,
    q_vel: f64,
    r: f64,
}

impl CourierKalman {
    /// Seed from the first GPS fix. `p0` = initial position uncertainty; `q_*` = process noise
    /// (how much we let the model drift per step); `r` = GPS measurement variance.
    pub fn new(lat0: f64, lng0: f64, p0: f64, q_pos: f64, q_vel: f64, r: f64) -> Self {
        CourierKalman {
            lat: AxisKalman::new(lat0, p0),
            lng: AxisKalman::new(lng0, p0),
            q_pos,
            q_vel,
            r,
        }
    }

    /// Advance the model by `dt` seconds (no new measurement) — e.g. to extrapolate between fixes.
    pub fn predict(&mut self, dt: f64) {
        self.lat.predict(dt, self.q_pos, self.q_vel);
        self.lng.predict(dt, self.q_pos, self.q_vel);
    }

    /// Fold in a noisy GPS fix `(lat, lng)`.
    pub fn update(&mut self, lat_obs: f64, lng_obs: f64) {
        self.lat.update(lat_obs, self.r);
        self.lng.update(lng_obs, self.r);
    }

    /// One full step: predict `dt` then correct with a fix. Returns the smoothed `(lat, lng)`.
    pub fn step(&mut self, dt: f64, lat_obs: f64, lng_obs: f64) -> (f64, f64) {
        self.predict(dt);
        self.update(lat_obs, lng_obs);
        self.position()
    }

    pub fn position(&self) -> (f64, f64) {
        (self.lat.pos, self.lng.pos)
    }
    pub fn velocity(&self) -> (f64, f64) {
        (self.lat.vel, self.lng.vel)
    }
}

/// Closed-form steady-state Kalman gain for the **1-D random-walk** filter (position only,
/// process variance `q`, measurement variance `r`) — the exact `α` at which
/// `geo::ema_next(prev, z, α)` IS a Kalman update. Solves the scalar DARE `r·K² + q·K − q = 0`:
/// `K* = (−q + √(q² + 4qr)) / (2r)`.
pub fn steady_state_gain_1d(q: f64, r: f64) -> f64 {
    (-q + (q * q + 4.0 * q * r).sqrt()) / (2.0 * r)
}

#[cfg(test)]
mod tests {
    use super::*;

    // deterministic zero-mean measurement noise (no RNG): a fixed, repeating zig-zag.
    fn noise(t: usize) -> f64 {
        const SEQ: [f64; 6] = [0.6, -0.4, 0.5, -0.7, 0.3, -0.3];
        SEQ[t % SEQ.len()]
    }

    // GREEN: on a noisy constant-velocity track, the filter's position error is materially SMALLER
    // than the raw GPS error — the whole point of filtering.
    #[test]
    fn filter_reduces_noise_vs_raw() {
        // truth: pos(t) = 10 + 2·t (velocity 2), dt = 1.
        let truth = |t: usize| 10.0 + 2.0 * t as f64;
        let mut kf = AxisKalman::new(truth(0) + noise(0), 100.0);
        let (mut raw_err, mut filt_err) = (0.0f64, 0.0f64);
        for t in 1..30usize {
            let z = truth(t) + noise(t);
            kf.predict(1.0, 1e-3, 1e-3);
            kf.update(z, 4.0);
            raw_err += (z - truth(t)).abs();
            filt_err += (kf.pos - truth(t)).abs();
        }
        assert!(
            filt_err < 0.6 * raw_err,
            "filter must beat raw: filt {filt_err:.3} vs raw {raw_err:.3}"
        );
    }

    // RED→GREEN: the UPDATE step is load-bearing. Predict-only (never folding measurements) cannot
    // recover the true velocity from a bad initial guess, so it diverges; adding updates tracks.
    #[test]
    fn update_is_load_bearing() {
        let truth = |t: usize| 5.0 + 3.0 * t as f64; // true velocity 3
        // predict-only, seeded with WRONG velocity 0 → drifts away from truth.
        let mut blind = AxisKalman::new(truth(0), 100.0); // vel starts at 0
        for _ in 1..20 {
            blind.predict(1.0, 1e-3, 1e-3);
        }
        let blind_err = (blind.pos - truth(19)).abs();
        // with updates, the filter infers the velocity and tracks.
        let mut seeing = AxisKalman::new(truth(0), 100.0);
        for t in 1..20usize {
            seeing.predict(1.0, 1e-3, 1e-3);
            seeing.update(truth(t) + noise(t), 4.0);
        }
        let seeing_err = (seeing.pos - truth(19)).abs();
        assert!(blind_err > 40.0, "predict-only must drift far, got {blind_err:.2}");
        assert!(seeing_err < 3.0, "with updates must track, got {seeing_err:.2}");
        assert!(seeing.vel > 2.0, "velocity inferred ~3, got {:.2}", seeing.vel);
    }

    // GREEN: the tie to geo::ema_next — a 1-D random-walk Kalman driven to steady state has gain
    // exactly `steady_state_gain_1d(q, r)`, and one more step equals `ema_next(prev, z, K*)`.
    #[test]
    fn steady_state_gain_matches_ema() {
        let (q, r) = (0.5, 4.0);
        let k_star = steady_state_gain_1d(q, r);
        // iterate the scalar random-walk Kalman gain to its fixed point.
        let mut p = 1.0f64; // posterior variance
        let mut k = 0.0f64;
        for _ in 0..200 {
            let p_pred = p + q; // predict
            k = p_pred / (p_pred + r); // gain
            p = (1.0 - k) * p_pred; // posterior
        }
        assert!((k - k_star).abs() < 1e-9, "converged gain {k} ≠ closed-form K* {k_star}");
        // the steady-state update IS an EMA with α = K*:  x' = x + K*(z − x) = ema_next(x, z, K*).
        let (prev, z) = (7.0, 9.5);
        let kalman_step = prev + k_star * (z - prev);
        let ema_step = crate::geo::ema_next(prev, z, k_star);
        assert!((kalman_step - ema_step).abs() < 1e-12, "Kalman steady-state ≠ ema_next");
    }

    // GREEN: on a stationary courier (a stack of noisy fixes at one point), the estimate converges
    // to that point and velocity → ~0.
    #[test]
    fn converges_on_stationary_courier() {
        let mut kf = CourierKalman::new(50.0, 30.0, 100.0, 1e-4, 1e-4, 2.0);
        for t in 0..40usize {
            kf.step(1.0, 50.0 + noise(t) * 0.1, 30.0 - noise(t) * 0.1);
        }
        let (lat, lng) = kf.position();
        assert!((lat - 50.0).abs() < 0.2 && (lng - 30.0).abs() < 0.2, "settle near (50,30): {lat},{lng}");
        let (vl, vn) = kf.velocity();
        assert!(vl.abs() < 0.2 && vn.abs() < 0.2, "stationary ⇒ ~0 velocity: {vl},{vn}");
    }

    // Determinism: identical inputs ⇒ bitwise-identical state (fixed dt, no RNG).
    #[test]
    fn deterministic() {
        let run = || {
            let mut kf = CourierKalman::new(0.0, 0.0, 10.0, 1e-3, 1e-3, 4.0);
            for t in 0..15usize {
                kf.step(1.0, t as f64 + noise(t), 2.0 * t as f64 + noise(t));
            }
            kf.position()
        };
        assert_eq!(run(), run(), "Kalman must be deterministic");
    }
}
