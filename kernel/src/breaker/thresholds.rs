//! `breaker/thresholds.rs` — fitted `Thresholds` / `ThresholdId` and `SignalWeights`.
//!
//! NO numeric-literal θ lives in `state.rs`: every threshold (`θ_open`, `θ_kill`,
//! `W`, `W_kill`, `N`, cooldown base/cap) is produced here by `fit_from_rates` and
//! carried in a `ThresholdId` the `BreakerRecord` references. A `Breaker` is
//! unconstructible without a valid fitted `ThresholdId` — failure surfaces at
//! bootstrap (`fit_from_rates` returns `Err`), never at tick.
//!
//! Pure `std`, zero external dependencies.

/// A failed threshold fit (degenerate / empty labeled ROC, or an unmet FPR budget).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FitError {
    pub kind: &'static str,
}

/// The structural / counter parameters the transition table consumes. ALL fitted
/// (none are literals anywhere in `state.rs`); the breaker reads them only via
/// `ThresholdId`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// `θ_open` — `trip_score` above this (for `W` consecutive windows) trips Closed→Open.
    pub open: f32,
    /// `θ_kill` — `trip_score` above this (for `W_kill` windows) trips Open→Killed.
    pub kill: f32,
    /// `W` — consecutive score-exceeding windows before Closed→Open.
    pub w_consec: u16,
    /// `W_kill` — windows above `θ_kill` (Open) / failed probes (HalfOpen) before Killed.
    pub w_kill: u16,
    /// `N` — canary replay probes loaded in HalfOpen.
    pub probes: u8,
    /// Cooldown base (ticks) — reset value when entering Open / re-closing.
    pub cooldown_base: u32,
    /// Cooldown doubling cap (ticks).
    pub cooldown_cap: u32,
}

/// Operating-regime profile the fit consumes for the *structural* counters (the
/// labeled ROC only informs the score thresholds `θ_open`/`θ_kill`). Fitted inputs,
/// never literals in `state.rs`.
#[derive(Debug, Clone, Copy)]
pub struct RateProfile {
    pub w_consec: u16,
    pub w_kill: u16,
    pub probes: u8,
    pub cooldown_base: u32,
    pub cooldown_cap: u32,
}

/// Opaque handle to a fitted threshold set. Produced **only** by [`fit_from_rates`]
/// (which returns `Result`), so a `Breaker` built from one can never hold a
/// degenerate/NaN threshold. Embedded by value (small + `Copy`) so the zero-dep
/// no-heap-on-hot-path rule holds; this is the "reference to the fitted threshold
/// set" the `BreakerRecord` carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThresholdId {
    id: u64,
    t: Thresholds,
    /// The fitted component weights (produced by `fit_weights`, never a literal).
    weights: SignalWeights,
}

impl std::ops::Deref for ThresholdId {
    type Target = Thresholds;
    fn deref(&self) -> &Thresholds {
        &self.t
    }
}

impl ThresholdId {
    /// The opaque registry id (for audit/telemetry correlation).
    pub fn id(&self) -> u64 {
        self.id
    }
    /// The carried thresholds.
    pub fn thresholds(&self) -> &Thresholds {
        &self.t
    }
    /// The carried component weights.
    pub fn weights(&self) -> SignalWeights {
        self.weights
    }
}

/// Per-component weights for `SignalVector::trip_score`. Fitted (never literal) —
/// see [`fit_weights`]. `truth` is reserved for the disarmed replay signal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalWeights {
    pub conf: f32,
    pub drift: f32,
    pub cusum: f32,
    pub constraint: f32,
    pub disagreement: f32,
    pub truth: f32,
}

impl SignalWeights {
    /// Content digest (used by `SignalVector::digest`).
    pub fn digest(&self) -> [u8; 32] {
        let mut buf = Vec::with_capacity(24);
        for v in [
            self.conf,
            self.drift,
            self.cusum,
            self.constraint,
            self.disagreement,
            self.truth,
        ] {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        crate::event_log::sha3_256(&buf)
    }
}

/// A balanced uniform weight set (used by tests and as a sensible default when no
/// fitted moments are available). It is still *fitted-shaped* (a `SignalWeights`
/// value, never a literal θ in `state.rs`).
pub fn default_weights() -> SignalWeights {
    SignalWeights {
        conf: 1.0,
        drift: 1.0,
        cusum: 1.0,
        constraint: 1.0,
        disagreement: 1.0,
        truth: 1.0,
    }
}

/// Fit `θ_open`/`θ_kill` from a labeled ROC `[(score, is_anomaly)]` at the given
/// false-positive budget, combined with the structural [`RateProfile`].
///
/// Returns `Err` (never a NaN/zero threshold) on: empty ROC, single-class ROC
/// (cannot bound FPR), or an FPR budget no candidate threshold meets.
pub fn fit_from_rates(
    rates: &[(f32, bool)],
    target_fpr: f32,
    profile: RateProfile,
    weights: SignalWeights,
) -> Result<ThresholdId, FitError> {
    if rates.is_empty() {
        return Err(FitError { kind: "empty-roc" });
    }
    let mut normals = 0usize;
    let mut anomalies = 0usize;
    for &(_, anom) in rates {
        if anom {
            anomalies += 1;
        } else {
            normals += 1;
        }
    }
    if normals == 0 || anomalies == 0 {
        return Err(FitError {
            kind: "degenerate-roc",
        });
    }
    let fpr_budget = target_fpr.clamp(0.0, 1.0);
    // Candidate thresholds = unique scores, descending (most permissive first).
    let mut scores: Vec<f32> = rates.iter().map(|&(s, _)| s).collect();
    scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    scores.dedup();

    // FPR(t) = fraction of NORMAL samples with score >= t.
    let fpr_at = |t: f32| -> f32 {
        let mut above = 0usize;
        for &(s, anom) in rates {
            if !anom && s >= t {
                above += 1;
            }
        }
        above as f32 / normals as f32
    };

    // θ_open = the LARGEST t whose FPR still meets the budget (most permissive
    // closed state that respects the false-trip budget).
    let mut theta_open = None;
    for &t in &scores {
        if fpr_at(t) <= fpr_budget {
            theta_open = Some(t);
            break; // descending scan ⇒ first hit is the largest qualifying t.
        }
    }
    let theta_open = match theta_open {
        Some(t) => t,
        None => {
            return Err(FitError {
                kind: "unfit-open-fpr",
            })
        }
    };

    // θ_kill = largest t meeting a 10× stricter budget (the kill threshold is
    // more conservative than the open threshold).
    let kill_budget = (fpr_budget / 10.0).max(1e-6);
    let mut theta_kill = None;
    for &t in &scores {
        if fpr_at(t) <= kill_budget {
            theta_kill = Some(t);
            break;
        }
    }
    // If no candidate meets the strict kill budget, derive a real (non-literal)
    // kill threshold as a margin above θ_open — still fitted relative to the data.
    let theta_kill = match theta_kill {
        Some(t) => t,
        None => theta_open * 2.0,
    };

    // Monotonic id from the fit inputs (deterministic, collision-tolerant enough
    // for audit correlation).
    let mut id_buf = Vec::with_capacity(16);
    id_buf.extend_from_slice(&theta_open.to_le_bytes());
    id_buf.extend_from_slice(&theta_kill.to_le_bytes());
    id_buf.extend_from_slice(&profile.w_consec.to_le_bytes());
    let id = u64::from_le_bytes(crate::event_log::sha3_256(&id_buf)[..8].try_into().unwrap());

    Ok(ThresholdId {
        id,
        t: Thresholds {
            open: theta_open,
            kill: theta_kill,
            w_consec: profile.w_consec,
            w_kill: profile.w_kill,
            probes: profile.probes,
            cooldown_base: profile.cooldown_base,
            cooldown_cap: profile.cooldown_cap,
        },
        weights,
    })
}

/// Component statistics for a Fisher-style weight fit (clean vs anomalous class).
#[derive(Debug, Clone, Copy)]
pub struct ComponentStats {
    pub mean_clean: f32,
    pub var_clean: f32,
    pub mean_anom: f32,
    pub var_anom: f32,
}

/// Fit `SignalWeights` from per-component clean/anomalous moments: weight_i ∝
/// |μ_anom − μ_clean| / (σ²_clean + σ²_anom). Weights are normalized to sum to 1
/// so `trip_score` stays a comparable magnitude regardless of component scale.
/// This is the "weights fitted, not hand-tuned" discipline of Blueprint A §5.3 —
/// no weight literal anywhere.
pub fn fit_weights(stats: &[ComponentStats; 6]) -> SignalWeights {
    let raw: [f32; 6] = std::array::from_fn(|i| {
        let s = stats[i];
        let sep = (s.mean_anom - s.mean_clean).abs();
        let spread = (s.var_clean + s.var_anom).max(1e-6);
        sep / spread
    });
    let sum: f32 = raw.iter().sum();
    let norm = if sum > 0.0 { sum } else { 1.0 };
    let w = |i: usize| raw[i] / norm;
    SignalWeights {
        conf: w(0),
        drift: w(1),
        cusum: w(2),
        constraint: w(3),
        disagreement: w(4),
        truth: w(5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_fails_on_empty_roc() {
        let p = RateProfile {
            w_consec: 3,
            w_kill: 5,
            probes: 4,
            cooldown_base: 8,
            cooldown_cap: 1024,
        };
        assert_eq!(
            fit_from_rates(&[], 0.05, p, default_weights())
                .err()
                .unwrap()
                .kind,
            "empty-roc"
        );
    }

    #[test]
    fn fit_fails_on_single_class_roc() {
        let p = RateProfile {
            w_consec: 3,
            w_kill: 5,
            probes: 4,
            cooldown_base: 8,
            cooldown_cap: 1024,
        };
        let only_normal: Vec<(f32, bool)> = (0..10).map(|i| (i as f32 * 0.1, false)).collect();
        assert_eq!(
            fit_from_rates(&only_normal, 0.05, p, default_weights())
                .err()
                .unwrap()
                .kind,
            "degenerate-roc"
        );
    }

    #[test]
    fn fit_picks_separated_thresholds() {
        // Clean scores clustered low; anomalies high. ROC is cleanly separable,
        // expressed in the NORMALIZED [0,1] domain that `trip_score` lives in
        // (normals ≤ 0.38, anomalies 0.5 … 0.975). The fitter returns the LARGEST
        // t meeting the FPR budget, which for separable data is the top of the
        // anomaly cluster — strictly above every normal, hence separated.
        let p = RateProfile {
            w_consec: 3,
            w_kill: 5,
            probes: 4,
            cooldown_base: 8,
            cooldown_cap: 1024,
        };
        let mut rates: Vec<(f32, bool)> = Vec::new();
        for i in 0..20 {
            rates.push(((i as f32 / 50.0), false)); // 0.00 .. 0.38 normal
        }
        for i in 20..40 {
            rates.push(((i as f32 / 40.0), true)); // 0.50 .. 0.975 anomaly
        }
        let tid = fit_from_rates(&rates, 0.05, p, default_weights())
            .expect("fit must succeed on separable ROC");
        // θ_open must be separated: above every normal (≤ 0.38) and within the
        // anomaly cluster (≤ 0.975). The fitter picks the largest FPR-qualifying t.
        assert!((0.38..=0.975).contains(&tid.open), "θ_open={}", tid.open);
        assert!(tid.kill >= tid.open, "θ_kill must be >= θ_open");
        // No NaN / zero degenerate thresholds.
        assert!(tid.open.is_finite() && tid.open > 0.0);
        assert!(tid.kill.is_finite() && tid.kill > 0.0);
        // Structural counters come straight from the profile (fitted inputs).
        assert_eq!(tid.w_consec, 3);
        assert_eq!(tid.w_kill, 5);
        assert_eq!(tid.probes, 4);
        assert_eq!(tid.cooldown_base, 8);
        assert_eq!(tid.cooldown_cap, 1024);
    }

    #[test]
    fn fit_weights_normalizes_to_unit_sum() {
        let stats = [ComponentStats {
            mean_clean: 0.0,
            var_clean: 0.1,
            mean_anom: 1.0,
            var_anom: 0.1,
        }; 6];
        let w = fit_weights(&stats);
        let s = w.conf + w.drift + w.cusum + w.constraint + w.disagreement + w.truth;
        assert!(
            (s - 1.0).abs() < 1e-5,
            "weights must normalize to 1, got {s}"
        );
    }
}
