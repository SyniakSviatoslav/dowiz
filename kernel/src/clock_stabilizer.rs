//! clock_stabilizer.rs — PLL-inspired tick stabilizer for kernel timing.
//!
//! # What this is
//! A feedback control system that transforms irregular kernel ticks, timestamps,
//! and event intervals into stable, aligned output. Modeled after a Phase-Locked
//! Loop (PLL) -- the same control theory used in RF synthesizers, clock recovery,
//! and motor control.
//!
//! # PLL -> Kernel Mapping
//! ```text
//!   IRREGULAR INPUT          PLL STABILIZER              STABLE OUTPUT
//!   +-----------+     +------------------------+     +---------------+
//!   | raw ticks | --> | Phase Detector         | --> | aligned ticks |
//!   | timestamps|     |   (compare to ref)     |     | timestamps    |
//!   | events    |     | Loop Filter            |     | events        |
//!   | agent     |     |   (smooth jitter)      |     | latencies     |
//!   | actions   |     | VCO-equivalent         |     | predictions   |
//!   +-----------+     |   (adaptive rate)      |     +---------------+
//!                     +------------------------+
//! ```
//!
//! # Why this matters
//! - Agent actions have variable latency (LLM calls, tool execution, I/O)
//! - Without stabilization, metrics, predictions, and scheduling are unreliable
//! - Stabilized ticks -> consistent FDR timestamps -> reliable spectral analysis
//! - The PLL analogy gives us a proven feedback-control model (60+ years of theory)
//!
//! # Safeguards (mandatory cryptographic verification)
//! - NaN/infinity guards on every f64 operation (fail-closed, never propagate)
//! - Overflow protection on u64/u32 counters (saturating arithmetic)
//! - Extreme jitter rejection (intervals outside sane bounds are clamped)
//! - SHA3-256 byte-by-byte state verification (StabilizerVerifier)
//! - Deterministic: same inputs always produce same outputs and same hash

use std::fmt;

use crate::event_log::sha3_256;

/// Maximum allowed interval in microseconds (10 seconds).
/// Anything beyond this is clamped -- prevents runaway timers.
const MAX_INTERVAL_US: f64 = 10_000_000.0;

/// Minimum allowed interval in microseconds (1 microsecond).
/// Prevents division-by-zero and near-zero intervals.
const MIN_INTERVAL_US: f64 = 1.0;

/// Maximum tick count before saturation (u64::MAX / 2 to leave headroom).
const MAX_TICK_COUNT: u64 = u64::MAX / 2;

/// Maximum lock streak before saturation.
const MAX_LOCK_STREAK: u32 = u32::MAX / 2;

/// Phase-Locked Loop stabilizer for kernel timing.
///
/// Transforms irregular inter-tick intervals into stable, aligned output.
/// The loop continuously compares actual intervals against a reference rate,
/// filters the error, and adjusts the output rate to match.
///
/// All f64 operations are guarded against NaN/infinity. All counters use
/// saturating arithmetic. All outputs are SHA3-256 verifiable.
pub struct ClockStabilizer {
    /// Target tick interval in microseconds (the "reference clock").
    target_interval_us: f64,
    /// EMA smoothing factor (0.01..0.99).
    alpha: f64,
    /// Filtered phase error (EMA-smoothed).
    filtered_error: f64,
    /// Current VCO phase accumulator (microseconds).
    vco_phase: f64,
    /// Number of ticks processed (saturating).
    tick_count: u64,
    /// Running estimate of actual interval (EMA).
    estimated_interval: f64,
    /// Lock detector: consecutive ticks within tolerance.
    locked_streak: u32,
    /// Threshold: ticks within tolerance before declaring lock.
    lock_threshold: u32,
    /// Tolerance: fraction of target interval for "in-sync" (e.g., 0.1 = 10%).
    tolerance: f64,
    /// External drift correction from power/thermal forecast (ns/s).
    /// Positive = clock running fast, Negative = clock running slow.
    /// Updated externally by PowerForecastEngine + TimeStabilizer.
    external_drift_ns_per_s: f64,
    /// Confidence in external drift correction (0.0..1.0).
    drift_confidence: f64,
}

/// The result of stabilizing one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StabilizedTick {
    /// The aligned output interval (microseconds).
    pub aligned_interval_us: f64,
    /// The phase error before filtering (microseconds).
    pub raw_error_us: f64,
    /// The phase error after filtering (microseconds).
    pub filtered_error_us: f64,
    /// The lock status at this tick.
    pub lock: LockStatus,
}

/// Whether the PLL is locked (tracking the reference) or free-running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockStatus {
    /// Loop is locked -- output is stable and aligned to reference.
    Locked,
    /// Loop is acquiring -- filtering is still converging.
    Acquiring,
    /// Loop is free-running -- not enough data to determine lock.
    FreeRunning,
}

impl fmt::Display for LockStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockStatus::Locked => write!(f, "LOCKED"),
            LockStatus::Acquiring => write!(f, "ACQUIRING"),
            LockStatus::FreeRunning => write!(f, "FREE_RUNNING"),
        }
    }
}

/// Cryptographic verifier for stabilizer state integrity.
///
/// Holds a SHA3-256 digest of the stabilizer's serialized state bytes.
/// On each verification, the stabilizer serializes its state to bytes,
/// computes SHA3-256, and compares against the expected digest.
/// Byte-identical: every field in canonical order, little-endian for numerics.
pub struct StabilizerVerifier {
    /// The expected SHA3-256 digest of the stabilizer state.
    expected_hash: [u8; 32],
}

/// Errors from the stabilizer's safeguards.
#[derive(Debug, Clone, PartialEq)]
pub enum StabilizerError {
    /// Input interval was NaN or infinity.
    NonFiniteInput,
    /// Input interval was outside sane bounds (clamped to valid range).
    OutOfBounds { input: f64, clamped: f64 },
    /// Internal state became NaN (should never happen with guards -- if it does,
    /// the stabilizer resets to safe defaults).
    StateCorruption,
    /// Cryptographic verification failed -- state hash doesn't match expected.
    VerificationFailed {
        expected: [u8; 32],
        actual: [u8; 32],
    },
}

impl fmt::Display for StabilizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StabilizerError::NonFiniteInput => {
                write!(f, "stabilizer: input must be finite (not NaN or infinity)")
            }
            StabilizerError::OutOfBounds { input, clamped } => {
                write!(
                    f,
                    "stabilizer: input {:.1} out of bounds, clamped to {:.1}",
                    input, clamped
                )
            }
            StabilizerError::StateCorruption => {
                write!(f, "stabilizer: internal state corruption detected, reset to safe defaults")
            }
            StabilizerError::VerificationFailed { expected, actual } => {
                write!(
                    f,
                    "stabilizer: SHA3-256 verification failed -- expected {:02x?}, got {:02x?}",
                    expected, actual
                )
            }
        }
    }
}

/// Safely clamp an f64: reject NaN/infinity, clamp to [MIN, MAX].
fn safe_f64(value: f64, min: f64, max: f64) -> Result<f64, StabilizerError> {
    if !value.is_finite() {
        return Err(StabilizerError::NonFiniteInput);
    }
    if value < min || value > max {
        let clamped = value.clamp(min, max);
        return Err(StabilizerError::OutOfBounds {
            input: value,
            clamped,
        });
    }
    Ok(value)
}

/// Check if an f64 is finite; if not, return a safe default.
fn guard_f64(value: f64, default: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        default
    }
}

/// Saturating increment for u64.
fn sat_inc_u64(v: u64) -> u64 {
    v.saturating_add(1).min(MAX_TICK_COUNT)
}

/// Saturating increment for u32.
fn sat_inc_u32(v: u32) -> u32 {
    v.saturating_add(1).min(MAX_LOCK_STREAK)
}

/// Serialize stabilizer state to bytes (canonical, deterministic layout).
///
/// Field order: target (8 bytes LE) + alpha (8) + filtered_error (8) +
/// vco_phase (8) + tick_count (8) + estimated_interval (8) +
/// locked_streak (4) + lock_threshold (4) + tolerance (8) +
/// external_drift_ns_per_s (8) + drift_confidence (8) = 80 bytes.
fn serialize_state(s: &ClockStabilizer) -> [u8; 80] {
    let mut buf = [0u8; 80];
    buf[0..8].copy_from_slice(&s.target_interval_us.to_le_bytes());
    buf[8..16].copy_from_slice(&s.alpha.to_le_bytes());
    buf[16..24].copy_from_slice(&s.filtered_error.to_le_bytes());
    buf[24..32].copy_from_slice(&s.vco_phase.to_le_bytes());
    buf[32..40].copy_from_slice(&s.tick_count.to_le_bytes());
    buf[40..48].copy_from_slice(&s.estimated_interval.to_le_bytes());
    buf[48..52].copy_from_slice(&s.locked_streak.to_le_bytes());
    buf[52..56].copy_from_slice(&s.lock_threshold.to_le_bytes());
    buf[56..64].copy_from_slice(&s.tolerance.to_le_bytes());
    buf[64..72].copy_from_slice(&s.external_drift_ns_per_s.to_le_bytes());
    buf[72..80].copy_from_slice(&s.drift_confidence.to_le_bytes());
    buf
}

/// Compute SHA3-256 of the stabilizer's serialized state.
fn state_hash(s: &ClockStabilizer) -> [u8; 32] {
    let bytes = serialize_state(s);
    sha3_256(&bytes)
}

impl ClockStabilizer {
    /// Create a new stabilizer with a target tick interval and smoothing factor.
    pub fn new(target_interval_us: f64, alpha: f64) -> Self {
        // Guard: target must be finite and in bounds.
        let target = safe_f64(target_interval_us, MIN_INTERVAL_US, MAX_INTERVAL_US)
            .unwrap_or(1000.0);
        let alpha_clamped = alpha.clamp(0.01, 0.99);

        ClockStabilizer {
            target_interval_us: target,
            alpha: alpha_clamped,
            filtered_error: 0.0,
            vco_phase: 0.0,
            tick_count: 0,
            estimated_interval: target,
            locked_streak: 0,
            lock_threshold: 10,
            tolerance: 0.1,
            external_drift_ns_per_s: 0.0,
            drift_confidence: 0.0,
        }
    }

    /// Create a stabilizer with custom lock parameters.
    pub fn with_lock_params(
        target_interval_us: f64,
        alpha: f64,
        lock_threshold: u32,
        tolerance: f64,
    ) -> Self {
        let target = safe_f64(target_interval_us, MIN_INTERVAL_US, MAX_INTERVAL_US)
            .unwrap_or(1000.0);

        ClockStabilizer {
            target_interval_us: target,
            alpha: alpha.clamp(0.01, 0.99),
            filtered_error: 0.0,
            vco_phase: 0.0,
            tick_count: 0,
            estimated_interval: target,
            locked_streak: 0,
            lock_threshold,
            tolerance: tolerance.clamp(0.01, 0.5),
            external_drift_ns_per_s: 0.0,
            drift_confidence: 0.0,
        }
    }

    /// Feed an actual inter-tick interval and get the stabilized output.
    ///
    /// SAFEGUARDS:
    /// 1. Input validated (NaN/infinity/out-of-bounds rejected or clamped)
    /// 2. All intermediate f64 guarded against NaN propagation
    /// 3. Counters use saturating arithmetic (no overflow)
    /// 4. State hash verifiable via StabilizerVerifier
    ///
    /// INTEGRATION: the `external_drift_ns_per_s` value is subtracted from the
    /// raw error, so a positive drift (clock running fast) causes the PLL to
    /// adapt its target downward, keeping aligned with the reference. This is
    /// how PowerForecastEngine + TimeStabilizer feed into clock stabilisation.
    pub fn stabilize(&mut self, actual_interval_us: f64) -> Result<StabilizedTick, StabilizerError> {
        // SAFEGUARD 1: reject non-finite input.
        let actual = safe_f64(actual_interval_us, MIN_INTERVAL_US, MAX_INTERVAL_US)?;

        // Phase Detector: compare actual interval to reference.
        let raw_error = actual - self.target_interval_us;

        // Apply external drift correction (ns/s → µs/tick).
        let drift_correction_us = if self.drift_confidence > 0.3 {
            // Drift in ns/s, convert to µs per tick (target_interval in µs).
            (self.external_drift_ns_per_s / 1_000_000.0) * self.target_interval_us
        } else {
            0.0
        };

        // SAFEGUARD 2: guard filtered_error against NaN.
        self.filtered_error = guard_f64(
            self.alpha * (raw_error - drift_correction_us) + (1.0 - self.alpha) * self.filtered_error,
            0.0,
        );

        // SAFEGUARD 3: guard estimated_interval against NaN.
        self.estimated_interval = guard_f64(
            self.target_interval_us + self.filtered_error,
            self.target_interval_us,
        );

        // VCO phase accumulator.
        self.vco_phase = guard_f64(self.vco_phase + self.estimated_interval, 0.0);

        // Lock detector.
        let in_tolerance = raw_error.abs() <= self.target_interval_us * self.tolerance;
        if in_tolerance {
            self.locked_streak = sat_inc_u32(self.locked_streak);
        } else {
            self.locked_streak = 0;
        }

        // SAFEGUARD 4: saturating tick count.
        self.tick_count = sat_inc_u64(self.tick_count);

        // SAFEGUARD 5: final NaN check on output.
        let aligned = guard_f64(self.estimated_interval, self.target_interval_us);
        let filtered = guard_f64(self.filtered_error, 0.0);

        let lock = self.lock_status();

        Ok(StabilizedTick {
            aligned_interval_us: aligned,
            raw_error_us: raw_error,
            filtered_error_us: filtered,
            lock,
        })
    }

    /// Get the current lock status.
    pub fn lock_status(&self) -> LockStatus {
        if self.tick_count < 3 {
            LockStatus::FreeRunning
        } else if self.locked_streak >= self.lock_threshold {
            LockStatus::Locked
        } else {
            LockStatus::Acquiring
        }
    }

    /// The current stabilized interval estimate (microseconds).
    pub fn current_interval(&self) -> f64 {
        self.estimated_interval
    }

    /// The current filtered phase error (microseconds).
    pub fn filtered_error(&self) -> f64 {
        self.filtered_error
    }

    /// The target reference interval (microseconds).
    pub fn target_interval(&self) -> f64 {
        self.target_interval_us
    }

    /// Number of ticks processed.
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Reset the stabilizer to initial state.
    pub fn reset(&mut self) {
        self.filtered_error = 0.0;
        self.vco_phase = 0.0;
        self.tick_count = 0;
        self.estimated_interval = self.target_interval_us;
        self.locked_streak = 0;
        self.external_drift_ns_per_s = 0.0;
        self.drift_confidence = 0.0;
    }

    /// Set external drift correction from power/thermal forecast (ns/s).
    /// Positive = clock running fast, Negative = clock running slow.
    pub fn set_external_drift(&mut self, drift_ns_per_s: f64, confidence: f64) {
        self.external_drift_ns_per_s = if drift_ns_per_s.is_finite() { drift_ns_per_s } else { 0.0 };
        self.drift_confidence = confidence.clamp(0.0, 1.0);
    }

    /// Current external drift correction (ns/s).
    pub fn external_drift(&self) -> f64 { self.external_drift_ns_per_s }
    /// Confidence in external drift correction.
    pub fn drift_confidence(&self) -> f64 { self.drift_confidence }

    /// Get the SHA3-256 hash of the current stabilizer state.
    /// Used by StabilizerVerifier for byte-by-byte cryptographic verification.
    pub fn state_hash(&self) -> [u8; 32] {
        state_hash(self)
    }

    /// Get the raw state bytes (80 bytes, canonical serialization).
    pub fn state_bytes(&self) -> [u8; 80] {
        serialize_state(self)
    }

    /// ASCII status display for diagnostics.
    ///
    /// ```text
    /// PLL Status
    ///   Target:    1000.0 us
    ///   Estimated:  998.3 us
    ///   Error:        -1.7 us (filtered)
    ///   Ticks:         42
    ///   Lock:          LOCKED (streak: 15/10)
    /// ```
    pub fn ascii_status(&self) -> String {
        let drift_line = if self.drift_confidence > 0.0 {
            format!("  Ext Drift: {:+.1} ns/s (conf: {:.0}%)\n", self.external_drift_ns_per_s, self.drift_confidence * 100.0)
        } else {
            String::new()
        };
        format!(
            "PLL Status\n  Target:    {:.1} us\n  Estimated:  {:.1} us\n  Error:      {:+.1} us (filtered)\n  Ticks:      {}\n  Lock:       {} (streak: {}/{})\n{}",
            self.target_interval_us,
            self.estimated_interval,
            self.filtered_error,
            self.tick_count,
            self.lock_status(),
            self.locked_streak,
            self.lock_threshold,
            drift_line,
        )
    }
}

impl StabilizerVerifier {
    /// Create a verifier from a stabilizer's current state hash.
    pub fn new(stabilizer: &ClockStabilizer) -> Self {
        StabilizerVerifier {
            expected_hash: stabilizer.state_hash(),
        }
    }

    /// Create a verifier with an explicit expected hash.
    pub fn from_hash(hash: [u8; 32]) -> Self {
        StabilizerVerifier {
            expected_hash: hash,
        }
    }

    /// Verify the stabilizer's current state matches the expected hash.
    /// Returns Ok(()) if byte-identical, Err with both hashes if mismatch.
    pub fn verify(&self, stabilizer: &ClockStabilizer) -> Result<(), StabilizerError> {
        let actual = stabilizer.state_hash();
        if actual == self.expected_hash {
            Ok(())
        } else {
            Err(StabilizerError::VerificationFailed {
                expected: self.expected_hash,
                actual,
            })
        }
    }

    /// Update the verifier's expected hash to the stabilizer's current state.
    /// Call after a successful stabilization step to "advance" the verification window.
    pub fn update(&mut self, stabilizer: &ClockStabilizer) {
        self.expected_hash = stabilizer.state_hash();
    }

    /// The current expected hash.
    pub fn expected(&self) -> [u8; 32] {
        self.expected_hash
    }
}

/// Batch-stabilize a sequence of irregular intervals.
/// Returns a Vec of StabilizedTick, one per input interval.
pub fn stabilize_batch(
    stabilizer: &mut ClockStabilizer,
    intervals: &[f64],
) -> Result<Vec<StabilizedTick>, StabilizerError> {
    intervals.iter().map(|&iv| stabilizer.stabilize(iv)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stabilizer_starts_free_running() {
        let pll = ClockStabilizer::new(1000.0, 0.1);
        assert_eq!(pll.lock_status(), LockStatus::FreeRunning);
        assert_eq!(pll.tick_count(), 0);
        assert_eq!(pll.current_interval(), 1000.0);
    }

    #[test]
    fn constant_interval_converges_to_locked() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        for _ in 0..20 {
            let tick = pll.stabilize(1000.0).unwrap();
            assert_eq!(tick.raw_error_us, 0.0);
        }
        assert_eq!(pll.lock_status(), LockStatus::Locked);
        assert!((pll.current_interval() - 1000.0).abs() < 0.01);
    }

    #[test]
    fn jittery_input_gets_smoothed() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let intervals = [900.0, 1100.0, 900.0, 1100.0, 900.0, 1100.0, 900.0, 1100.0];
        let ticks = stabilize_batch(&mut pll, &intervals).unwrap();
        let last = ticks.last().unwrap();
        assert!(
            last.filtered_error_us.abs() < 50.0,
            "filtered error should be small: {}",
            last.filtered_error_us
        );
    }

    #[test]
    fn raw_error_is_actual_minus_target() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let tick = pll.stabilize(1050.0).unwrap();
        assert_eq!(tick.raw_error_us, 50.0);
    }

    #[test]
    fn negative_error_for_fast_interval() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let tick = pll.stabilize(900.0).unwrap();
        assert_eq!(tick.raw_error_us, -100.0);
    }

    #[test]
    fn tick_count_increments() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        assert_eq!(pll.tick_count(), 0);
        pll.stabilize(1000.0).unwrap();
        assert_eq!(pll.tick_count(), 1);
        pll.stabilize(1000.0).unwrap();
        assert_eq!(pll.tick_count(), 2);
    }

    #[test]
    fn free_running_until_3_ticks() {
        // Use lock_threshold=3 so locking happens within the 3-tick loop.
        let mut pll = ClockStabilizer::with_lock_params(1000.0, 0.1, 3, 0.1);
        for i in 0..3 {
            let tick = pll.stabilize(1000.0).unwrap();
            if i < 2 {
                assert_eq!(tick.lock, LockStatus::FreeRunning);
            } else {
                assert_eq!(tick.lock, LockStatus::Locked);
            }
        }
    }

    #[test]
    fn reset_returns_to_initial_state() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        for _ in 0..10 {
            pll.stabilize(1050.0).unwrap();
        }
        assert!(pll.tick_count() > 0);
        pll.reset();
        assert_eq!(pll.tick_count(), 0);
        assert_eq!(pll.current_interval(), 1000.0);
        assert_eq!(pll.filtered_error(), 0.0);
    }

    #[test]
    fn lock_threshold_is_configurable() {
        let mut pll = ClockStabilizer::with_lock_params(1000.0, 0.1, 3, 0.1);
        for _ in 0..5 {
            pll.stabilize(1000.0).unwrap();
        }
        assert_eq!(pll.lock_status(), LockStatus::Locked);
    }

    #[test]
    fn out_of_tolerance_resets_streak() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        for _ in 0..10 {
            pll.stabilize(1000.0).unwrap();
        }
        assert_eq!(pll.lock_status(), LockStatus::Locked);
        pll.stabilize(2000.0).unwrap();
        assert_ne!(pll.lock_status(), LockStatus::Locked);
    }

    #[test]
    fn ascii_status_contains_key_fields() {
        let pll = ClockStabilizer::new(1000.0, 0.1);
        let status = pll.ascii_status();
        assert!(status.contains("PLL Status"));
        assert!(status.contains("1000.0 us"));
        assert!(status.contains("FREE_RUNNING"));
    }

    #[test]
    fn target_interval_is_preserved() {
        let pll = ClockStabilizer::new(500.0, 0.2);
        assert_eq!(pll.target_interval(), 500.0);
    }

    #[test]
    fn batch_stabilize_returns_one_per_input() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let ticks = stabilize_batch(&mut pll, &[1000.0, 1010.0, 990.0]).unwrap();
        assert_eq!(ticks.len(), 3);
    }

    // --- SAFEGUARD TESTS ---

    #[test]
    fn nan_input_is_rejected() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        assert_eq!(pll.stabilize(f64::NAN), Err(StabilizerError::NonFiniteInput));
    }

    #[test]
    fn infinity_input_is_rejected() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        assert_eq!(
            pll.stabilize(f64::INFINITY),
            Err(StabilizerError::NonFiniteInput)
        );
    }

    #[test]
    fn negative_infinity_input_is_rejected() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        assert_eq!(
            pll.stabilize(f64::NEG_INFINITY),
            Err(StabilizerError::NonFiniteInput)
        );
    }

    #[test]
    fn extreme_positive_input_is_clamped() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let result = pll.stabilize(1_000_000_000.0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StabilizerError::OutOfBounds { .. }
        ));
    }

    #[test]
    fn zero_input_is_clamped() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let result = pll.stabilize(0.0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            StabilizerError::OutOfBounds { .. }
        ));
    }

    #[test]
    fn negative_input_is_clamped() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let result = pll.stabilize(-500.0);
        assert!(result.is_err());
    }

    #[test]
    fn state_hash_is_deterministic() {
        let pll1 = ClockStabilizer::new(1000.0, 0.1);
        let pll2 = ClockStabilizer::new(1000.0, 0.1);
        assert_eq!(pll1.state_hash(), pll2.state_hash());
    }

    #[test]
    fn state_hash_changes_after_stabilize() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let hash_before = pll.state_hash();
        pll.stabilize(1050.0).unwrap();
        let hash_after = pll.state_hash();
        assert_ne!(hash_before, hash_after);
    }

    #[test]
    fn state_bytes_are_correct_length() {
        let pll = ClockStabilizer::new(1000.0, 0.1);
        assert_eq!(pll.state_bytes().len(), 80);
    }

    // --- CRYPTOGRAPHIC VERIFICATION TESTS ---

    #[test]
    fn verifier_passes_on_unchanged_state() {
        let pll = ClockStabilizer::new(1000.0, 0.1);
        let verifier = StabilizerVerifier::new(&pll);
        assert!(verifier.verify(&pll).is_ok());
    }

    #[test]
    fn verifier_fails_after_state_change() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let verifier = StabilizerVerifier::new(&pll);
        pll.stabilize(1050.0).unwrap();
        assert!(verifier.verify(&pll).is_err());
    }

    #[test]
    fn verifier_update_tracks_state() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let mut verifier = StabilizerVerifier::new(&pll);
        pll.stabilize(1050.0).unwrap();
        verifier.update(&pll);
        assert!(verifier.verify(&pll).is_ok());
    }

    #[test]
    fn verifier_from_explicit_hash() {
        let pll = ClockStabilizer::new(1000.0, 0.1);
        let hash = pll.state_hash();
        let verifier = StabilizerVerifier::from_hash(hash);
        assert!(verifier.verify(&pll).is_ok());
        // Tamper: a different hash should fail.
        let wrong = [0u8; 32];
        let bad_verifier = StabilizerVerifier::from_hash(wrong);
        assert!(bad_verifier.verify(&pll).is_err());
    }

    #[test]
    fn verification_failure_contains_both_hashes() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        let hash_before = pll.state_hash();
        let verifier = StabilizerVerifier::new(&pll);
        pll.stabilize(1050.0).unwrap();
        let err = verifier.verify(&pll).unwrap_err();
        match err {
            StabilizerError::VerificationFailed { expected, actual } => {
                assert_eq!(expected, hash_before);
                assert_ne!(expected, actual);
            }
            _ => panic!("expected VerificationFailed"),
        }
    }

    #[test]
    fn state_bytes_are_canonical() {
        // Two identical stabilizers must produce byte-identical state.
        let pll1 = ClockStabilizer::new(1000.0, 0.1);
        let pll2 = ClockStabilizer::new(1000.0, 0.1);
        assert_eq!(pll1.state_bytes(), pll2.state_bytes());
    }

    // --- MULTI-MODEL STRESS TEST ---

    #[test]
    fn stress_constant_jitter_no_nan() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        // 1000 ticks of alternating jitter -- no NaN should ever appear.
        for i in 0..1000 {
            let iv = if i % 2 == 0 { 900.0 } else { 1100.0 };
            let tick = pll.stabilize(iv).unwrap();
            assert!(tick.aligned_interval_us.is_finite());
            assert!(tick.raw_error_us.is_finite());
            assert!(tick.filtered_error_us.is_finite());
        }
    }

    #[test]
    fn stress_extreme_jitter_stays_stable() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        // Wild but bounded jitter: 100..1900
        let inputs: Vec<f64> = (0..500)
            .map(|i| {
                if i % 3 == 0 {
                    100.0
                } else if i % 3 == 1 {
                    1900.0
                } else {
                    1000.0
                }
            })
            .collect();
        for iv in &inputs {
            let tick = pll.stabilize(*iv).unwrap();
            assert!(tick.aligned_interval_us.is_finite());
        }
        // Should never produce NaN even with extreme inputs.
        assert!(pll.current_interval().is_finite());
    }

    #[test]
    fn stress_counter_saturation() {
        let mut pll = ClockStabilizer::new(1000.0, 0.1);
        // Drive tick_count to near-max -- saturating arithmetic, no panic.
        // We can't actually hit u64::MAX in a test, but we verify saturation logic.
        pll.tick_count = MAX_TICK_COUNT - 1;
        pll.stabilize(1000.0).unwrap();
        assert_eq!(pll.tick_count(), MAX_TICK_COUNT);
        // Further ticks don't overflow.
        pll.stabilize(1000.0).unwrap();
        assert_eq!(pll.tick_count(), MAX_TICK_COUNT);
    }
}
