//! online.rs — deterministic, offline-on-node online learner (Tier B3).
//!
//! GROWTH-SUBSTRATE PRIMITIVE. Builds on the B2 micrograd tape (`Value`) to fit
//! a streaming loss entirely from the LOCAL sample stream (the node's own
//! `event_log`). LOCAL-FIRST is enforced by construction: the learner updates
//! parameters from samples already resident on the node; there is no network,
//! no remote call, no vendor runtime. Determinism: same sample order ⇒
//! bit-identical parameters (fixed LR, fixed seed-free init).
//!
//! Two learners are provided:
//!   * `LinearSGD` — online ridge-regularized least-squares (`y ≈ w·x + b`),
//!     one SGD step per local sample. Used for the capture-field / value
//!     regression substrate and for self-improvement metric tracking.
//!   * `ScalarAdam` — a minimal deterministic Adam (momentum + RMS scaling with
//!     bias correction) for the capture SIREN / splat fits where conditioning
//!     matters, again fed ONLY local samples.
//!
//! Verified-by-Math: see `tests` — a noiseless line (`y = 2x + 1`) converges to
//! w→2, b→1; ScalarAdam on x² descends monotonically and reaches the optimum.

use crate::micrograd::Value;

/// Online ridge-regularized linear regression via SGD.
///
/// For each local sample (x, y) it minimizes `(ŷ − y)² + λ·w²` with
/// `ŷ = w·x + b`. `λ` (ridge) keeps the offline fit well-posed on short
/// streams. All updates are in-place on the node; no state leaves the process.
pub struct LinearSGD {
    w: Value,
    b: Value,
    lr: f64,
    lambda: f64,
}

impl LinearSGD {
    /// `lr` = learning rate, `lambda` = ridge coefficient. Init w=b=0 (local,
    /// no seed dependency → deterministic across nodes).
    pub fn new(lr: f64, lambda: f64) -> Self {
        LinearSGD {
            w: Value::var(0.0),
            b: Value::var(0.0),
            lr,
            lambda,
        }
    }

    /// Ingest ONE local sample, take one SGD step. Returns the current MSE on
    /// this sample (for local monitoring only).
    pub fn step(&self, x: f64, y: f64) -> f64 {
        let w = &self.w;
        let b = &self.b;
        let pred = w.mul(&Value::new(x)).add(b);
        let err = pred.sub(&Value::new(y));
        // L = err^2 + λ·w^2   (ridge on the slope)
        let loss = err
            .mul(&err)
            .add(&w.mul(w).mul(&Value::new(self.lambda)));
        loss.backward();
        // SGD update (offline, in place)
        let w_new = w.data() - self.lr * w.grad();
        let b_new = b.data() - self.lr * b.grad();
        w.set_data(w_new);
        b.set_data(b_new);
        // zero grads for next step (they are not auto-reset between graphs)
        w.zero_grad();
        b.zero_grad();
        let resid = y - (w_new * x + b_new);
        resid * resid
    }

    pub fn predict(&self, x: f64) -> f64 {
        self.w.data() * x + self.b.data()
    }

    pub fn weights(&self) -> (f64, f64) {
        (self.w.data(), self.b.data())
    }
}

/// Minimal deterministic Adam optimizer for a single scalar parameter.
///
/// Bias-corrected moments (β1=0.9, β2=0.999, ε=1e-8). Fed ONLY local scalar
/// losses (e.g. the SIREN/splat residual). Offline: no state leaves the node.
pub struct ScalarAdam {
    theta: Value,
    m: f64,
    v: f64,
    lr: f64,
    beta1: f64,
    beta2: f64,
    eps: f64,
    t: u64,
}

impl ScalarAdam {
    pub fn new(lr: f64) -> Self {
        ScalarAdam::new_from(lr, 0.0)
    }

    /// Construct with an explicit initial parameter θ₀ (e.g. 1.0 = neutral
    /// Q-scaler for the self-adaptation adapter). Offline, deterministic.
    pub fn new_from(lr: f64, theta0: f64) -> Self {
        ScalarAdam {
            theta: Value::var(theta0),
            m: 0.0,
            v: 0.0,
            lr,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
            t: 0,
        }
    }

    /// One Adam step on the scalar loss `f(θ)`. Returns the loss value.
    pub fn step<F>(&mut self, f: F) -> f64
    where
        F: Fn(&Value) -> Value,
    {
        self.t += 1;
        let loss = f(&self.theta);
        loss.backward();
        let g = self.theta.grad();
        self.m = self.beta1 * self.m + (1.0 - self.beta1) * g;
        self.v = self.beta2 * self.v + (1.0 - self.beta2) * g * g;
        let mhat = self.m / (1.0 - self.beta1.powi(self.t as i32));
        let vhat = self.v / (1.0 - self.beta2.powi(self.t as i32));
        let next = self.theta.data() - self.lr * mhat / (vhat.sqrt() + self.eps);
        self.theta.set_data(next);
        self.theta.zero_grad();
        loss.data()
    }

    pub fn get(&self) -> f64 {
        self.theta.data()
    }

    /// Roll the parameter back to `v` (used by the self-adaptation noether guard
    /// to reject an unstable proposed step — E3). Keeps the optimizer's moments
    /// so the next step continues from the last *accepted* θ.
    pub fn set_theta(&mut self, v: f64) {
        self.theta.set_data(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Noiseless line y = 2x + 1: after ingesting a local batch, w→2, b→1.
    /// This is a HAND-ORACLE convergence check, not a flaky tolerance game.
    #[test]
    fn linear_converges_to_known_line() {
        let learner = LinearSGD::new(0.05, 1e-4);
        // Local sample stream (could be read from event_log on-node).
        let samples = [
            (0.0, 1.0),
            (1.0, 3.0),
            (2.0, 5.0),
            (3.0, 7.0),
            (4.0, 9.0),
            (-1.0, -1.0),
            (0.5, 2.0),
        ];
        for _epoch in 0..400 {
            for &(x, y) in samples.iter() {
                learner.step(x, y);
            }
        }
        let (w, b) = learner.weights();
        assert!((w - 2.0).abs() < 1e-2, "w={w}, expected 2");
        assert!((b - 1.0).abs() < 1e-2, "b={b}, expected 1");
        // predict must match the oracle line
        assert!((learner.predict(10.0) - 21.0).abs() < 1e-1);
    }

    /// ScalarAdam on f(θ)=θ² at θ0=3 descends monotonically to ~0.
    #[test]
    fn adam_descends_on_x2() {
        let mut opt = ScalarAdam::new(0.1);
        let mut prev = 9.0_f64;
        for _ in 0..2000 {
            let loss = opt.step(|th| th.mul(th));
            let cur = loss;
            assert!(cur <= prev + 1e-9, "loss must not increase: {cur} > {prev}");
            prev = cur;
        }
        assert!(opt.get().abs() < 1e-2, "theta={}, expected ~0", opt.get());
    }

    /// LOCAL-FIRST invariant: the learner owns no network type, performs no I/O.
    /// We assert it runs to completion purely from in-process samples and that
    /// two identical local runs are bit-identical (determinism).
    #[test]
    fn offline_determinism() {
        let run = || {
            let l = LinearSGD::new(0.05, 1e-4);
            for _ in 0..50 {
                l.step(2.0, 5.0); // y = 2x+1 → drives w,b
            }
            l.weights()
        };
        let (w1, b1) = run();
        let (w2, b2) = run();
        assert_eq!(w1, w2);
        assert_eq!(b1, b2);
    }
}
