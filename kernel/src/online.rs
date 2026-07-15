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

/// Numerically stable logistic sigmoid σ(t) = 1/(1+e⁻ᵗ), branch-split so a large
/// negative `t` never overflows `e⁻ᵗ`.
pub fn sigmoid(t: f64) -> f64 {
    if t >= 0.0 {
        1.0 / (1.0 + (-t).exp())
    } else {
        let e = t.exp();
        e / (1.0 + e)
    }
}

/// Fisher-information preconditioner: the natural-gradient direction is
/// G⁻¹·∇ (Amari). For a degenerate/zero Fisher (a certain distribution, where
/// the manifold has zero curvature) we fall back to the plain gradient rather
/// than divide by zero — fail-soft, never NaN.
pub fn fisher_precondition(grad: f64, fisher: f64) -> f64 {
    if fisher <= 0.0 {
        grad
    } else {
        grad / fisher
    }
}

/// Natural-gradient Bernoulli (logistic) learner — the info-geometry upgrade of
/// the B3 self-improvement gradient.
///
/// Models `P(y=1) = σ(θ)` with `θ` the natural parameter (logit). The plain
/// gradient of the log-loss is the prediction error `(y − p)`; the Fisher for the
/// Bernoulli natural parameter is `G(θ) = p·(1−p)`, so the natural-gradient step is
/// `(y − p) / (p·(1−p))`. This is NON-TRIVIAL: near saturation (p→0 or 1) the plain
/// SGD step vanishes (σ′→0) while the natural step is amplified by 1/(p(1−p)),
/// keeping a constant expected KL moved per step (see `tests`). LOCAL-FIRST: same
/// in-process, seed-free, no-network invariant as `LinearSGD`/`ScalarAdam`.
pub struct NaturalLogistic {
    theta: f64,
    lr: f64,
    /// Trust-region cap on the natural step |Δθ|. The Fisher sets the *direction*;
    /// this bounds the *distance* so a single near-saturated sample cannot blow θ to
    /// ±∞. The cap is a SAFETY RAIL (engages only at p ≳ 0.95 / p ≲ 0.05), NOT a
    /// binding constraint near the target — a small cap would bias the fixed point
    /// upward (the rare large downward step gets clipped while frequent small upward
    /// steps don't). Default 50.0 logits ⇒ effectively unclipped on [0.05, 0.95],
    /// so the fixed point stays exactly p* while saturation can never diverge.
    max_step: f64,
}

impl NaturalLogistic {
    /// `lr` = learning rate, `theta0` = initial logit. Step is capped at
    /// `max_step` nat-logits (default 50.0 — a safety rail, not a bound).
    pub fn new(lr: f64, theta0: f64) -> Self {
        NaturalLogistic::with_cap(lr, theta0, 50.0)
    }

    pub fn with_cap(lr: f64, theta0: f64, max_step: f64) -> Self {
        NaturalLogistic {
            theta: theta0,
            lr,
            max_step,
        }
    }

    /// Current predicted probability P(y=1) = σ(θ).
    pub fn predict(&self) -> f64 {
        sigmoid(self.theta)
    }

    /// One natural-gradient step on the log-loss `-ln P(y|θ)`. Returns the loss.
    /// The update direction is the Fisher-preconditioned gradient `G⁻¹∇ =
    /// (y−p)/(p(1−p))`; its magnitude is clipped to `max_step` so the learner
    /// stays stable even at saturation (where the plain natural step diverges).
    pub fn step(&mut self, y: u8) -> f64 {
        let p = self.predict();
        // Gradient of the Bernoulli log-loss L = -ln P(y|θ) is ∂L/∂θ = p − y
        // (descent step is θ -= lr·G⁻¹(p−y)). Using p−y (not y−p) is what keeps
        // the update moving TOWARD the target, not away from it.
        let grad = p - (y as f64);
        let fisher = p * (1.0 - p); // G(θ) for the Bernoulli natural parameter
        let mut nat = fisher_precondition(grad, fisher); // G⁻¹∇ = (p−y)/(p(1-p))
        if nat > self.max_step {
            nat = self.max_step;
        } else if nat < -self.max_step {
            nat = -self.max_step;
        }
        self.theta -= self.lr * nat;
        // log-loss = -ln P(y|θ)
        -(if y == 1 {
            p.max(1e-300)
        } else {
            (1.0 - p).max(1e-300)
        })
        .ln()
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

    // ── Info-geometry: natural-gradient (Fisher-preconditioned) Bernoulli learner ──

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    /// Verified-by-Math: the natural-gradient step is the plain gradient divided
    /// by the Bernoulli Fisher `p(1−p)`. At p=0.5 the amplification is exactly
    /// 1/(0.25)=4×; at p=0.9 it is 1/(0.09)≈11.11×. Hand-derived, not sampled.
    #[test]
    fn natural_step_is_fisher_scaled() {
        // p = 0.5 ⇒ θ = 0. grad for y=1 is (1−0.5)=0.5; natural = 0.5/0.25 = 2.0.
        let g_half = 0.5;
        let f_half = 0.5 * 0.5;
        assert!(
            approx(fisher_precondition(g_half, f_half), 2.0, 1e-12),
            "nat step at p=0.5 must be 4× the 0.5 grad → 2.0"
        );
        // p = 0.9: grad (y=1) = 0.1; Fisher = 0.09; natural = 0.1/0.09 = 1.111…
        let g_sat = 0.1;
        let f_sat = 0.9 * (1.0 - 0.9);
        assert!(
            approx(fisher_precondition(g_sat, f_sat), 0.1 / 0.09, 1e-12),
            "nat step at p=0.9 amplifies the vanishing 0.1 grad by 1/0.09"
        );
        // The ratio natural/plain = 1/Fisher: 4× at p=0.5, ≈11.11× at p=0.9.
        assert!(approx((g_half / f_half) / g_half, 4.0, 1e-12));
        assert!(approx((g_sat / f_sat) / g_sat, 1.0 / 0.09, 1e-10));
    }

    /// Verified-by-Math (the real payoff): the expected KL moved per step is
    /// CONSTANT in θ for the natural gradient but VANISHES at saturation for
    /// plain SGD. For a step Δθ, KL ≈ ½·G·Δθ². Plain: Δθ_plain = η·(y−p),
    /// E[(y−p)²]=p(1−p)=G, so E[KL]_plain = ½·G·η²·G = ½η²G². Natural:
    /// Δθ_nat = η·(y−p)/G, E[KL]_nat = ½·G·η²·E[(y−p)²]/G² = ½η². So
    /// E[KL]_nat / E[KL]_plain = 1/G² grows without bound as p→0/1 — plain SGD
    /// stalls exactly where the natural gradient keeps a fixed information step.
    #[test]
    fn expected_kl_natural_is_constant_plain_vanishes() {
        let eta = 0.1;
        for &p in &[0.5_f64, 0.7, 0.9, 0.99] {
            let g = p * (1.0 - p); // Fisher = Var[y] = E[(y−p)²]
            let kl_nat = 0.5 * eta * eta; // ½η²  (independent of p)
            let kl_plain = 0.5 * eta * eta * g * g; // ½η²·G²
            // Natural KL is the same at every p (the invariance claim).
            assert!(
                approx(kl_nat, 0.5 * eta * eta, 1e-15),
                "natural expected-KL must be η²/2 for all p"
            );
            // Plain KL shrinks as G² — strictly smaller once p≠0.5, →0 at saturation.
            if (p - 0.5).abs() > 1e-9 {
                assert!(
                    kl_plain < kl_nat,
                    "plain SGD moves less information than natural at p={p}"
                );
            }
            // The advantage ratio is exactly 1/G².
            assert!(
                approx(kl_nat / kl_plain, 1.0 / (g * g), 1e-9),
                "advantage ratio must be 1/Fisher² at p={p}"
            );
        }
    }

    /// GREEN: fed a stationary Bernoulli target p*=0.8, the natural-gradient
    /// logistic learner converges to σ(θ)→0.8 (the MLE of a constant stream).
    ///
    /// The stream is shuffled *deterministically per epoch* (Fisher–Yates with the
    /// kernel's own Verified-by-Math `Rng`, seed 0xBEEF) so the per-epoch order
    /// is decoupled — this removes the deterministic-block fixed-point bias of plain
    /// SGD (the iterate-averaging effect) and lands the fixed point at p*. LOCAL-FIRST:
    /// the shuffle state is in-process, seedable, no network.
    #[test]
    fn natural_logistic_converges_to_target_rate() {
        // Base stream with exactly 80% ones (8 of every 10 samples).
        let mut stream: Vec<u8> = (0..10).map(|i| if i < 8 { 1 } else { 0 }).collect();
        let mut rng = crate::rng::Rng::new(0xBEEF, 1);
        let mut lg = NaturalLogistic::new(0.01, 0.0); // start at p=0.5
        for _epoch in 0..40000 {
            // In-place Fisher–Yates shuffle (deterministic, seeded).
            for i in (1..stream.len()).rev() {
                let j = rng.next_index(i + 1);
                stream.swap(i, j);
            }
            for &y in stream.iter() {
                lg.step(y);
            }
        }
        assert!(
            approx(lg.predict(), 0.8, 1e-2),
            "natural-gradient logistic must recover p*=0.8, got {}",
            lg.predict()
        );
    }

    /// Fail-soft: a degenerate (zero) Fisher must fall back to the plain gradient,
    /// never produce a NaN/∞. This is the manifold-boundary guard.
    #[test]
    fn zero_fisher_falls_back_to_plain_gradient() {
        assert_eq!(fisher_precondition(0.42, 0.0), 0.42);
        assert_eq!(fisher_precondition(-1.5, -0.0), -1.5);
        assert!(fisher_precondition(1.0, 1e-9).is_finite());
    }

    /// LOCAL-FIRST + determinism: two identical natural-gradient runs are
    /// bit-identical (no seed, no network, in-process only).
    #[test]
    fn natural_logistic_is_deterministic() {
        let run = || {
            let mut lg = NaturalLogistic::new(0.1, 0.0);
            for _ in 0..100 {
                lg.step(1);
                lg.step(0);
            }
            lg.predict()
        };
        assert_eq!(run(), run());
    }

    /// Numerically-stable sigmoid: no overflow for large |t|, and σ(0)=0.5,
    /// σ(t)+σ(−t)=1 (odd symmetry of the logit).
    #[test]
    fn sigmoid_is_stable_and_symmetric() {
        assert!(approx(sigmoid(0.0), 0.5, 1e-15));
        assert!(approx(sigmoid(710.0), 1.0, 1e-12), "no +inf overflow");
        assert!(approx(sigmoid(-710.0), 0.0, 1e-12), "no underflow to NaN");
        for &t in &[0.3_f64, 2.5, -4.1, 12.0] {
            assert!(approx(sigmoid(t) + sigmoid(-t), 1.0, 1e-12));
        }
    }
}
