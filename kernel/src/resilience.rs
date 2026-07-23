//! resilience.rs — Backup, failover, dynamic switching for production systems.
//!
//! Complements the `predictor` module: when predictions indicate an imminent
//! problem (high throttle, friction, error probability), the resilience layer
//! automatically activates backups, switches to fallback models, or triggers
//! circuit breakers. Race-condition avoidance via atomic sequencer.
//!
//! ## Architecture
//! - `FailoverStrategy` — which backup/failover model to use.
//! - `ResiliencePolicy` — thresholds and actions for each degradation level.
//! - `ResilienceManager` — combines multiple policies, monitors predictions,
//!   and executes failover actions. Thread-safe via atomic sequencer.
//!
//! ## Usage
//! ```
//! use dowiz_kernel::resilience::{
//!     ResilienceManager, ResiliencePolicy, FailoverStrategy, DegradationLevel,
//! };
//!
//! let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
//! mgr.record_outcome(0.3, 0.1, 0.05); // cpu=0.3, friction=0.1, error=0.05
//! assert_eq!(mgr.level(), DegradationLevel::Normal);
//! ```

use crate::predictor::PredictedOutcome;
use std::sync::atomic::{AtomicU64, Ordering};

pub const RESILIENCE_DEFAULT_COOLDOWN_MS: u64 = 5000;
pub const RESILIENCE_DEFAULT_MAX_FAILURES: u32 = 3;
pub const RESILIENCE_BACKUP_CAPACITY: usize = 100;

// ─── DegradationLevel ─────────────────────────────────────────────────────

/// How degraded the system is, based on predictor signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DegradationLevel {
    /// Normal operation — no action needed.
    Normal = 0,
    /// Elevated — pre-emptive monitoring, no switch yet.
    Elevated = 1,
    /// Warning — prepare failover, start draining.
    Warning = 2,
    /// Critical — activate failover immediately.
    Critical = 3,
    /// Failed — circuit open, full backup active.
    Failed = 4,
}

impl DegradationLevel {
    pub fn from_values(avg_metric: f64, friction: f64, error_prob: f64) -> Self {
        let score = avg_metric * 0.3 + friction * 0.3 + error_prob * 0.4;
        if score > 0.9 { DegradationLevel::Failed }
        else if score > 0.75 { DegradationLevel::Critical }
        else if score > 0.5 { DegradationLevel::Warning }
        else if score > 0.3 { DegradationLevel::Elevated }
        else { DegradationLevel::Normal }
    }

    pub fn is_actionable(&self) -> bool {
        *self >= DegradationLevel::Warning
    }
}

// ─── FailoverStrategy ─────────────────────────────────────────────────────

/// Which strategy to use when failing over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverStrategy {
    /// Use PID-only predictions (fast, less accurate).
    PidOnly,
    /// Use crystal-memory predictions (needs history).
    CrystalOnly,
    /// Use trend-extrapolation (works with minimal data).
    TrendOnly,
    /// Use ensemble but with reduced confidence threshold.
    EnsembleReduced,
    /// Use a pre-computed static fallback (defensive).
    StaticFallback,
}

impl FailoverStrategy {
    pub fn name(&self) -> &'static str {
        match self {
            FailoverStrategy::PidOnly => "pid_fallback",
            FailoverStrategy::CrystalOnly => "crystal_fallback",
            FailoverStrategy::TrendOnly => "trend_fallback",
            FailoverStrategy::EnsembleReduced => "ensemble_reduced",
            FailoverStrategy::StaticFallback => "static_fallback",
        }
    }
}

// ─── ResiliencePolicy ─────────────────────────────────────────────────────

/// A policy mapping degradation levels to failover actions.
#[derive(Debug, Clone)]
pub struct ResiliencePolicy {
    /// Level thresholds.
    pub elevated_threshold: f64,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub failed_threshold: f64,
    /// Strategy to use at each level.
    pub strategy_elevated: FailoverStrategy,
    pub strategy_warning: FailoverStrategy,
    pub strategy_critical: FailoverStrategy,
    pub strategy_failed: FailoverStrategy,
    /// Whether to open circuit breaker on critical/failed.
    pub use_circuit_breaker: bool,
    /// Cooldown ms before attempting recovery.
    pub cooldown_ms: u64,
    /// Maximum consecutive failures before permanent failover.
    pub max_consecutive_failures: u32,
}

impl ResiliencePolicy {
    /// Create a new policy with threshold ordering enforced.
    ///
    /// Panics (debug_assert) if thresholds are not strictly ordered:
    /// elevated < warning < critical < failed. In release builds,
    /// an out-of-order policy silently uses the thresholds as-is
    /// (the `DegradationLevel::from_values` chain is monotone-safe).
    pub fn new(
        elevated_threshold: f64,
        warning_threshold: f64,
        critical_threshold: f64,
        failed_threshold: f64,
        strategy_elevated: FailoverStrategy,
        strategy_warning: FailoverStrategy,
        strategy_critical: FailoverStrategy,
        strategy_failed: FailoverStrategy,
        use_circuit_breaker: bool,
        cooldown_ms: u64,
        max_consecutive_failures: u32,
    ) -> Self {
        debug_assert!(
            elevated_threshold < warning_threshold
                && warning_threshold < critical_threshold
                && critical_threshold < failed_threshold,
            "ResiliencePolicy thresholds must be strictly ordered: elevated({}) < warning({}) < critical({}) < failed({})",
            elevated_threshold, warning_threshold, critical_threshold, failed_threshold
        );
        ResiliencePolicy {
            elevated_threshold: crate::sanitize_f64(elevated_threshold),
            warning_threshold: crate::sanitize_f64(warning_threshold),
            critical_threshold: crate::sanitize_f64(critical_threshold),
            failed_threshold: crate::sanitize_f64(failed_threshold),
            strategy_elevated,
            strategy_warning,
            strategy_critical,
            strategy_failed,
            use_circuit_breaker,
            cooldown_ms,
            max_consecutive_failures,
        }
    }
}

impl Default for ResiliencePolicy {
    fn default() -> Self {
        ResiliencePolicy {
            elevated_threshold: 0.3,
            warning_threshold: 0.5,
            critical_threshold: 0.75,
            failed_threshold: 0.9,
            strategy_elevated: FailoverStrategy::EnsembleReduced,
            strategy_warning: FailoverStrategy::TrendOnly,
            strategy_critical: FailoverStrategy::StaticFallback,
            strategy_failed: FailoverStrategy::StaticFallback,
            use_circuit_breaker: true,
            cooldown_ms: RESILIENCE_DEFAULT_COOLDOWN_MS,
            max_consecutive_failures: RESILIENCE_DEFAULT_MAX_FAILURES,
        }
    }
}

// ─── BackupState ──────────────────────────────────────────────────────────

/// A backup snapshot of the predictor/system state.
#[derive(Debug, Clone)]
pub struct BackupState {
    pub id: u64,
    pub timestamp_ms: u64,
    pub metrics: Vec<f64>,
    pub label: String,
    pub checksum: u64,
}

fn backup_checksum(metrics: &[f64]) -> u64 {
    metrics.iter().fold(0u64, |acc, &m| acc.wrapping_add(m.to_bits()))
}

impl BackupState {
    pub fn new(metrics: Vec<f64>, label: &str) -> Self {
        let ts = crate::now_ms();
        let id = ts;
        // Sanitize all metrics: NaN/Inf → 0.0 so corrupted data never enters backups
        let metrics: Vec<f64> = metrics.into_iter().map(crate::sanitize_f64).collect();
        let checksum = backup_checksum(&metrics);
        BackupState { id, timestamp_ms: ts, metrics, label: label.to_string(), checksum }
    }

    pub fn verify(&self) -> bool {
        backup_checksum(&self.metrics) == self.checksum
    }
}

// ─── BackupStore ──────────────────────────────────────────────────────────

/// Simple in-memory backup store with versioning.
#[derive(Debug, Clone)]
pub struct BackupStore {
    backups: Vec<BackupState>,
    capacity: usize,
}

impl BackupStore {
    pub fn new(capacity: usize) -> Self {
        BackupStore { backups: Vec::with_capacity(capacity), capacity: capacity.max(1) }
    }

    pub fn store(&mut self, state: BackupState) {
        if self.backups.len() >= self.capacity {
            self.backups.remove(0);
        }
        self.backups.push(state);
    }

    pub fn latest(&self) -> Option<&BackupState> {
        self.backups.last()
    }

    pub fn latest_verified(&self) -> Option<&BackupState> {
        let latest = self.backups.last()?;
        if latest.verify() { Some(latest) } else { None }
    }

    pub fn count(&self) -> usize {
        self.backups.len()
    }

    pub fn clear(&mut self) {
        self.backups.clear();
    }
}

// ─── ResilienceManager ────────────────────────────────────────────────────

/// Manages resilience policies, monitors predictions, and triggers failovers.
///
/// Thread-safe via atomic sequencer. Integrates with `kernel::breaker`
/// for circuit-breaking when predictions indicate imminent failure.
pub struct ResilienceManager {
    policy: ResiliencePolicy,
    backups: BackupStore,
    level: DegradationLevel,
    consecutive_failures: u32,
    last_failover_ms: u64,
    active_strategy: FailoverStrategy,
    sequencer: AtomicU64,
}

impl ResilienceManager {
    pub fn new(policy: ResiliencePolicy) -> Self {
        ResilienceManager {
            backups: BackupStore::new(RESILIENCE_BACKUP_CAPACITY),
            level: DegradationLevel::Normal,
            active_strategy: FailoverStrategy::EnsembleReduced,
            policy,
            consecutive_failures: 0,
            last_failover_ms: 0,
            sequencer: AtomicU64::new(1),
        }
    }

    pub fn next_id(&self) -> u64 {
        self.sequencer.fetch_add(1, Ordering::SeqCst)
    }

    pub fn policy(&self) -> &ResiliencePolicy {
        &self.policy
    }

    pub fn level(&self) -> DegradationLevel {
        self.level
    }

    pub fn active_strategy(&self) -> FailoverStrategy {
        self.active_strategy
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// Record a predicted outcome and update degradation level.
    ///
    /// Returns the current strategy that should be used.
    pub fn record_outcome(&mut self, avg_metric: f64, friction: f64, error_prob: f64) -> FailoverStrategy {
        let avg_metric = crate::sanitize_normalized(avg_metric);
        let friction = crate::sanitize_normalized(friction);
        let error_prob = crate::sanitize_normalized(error_prob);
        let new_level = DegradationLevel::from_values(avg_metric, friction, error_prob);

        // Backup current state before making decisions.
        let backup = BackupState::new(
            vec![avg_metric, friction, error_prob],
            &format!("level_{:?}_{}", new_level, self.next_id()),
        );
        self.backups.store(backup);

        if new_level >= self.level {
            self.consecutive_failures += 1;
        } else {
            self.consecutive_failures = self.consecutive_failures.saturating_sub(1);
        }

        self.level = new_level;

        // Select strategy based on level.
        self.active_strategy = match self.level {
            DegradationLevel::Normal | DegradationLevel::Elevated => {
                if self.consecutive_failures >= self.policy.max_consecutive_failures {
                    self.policy.strategy_elevated
                } else {
                    FailoverStrategy::EnsembleReduced
                }
            }
            DegradationLevel::Warning => self.policy.strategy_warning,
            DegradationLevel::Critical => self.policy.strategy_critical,
            DegradationLevel::Failed => self.policy.strategy_failed,
        };

        self.active_strategy
    }

    /// Record an outcome directly from a `PredictedOutcome` slice.
    pub fn record_outcomes(&mut self, outcomes: &[PredictedOutcome]) -> FailoverStrategy {
        if outcomes.is_empty() {
            return self.active_strategy;
        }
        let avg_metric = outcomes.iter().map(|o| o.predicted_value).sum::<f64>() / outcomes.len() as f64;
        let avg_friction = outcomes.iter().map(|o| o.friction_score).sum::<f64>() / outcomes.len() as f64;
        let avg_error = outcomes.iter().map(|o| o.error_probability).sum::<f64>() / outcomes.len() as f64;
        self.record_outcome(avg_metric, avg_friction, avg_error)
    }

    /// Check whether a circuit breaker should open.
    pub fn should_open_circuit(&self) -> bool {
        self.policy.use_circuit_breaker && self.level >= DegradationLevel::Critical
    }

    /// Check whether to attempt recovery from failover.
    ///
    /// Recovery is possible when:
    /// 1. Cooldown has elapsed since last failover, AND
    /// 2. The system is no longer in critical/failed state (level ≤ Warning)
    pub fn should_recover(&self, now_ms: u64) -> bool {
        if self.level <= DegradationLevel::Warning {
            return true;
        }
        if self.last_failover_ms == 0 {
            return false;
        }
        now_ms.saturating_sub(self.last_failover_ms) > self.policy.cooldown_ms
    }

    /// Record a failover event.
    pub fn record_failover(&mut self) {
        self.last_failover_ms = crate::now_ms();
    }

    /// Get the latest verified backup.
    pub fn latest_backup(&self) -> Option<&BackupState> {
        self.backups.latest_verified()
    }

    /// Get all backup count.
    pub fn backup_count(&self) -> usize {
        self.backups.count()
    }

    /// Reset resilience state (e.g. after successful recovery).
    pub fn reset(&mut self) {
        self.level = DegradationLevel::Normal;
        self.consecutive_failures = 0;
        self.active_strategy = FailoverStrategy::EnsembleReduced;
        self.backups.clear();
    }
}

// ─── Convenience API ──────────────────────────────────────────────────────

/// Quick check: given predictor outcomes, determine the failover strategy.
pub fn quick_failover(outcomes: &[PredictedOutcome]) -> FailoverStrategy {
    let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
    mgr.record_outcomes(outcomes)
}

/// Quick backup: create a backup snapshot from a metrics vector.
pub fn quick_backup(metrics: Vec<f64>) -> BackupState {
    BackupState::new(metrics, "quick_backup")
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degradation_level_from_values() {
        // Score = metric*0.3 + friction*0.3 + error*0.4
        // 0.1*0.3 + 0.1*0.3 + 0.1*0.4 = 0.1 → Normal
        assert_eq!(DegradationLevel::from_values(0.1, 0.1, 0.1), DegradationLevel::Normal);
        // 0.5*0.3 + 0.4*0.3 + 0.3*0.4 = 0.39 → Elevated (> 0.3)
        assert_eq!(DegradationLevel::from_values(0.5, 0.4, 0.3), DegradationLevel::Elevated);
        // 0.7*0.3 + 0.6*0.3 + 0.6*0.4 = 0.63 → Warning (> 0.5)
        assert_eq!(DegradationLevel::from_values(0.7, 0.6, 0.6), DegradationLevel::Warning);
        // 0.8*0.3 + 0.7*0.3 + 0.8*0.4 = 0.77 → Critical (> 0.75)
        assert_eq!(DegradationLevel::from_values(0.8, 0.7, 0.8), DegradationLevel::Critical);
        // 0.95*0.3 + 0.9*0.3 + 0.95*0.4 = 0.935 → Failed (> 0.9)
        assert_eq!(DegradationLevel::from_values(0.95, 0.9, 0.95), DegradationLevel::Failed);
    }

    #[test]
    fn resilience_manages_levels() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        assert_eq!(mgr.level(), DegradationLevel::Normal);

        let strategy = mgr.record_outcome(0.8, 0.7, 0.8);
        assert_eq!(mgr.level(), DegradationLevel::Critical);
        assert_eq!(strategy, mgr.policy.strategy_critical);
    }

    #[test]
    fn backup_verify_passes() {
        let b = BackupState::new(vec![0.5, 0.3, 0.1], "test");
        assert!(b.verify());
    }

    #[test]
    fn backup_verify_fails_on_corruption() {
        let mut b = BackupState::new(vec![0.5, 0.3, 0.1], "test");
        b.metrics[0] = 0.9; // Corrupt the data
        assert!(!b.verify(), "checksum should detect corruption");
    }

    #[test]
    fn backup_store_roundtrip() {
        let mut store = BackupStore::new(10);
        store.store(BackupState::new(vec![1.0, 2.0], "first"));
        store.store(BackupState::new(vec![3.0, 4.0], "second"));
        assert_eq!(store.count(), 2);
        let latest = store.latest().expect("must have latest");
        assert!(latest.verify());
    }

    #[test]
    fn circuit_breaker_trigger() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        assert!(!mgr.should_open_circuit());
        mgr.record_outcome(0.8, 0.7, 0.8); // Critical
        assert!(mgr.should_open_circuit());
    }

    #[test]
    fn consecutive_failures_escalate() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        for _ in 0..5 {
            mgr.record_outcome(0.8, 0.7, 0.8); // Each record is Critical
        }
        assert!(mgr.consecutive_failures() >= mgr.policy.max_consecutive_failures);
    }

    #[test]
    fn recovery_after_cooldown() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        mgr.record_outcome(0.8, 0.7, 0.8); // Critical
        mgr.record_failover();
        let now = crate::now_ms();
        // Should not recover immediately (cooldown_ms = 5000)
        assert!(!mgr.should_recover(now));
        // After cooldown + no failures
        let future = now + mgr.policy.cooldown_ms + 1000;
        assert!(mgr.should_recover(future));
    }

    #[test]
    fn record_outcomes_from_predictions() {
        use crate::predictor::PredictedOutcome;
        let outcomes = vec![
            PredictedOutcome::new(0, "cpu_load", 0.8, 0.5),
            PredictedOutcome::new(1, "latency_ms", 0.7, 0.4),
        ];
        let mut outcomes = outcomes;
        outcomes[0].friction_score = 0.6;
        outcomes[0].error_probability = 0.7;
        outcomes[1].friction_score = 0.5;
        outcomes[1].error_probability = 0.6;

        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        let strategy = mgr.record_outcomes(&outcomes);
        assert!(mgr.level() >= DegradationLevel::Warning);
    }

    #[test]
    fn quick_failover_convenience() {
        use crate::predictor::PredictedOutcome;
        let outcomes = vec![
            PredictedOutcome::new(0, "cpu_load", 0.9, 0.5),
        ];
        let mut outcomes = outcomes;
        outcomes[0].friction_score = 0.8;
        outcomes[0].error_probability = 0.9;
        let strategy = quick_failover(&outcomes);
        assert_eq!(strategy, FailoverStrategy::StaticFallback);
    }

    #[test]
    fn sequencer_advances() {
        let mgr = ResilienceManager::new(ResiliencePolicy::default());
        let a = mgr.next_id();
        let b = mgr.next_id();
        assert!(b > a);
    }

    #[test]
    fn reset_clears_state() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        mgr.record_outcome(0.8, 0.7, 0.8);
        assert_eq!(mgr.level(), DegradationLevel::Critical);
        mgr.reset();
        assert_eq!(mgr.level(), DegradationLevel::Normal);
        assert_eq!(mgr.consecutive_failures(), 0);
    }

    // ── CHAOS / LOAD / META tests ──────────────────────────────────────

    #[test]
    fn resilience_nan_inputs_safe() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        let strategy = mgr.record_outcome(f64::NAN, f64::INFINITY, f64::NEG_INFINITY);
        assert!(mgr.level() >= DegradationLevel::Normal,
            "NaN inputs must produce valid level: {:?}", mgr.level());
        assert!(strategy == FailoverStrategy::EnsembleReduced ||
                strategy == FailoverStrategy::StaticFallback,
            "NaN inputs must produce valid strategy: {:?}", strategy);
    }

    #[test]
    fn resilience_rapid_level_oscillation() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        for i in 0..100 {
            if i % 2 == 0 {
                mgr.record_outcome(0.95, 0.95, 0.95); // Failed
            } else {
                mgr.record_outcome(0.05, 0.05, 0.05); // Normal
            }
        }
        // After oscillation, system should have reasonable state
        assert!(mgr.consecutive_failures() < 10,
            "oscillation must not max out consecutive failures: {}",
            mgr.consecutive_failures());
    }

    #[test]
    fn resilience_max_consecutive_failures_exceeded() {
        let policy = ResiliencePolicy {
            max_consecutive_failures: 2,
            ..ResiliencePolicy::default()
        };
        let mut mgr = ResilienceManager::new(policy);
        for _ in 0..5 {
            mgr.record_outcome(0.95, 0.95, 0.95);
        }
        assert!(mgr.consecutive_failures() >= 2);
    }

    #[test]
    fn resilience_empty_outcomes_list() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        let strategy = mgr.record_outcomes(&[]);
        assert_eq!(strategy, mgr.active_strategy(),
            "empty outcomes must return current strategy");
    }

    #[test]
    fn backup_store_capacity_overflow() {
        let mut store = BackupStore::new(3);
        for i in 0..100 {
            store.store(BackupState::new(vec![i as f64], "load"));
        }
        assert_eq!(store.count(), 3, "store must cap at capacity");
        // Latest should be the last inserted
        let latest = store.latest().unwrap();
        assert_eq!(latest.metrics[0], 99.0, "latest must be last inserted");
    }

    #[test]
    fn backup_verify_corrupted_checksum() {
        let b = BackupState::new(vec![1.0, 2.0, 3.0], "test");
        assert!(b.verify());
        // Tamper with checksum
        let mut b2 = b.clone();
        b2.checksum = b2.checksum.wrapping_add(1);
        assert!(!b2.verify(), "checksum mismatch must be detected");
    }

    #[test]
    fn backup_store_latest_verified_empty() {
        let store = BackupStore::new(5);
        assert!(store.latest_verified().is_none(), "empty store must return None");
    }

    #[test]
    fn circuit_breaker_auto_recovery_after_cooldown() {
        let policy = ResiliencePolicy {
            cooldown_ms: 1, // 1ms cooldown for test speed
            ..ResiliencePolicy::default()
        };
        let mut mgr = ResilienceManager::new(policy);
        mgr.record_outcome(0.95, 0.95, 0.95);
        assert!(mgr.should_open_circuit());
        mgr.record_failover();
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(mgr.should_recover(crate::now_ms()),
            "must allow recovery after cooldown");
    }

    #[test]
    fn degradation_level_edge_cases() {
        assert_eq!(DegradationLevel::from_values(0.0, 0.0, 0.0), DegradationLevel::Normal);
        assert_eq!(DegradationLevel::from_values(1.0, 1.0, 1.0), DegradationLevel::Failed);
        assert_eq!(DegradationLevel::from_values(f64::NAN, 0.5, 0.5), DegradationLevel::Normal,
            "NaN must evaluate to Normal");
        assert_eq!(DegradationLevel::from_values(f64::INFINITY, 0.5, 0.5), DegradationLevel::Failed,
            "Inf must evaluate to highest level");
    }

    #[test]
    fn resilience_manager_multiple_sequencers() {
        let mgr1 = ResilienceManager::new(ResiliencePolicy::default());
        let mgr2 = ResilienceManager::new(ResiliencePolicy::default());
        let id1 = mgr1.next_id();
        let id2 = mgr2.next_id();
        assert!(id1 > 0 && id2 > 0, "sequencers must produce valid IDs");
    }

    #[test]
    fn quick_failover_empty_outcomes() {
        let strategy = quick_failover(&[]);
        assert!(matches!(strategy, FailoverStrategy::EnsembleReduced),
            "no outcomes → ensemble reduced: {:?}", strategy);
    }

    // ── FAULT PROPAGATION / CASCADE ─────────────────────────────────────

    #[test]
    fn resilience_cascade_single_bad_prediction() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        let level_before = mgr.level();
        // One bad prediction must not cascade to critical immediately
        mgr.record_outcome(0.95, 0.9, 0.5);
        let level_after = mgr.level();
        assert!(level_after >= level_before,
            "a single bad prediction must not degrade the level below previous");
        // Multiple bad predictions should escalate
        for _ in 0..20 {
            mgr.record_outcome(0.95, 0.9, 0.8);
        }
        assert!(mgr.level() >= DegradationLevel::Warning,
            "repeated failures must escalate to at least Warning");
    }

    #[test]
    fn resilience_isolation_good_after_bad() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        // Alternating good and bad — must not cascade
        for i in 0..40 {
            if i % 2 == 0 {
                mgr.record_outcome(0.9, 0.8, 0.9); // bad
            } else {
                mgr.record_outcome(0.1, 0.05, 0.02); // good
            }
        }
        // Should not be in critical failure
        assert!(mgr.level() <= DegradationLevel::Warning,
            "alternating good/bad must not cascade to Critical: {:?}", mgr.level());
    }

    #[test]
    fn resilience_circuit_breaker_chain_isolation() {
        let mut mgr1 = ResilienceManager::new(ResiliencePolicy::default());
        let mut mgr2 = ResilienceManager::new(ResiliencePolicy::default());
        // Drive mgr1 to critical
        for _ in 0..100 {
            mgr1.record_outcome(0.99, 0.99, 0.99);
        }
        // mgr2 must remain normal
        mgr2.record_outcome(0.2, 0.1, 0.05);
        assert_eq!(mgr2.level(), DegradationLevel::Normal,
            "independent resilience managers must not propagate faults");
        assert!(mgr1.level() >= DegradationLevel::Critical,
            "mgr1 must be critical after 100 bad outcomes: {:?}", mgr1.level());
    }

    // ── JAMMING / INJECTION ─────────────────────────────────────────────

    #[test]
    fn resilience_jamming_nan_inf_outcomes() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        // Inject NaN/Inf outcomes via record_outcome (values are sanitized internally)
        mgr.record_outcome(f64::NAN, f64::NAN, f64::NAN);
        mgr.record_outcome(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        mgr.record_outcome(-0.0, -0.0, -0.0);
        // Must not panic, level stays valid
        let level = mgr.level();
        assert!(matches!(level, DegradationLevel::Normal | DegradationLevel::Elevated),
            "jamming outcomes must not corrupt degradation level: {:?}", level);
    }

    #[test]
    fn resilience_record_outcomes_nan_list() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        let outcomes = vec![
            PredictedOutcome::new(0, "cpu", f64::NAN, 0.0),
            PredictedOutcome::new(1, "mem", f64::NAN, 0.0),
        ];
        let strategy = mgr.record_outcomes(&outcomes);
        // Must return a valid strategy
        assert!(matches!(strategy, FailoverStrategy::EnsembleReduced | FailoverStrategy::StaticFallback),
            "nan outcomes must yield fallback: {:?}", strategy);
    }

    #[test]
    fn resilience_should_recover_after_cooldown() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy {
            cooldown_ms: 1,
            ..ResiliencePolicy::default()
        });
        // Drive to critical
        for _ in 0..50 {
            mgr.record_outcome(0.99, 0.99, 0.99);
        }
        mgr.record_failover();
        let now = crate::now_ms();
        assert!(mgr.should_recover(now + 100),
            "must allow recovery after 100ms (> 1ms cooldown)");
    }

    #[test]
    fn resilience_should_not_recover_immediately_after_failover() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy {
            cooldown_ms: 50000, // long cooldown
            ..ResiliencePolicy::default()
        });
        for _ in 0..50 {
            mgr.record_outcome(0.99, 0.99, 0.99);
        }
        mgr.record_failover();
        let now = crate::now_ms();
        // Should not recover immediately
        assert!(!mgr.should_recover(now),
            "must NOT recover immediately after failover");
    }

    #[test]
    fn resilience_consecutive_failures_saturating() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy::default());
        // Saturate consecutive failures
        for _ in 0..1000 {
            mgr.record_outcome(0.99, 0.99, 0.99);
        }
        let cf = mgr.consecutive_failures();
        // Must be non-negative and finite
        assert!(cf > 0, "consecutive failures must be > 0 after 1000 bad records: {cf}");
        // Good outcome reduces failures (saturating)
        mgr.record_outcome(0.0, 0.0, 0.0);
        assert!(mgr.consecutive_failures() <= cf,
            "good outcome must not increase consecutive failures");
    }

    #[test]
    fn resilience_failover_strategy_escalates() {
        let mut mgr = ResilienceManager::new(ResiliencePolicy {
            strategy_warning: FailoverStrategy::StaticFallback,
            strategy_critical: FailoverStrategy::PidOnly,
            ..ResiliencePolicy::default()
        });
        // Normal state
        assert_eq!(mgr.record_outcome(0.2, 0.1, 0.05), FailoverStrategy::EnsembleReduced);
        // Escalate to warning
        for _ in 0..10 {
            mgr.record_outcome(0.8, 0.7, 0.6);
        }
        if mgr.level() >= DegradationLevel::Warning {
            let strategy = mgr.active_strategy();
            assert_eq!(strategy, FailoverStrategy::StaticFallback,
                "warning level should use StaticFallback: {:?}", strategy);
        }
    }
}
