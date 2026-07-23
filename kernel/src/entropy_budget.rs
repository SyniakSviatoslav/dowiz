//! entropy_budget.rs — Foster-Lyapunov supermartingale entropy budget,
//! T-annealing (temperature schedule), and BRANCH-dispersion detector.
//!
//! MATHEMATICAL FOUNDATION
//!   * Entropy budget: tracks Shannon entropy S(t) of the drift gate's
//!     output distribution. The Foster-Lyapunov criterion requires the
//!     Lyapunov function V(t) = S(t) + λ·ρ(t) to be non-increasing
//!     (supermartingale), where ρ(t) is the spectral radius and λ > 0
//!     is a coupling constant. If dV/dt > 0 persistently, the organism
//!     is diverging.
//!   * T-annealing: temperature schedule T(k) = T₀ / (1 + k/τ) for
//!     mutation acceptance. High T early (exploration), low T later
//!     (exploitation). Accepts a mutation with probability
//!     min(1, exp(-ΔE/T)) where ΔE is the spectral cost.
//!   * BRANCH-dispersion: detects when multiple branch evaluations
//!     return identical results (zero variance) — a bad-signal detector.
//!     If all N branches agree exactly, the diversity signal is zero,
//!     indicating the evaluator is not discriminating.
//!
//! All operations are std-only, deterministic, zero-dep. Pure functions
//! where possible; stateful structs for the online accumulators.

// Shannon / Foster-Lyapunov / annealing / BRANCH — pure std, no spectral import.

/// Shannon entropy of a discrete probability distribution (natural log).
/// Input: slice of non-negative weights (need not sum to 1 — normalised
/// internally). Returns 0.0 for empty or single-element distributions.
/// Deterministic: same weights ⇒ same entropy.
pub fn shannon_entropy(weights: &[f64]) -> f64 {
    let total: f64 = weights.iter().filter(|&&w| w > 0.0).sum();
    if total <= 0.0 || weights.len() <= 1 {
        return 0.0;
    }
    -weights
        .iter()
        .filter(|&&w| w > 0.0)
        .map(|&w| {
            let p = w / total;
            p * p.ln()
        })
        .sum::<f64>()
}

/// Foster-Lyapunov supermartingale entropy budget.
///
/// Tracks V(t) = S(t) + λ·ρ(t) across commits. The budget is "healthy"
/// as long as V(t+1) ≤ V(t) + drift_margin (non-increasing in expectation).
/// Persistent budget overrun (dV > 0 for `breach_window` consecutive steps)
/// signals divergence — the organism is consuming more entropy than it's
/// contracting.
#[derive(Debug, Clone)]
pub struct EntropyBudget {
    /// Coupling constant λ > 0: trades off entropy vs spectral radius.
    lambda: f64,
    /// Current entropy S(t).
    s: f64,
    /// Current spectral radius ρ(t).
    rho: f64,
    /// Current Lyapunov value V = S + λ·ρ.
    v: f64,
    /// Number of consecutive steps where V increased beyond margin.
    breach_streak: u32,
    /// Threshold: how many consecutive overruns before alarm.
    breach_window: u32,
    /// Drift margin: maximum allowed single-step increase in V (to absorb noise).
    margin: f64,
    /// Total commits observed.
    commits: u64,
}

impl EntropyBudget {
    /// `lambda` = coupling constant (start with 1.0), `margin` = max allowed
    /// single-step ΔV (start with 0.01 for tight control), `breach_window` =
    /// consecutive overruns before alarm (start with 5).
    pub fn new(lambda: f64, margin: f64, breach_window: u32) -> Self {
        EntropyBudget {
            lambda: crate::sanitize_f64(lambda),
            s: 0.0,
            rho: 0.0,
            v: 0.0,
            breach_streak: 0,
            breach_window,
            margin: crate::sanitize_f64(margin),
            commits: 0,
        }
    }

    /// Record one commit: update the entropy S from the drift gate's output
    /// distribution and ρ from the spectral radius. Returns the new V value.
    /// `drift_weights` is the distribution over drift classes
    /// (e.g. [count_damped, count_resonant, count_unstable] as proportions).
    pub fn step(&mut self, drift_weights: &[f64], rho: f64) -> f64 {
        let s = shannon_entropy(drift_weights);
        self.s = s;
        self.rho = rho;
        let v = s + self.lambda * rho;
        let delta = v - self.v;
        self.v = v;
        self.commits += 1;
        if delta > self.margin {
            self.breach_streak += 1;
        } else {
            self.breach_streak = 0;
        }
        v
    }

    /// Is the budget in breach? (V increasing persistently.)
    pub fn is_breached(&self) -> bool {
        self.breach_streak >= self.breach_window
    }

    /// Current Lyapunov value V = S + λ·ρ.
    pub fn lyapunov(&self) -> f64 {
        self.v
    }

    /// Current entropy S(t).
    pub fn entropy(&self) -> f64 {
        self.s
    }

    /// Current spectral radius ρ(t).
    pub fn spectral_radius(&self) -> f64 {
        self.rho
    }

    /// Breach streak count.
    pub fn breach_streak(&self) -> u32 {
        self.breach_streak
    }

    /// Total commits observed.
    pub fn commits(&self) -> u64 {
        self.commits
    }
}

/// T-annealing: temperature schedule for mutation acceptance.
///
/// T(k) = T₀ / (1 + k/τ) where k is the commit count and τ is the
/// annealing time constant. At high T, more mutations are accepted
/// (exploration); at low T, only improvements are accepted (exploitation).
/// Accepts a mutation with probability min(1, exp(-ΔE/T)) where ΔE is
/// the spectral cost (ρ_after - ρ_before, clamped ≥ 0).
#[derive(Debug, Clone)]
pub struct TAnnealing {
    /// Initial temperature T₀.
    t0: f64,
    /// Annealing time constant τ (in commits).
    tau: f64,
    /// Current commit count k.
    k: u64,
}

impl TAnnealing {
    /// `t0` = initial temperature (start with 1.0 for full exploration),
    /// `tau` = annealing time constant in commits (start with 100).
    pub fn new(t0: f64, tau: f64) -> Self {
        TAnnealing { t0, tau, k: 0 }
    }

    /// Current temperature T(k).
    pub fn temperature(&self) -> f64 {
        self.t0 / (1.0 + (self.k as f64) / self.tau)
    }

    /// Should we accept a mutation with spectral cost ΔE ≥ 0?
    /// Deterministic: uses a fixed-point comparison (not RNG) — accepts iff
    /// ΔE < T(k) * ln(2) (the 50% threshold). This keeps the decision
    /// reproducible across runs while still annealing.
    pub fn accept(&mut self, delta_e: f64) -> bool {
        self.k += 1;
        let t = self.t0 / (1.0 + ((self.k - 1) as f64) / self.tau);
        // Accept if cost is negative (improvement) or below the annealing threshold.
        delta_e <= 0.0 || delta_e < t * 2.0_f64.ln()
    }

    /// Peek at the acceptance threshold without advancing k.
    pub fn threshold(&self) -> f64 {
        self.temperature() * 2.0_f64.ln()
    }

    /// Current commit count.
    pub fn commits(&self) -> u64 {
        self.k
    }
}

/// BRANCH-dispersion detector: flags when multiple branch evaluations
/// return identical results (zero variance).
///
/// In the hydra's self-evaluation loop, multiple branches (e.g. different
/// LLM models, different evaluation strategies) should produce DIVERSE
/// signals. If all branches agree exactly, the evaluator is not
/// discriminating — the signal is "dead". This detector computes the
/// variance across branch outputs and flags zero-variance episodes.
#[derive(Debug, Clone)]
pub struct BranchDispersion {
    /// Ring buffer of per-round branch means.
    means: Vec<f64>,
    /// Window size (number of evaluation rounds to look back).
    window: usize,
    /// Write pointer into the ring buffer.
    ptr: usize,
    /// Count of filled entries.
    filled: usize,
}

impl BranchDispersion {
    /// `window` = how many recent rounds to track (start with 10).
    pub fn new(window: usize) -> Self {
        BranchDispersion {
            means: vec![0.0; window],
            window,
            ptr: 0,
            filled: 0,
        }
    }

    /// Record one evaluation round: `branch_values` are the outputs from
    /// each branch (e.g. [score_model_a, score_model_b, score_model_c]).
    /// Returns the dispersion (variance) of this round's branch values.
    pub fn record(&mut self, branch_values: &[f64]) -> f64 {
        let n = branch_values.len();
        if n == 0 {
            return 0.0;
        }
        let mean = branch_values.iter().sum::<f64>() / (n as f64);
        let variance = branch_values.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n as f64);
        self.means[self.ptr] = variance;
        self.ptr = (self.ptr + 1) % self.window;
        if self.filled < self.window {
            self.filled += 1;
        }
        variance
    }

    /// Is the dispersion zero on the LAST recorded round?
    pub fn is_zero_dispersion(&self) -> bool {
        if self.filled == 0 {
            return false;
        }
        let last = (self.ptr + self.window - 1) % self.window;
        self.means[last] < 1e-15
    }

    /// Rolling mean of variances over the window.
    pub fn rolling_variance(&self) -> f64 {
        if self.filled == 0 {
            return 0.0;
        }
        self.means[..self.filled].iter().sum::<f64>() / (self.filled as f64)
    }

    /// Filled count (how many rounds recorded so far).
    pub fn filled(&self) -> usize {
        self.filled
    }
}

/// Simplified BRANCH-dispersion check: returns true if all branch values
/// are exactly equal (zero variance). This is the core detector without
/// the rolling window.
pub fn branches_agree_exactly(branch_values: &[f64]) -> bool {
    if branch_values.len() <= 1 {
        return true;
    }
    let first = branch_values[0];
    branch_values.iter().all(|&x| (x - first).abs() < 1e-15)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Shannon entropy ──

    #[test]
    fn entropy_uniform_is_log_n() {
        // Uniform distribution over 4 elements: S = ln(4)
        let w = [0.25, 0.25, 0.25, 0.25];
        let s = shannon_entropy(&w);
        assert!((s - 4.0_f64.ln()).abs() < 1e-12, "S(uniform-4) = ln(4)");
    }

    #[test]
    fn entropy_single_element_is_zero() {
        assert!((shannon_entropy(&[1.0]) - 0.0).abs() < 1e-15);
    }

    #[test]
    fn entropy_empty_is_zero() {
        assert!((shannon_entropy(&[]) - 0.0).abs() < 1e-15);
    }

    #[test]
    fn entropy_peaked_is_near_zero() {
        // Highly concentrated: S ≈ 0.
        let w = [0.999999, 0.0000005, 0.0000005];
        let s = shannon_entropy(&w);
        assert!(s < 0.01, "peaked distribution → S ≈ 0, got {s}");
    }

    #[test]
    fn entropy_normalised_internally() {
        // Weights don't need to sum to 1 — normalised internally.
        let w = [2.0, 2.0, 2.0, 2.0];
        let s = shannon_entropy(&w);
        assert!((s - 4.0_f64.ln()).abs() < 1e-12, "normalised ⇒ same as uniform");
    }

    // ── Foster-Lyapunov entropy budget ──

    #[test]
    fn budget_v_increases_with_rho() {
        let mut budget = EntropyBudget::new(1.0, 100.0, 5); // large margin
        let weights = [0.5, 0.5];
        budget.step(&weights, 0.5);
        let v1 = budget.lyapunov();
        budget.step(&weights, 0.8);
        let v2 = budget.lyapunov();
        // V = S + λ·ρ, same S, ρ increased ⇒ V increased.
        assert!(v2 > v1, "V should increase with ρ");
    }

    #[test]
    fn budget_breach_detection() {
        let mut budget = EntropyBudget::new(1.0, 0.001, 3); // tight margin, 3-step window
        let weights = [0.5, 0.5];
        // Push ρ up steadily → V increases beyond margin → breach.
        for i in 0..10 {
            budget.step(&weights, 0.1 * (i as f64));
        }
        assert!(budget.is_breached(), "persistent V increase → breach");
    }

    #[test]
    fn budget_no_breach_on_stable() {
        let mut budget = EntropyBudget::new(1.0, 0.01, 5);
        let weights = [0.5, 0.5];
        // Constant ρ → V stable → no breach.
        for _ in 0..20 {
            budget.step(&weights, 0.5);
        }
        assert!(!budget.is_breached(), "stable V → no breach");
    }

    // ── T-annealing ──

    #[test]
    fn annealing_temperature_decreases() {
        let mut ta = TAnnealing::new(1.0, 100.0);
        let t0 = ta.temperature();
        for _ in 0..50 {
            ta.accept(0.1);
        }
        let t50 = ta.temperature();
        assert!(t50 < t0, "temperature must decrease with commits");
    }

    #[test]
    fn annealing_accepts_improvement() {
        let mut ta = TAnnealing::new(1.0, 100.0);
        // Negative ΔE (improvement) always accepted.
        assert!(ta.accept(-0.5));
    }

    #[test]
    fn annealing_rejects_large_cost_at_low_t() {
        let mut ta = TAnnealing::new(0.01, 10.0); // very low T₀, fast anneal
        for _ in 0..100 {
            ta.accept(0.0);
        }
        // After 100 commits at T₀=0.01, τ=10: T ≈ 0.01/11 ≈ 0.0009.
        // Threshold ≈ 0.0006. Large cost should be rejected.
        assert!(!ta.accept(1.0), "large cost at low T → reject");
    }

    // ── BRANCH-dispersion ──

    #[test]
    fn branches_agree_exactly_true_when_equal() {
        assert!(branches_agree_exactly(&[0.5, 0.5, 0.5]));
        assert!(branches_agree_exactly(&[1.0]));
        assert!(branches_agree_exactly(&[]));
    }

    #[test]
    fn branches_agree_exactly_false_when_different() {
        assert!(!branches_agree_exactly(&[0.5, 0.6, 0.5]));
        assert!(!branches_agree_exactly(&[0.0, 1.0]));
    }

    #[test]
    fn branch_dispersion_records_variance() {
        let mut bd = BranchDispersion::new(5);
        let v1 = bd.record(&[0.5, 0.5, 0.5]); // zero variance
        assert!(v1 < 1e-15, "identical values → zero variance");
        assert!(bd.is_zero_dispersion(), "identical branches → zero dispersion");
        let v2 = bd.record(&[0.0, 0.5, 1.0]); // nonzero variance
        assert!(v2 > 0.0, "different values → nonzero variance");
        assert!(!bd.is_zero_dispersion(), "different branches → nonzero dispersion");
        assert_eq!(bd.filled(), 2);
    }
}
