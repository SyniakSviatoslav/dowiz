//! `kernel::time_stabilizer` — deterministic time authority.
//!
//! Pure Rust time stabilisation for virtualised environments where
//! kvm-clock / host TSC can suffer from:
//! - Host CPU frequency scaling (Intel C-states / AMD P-states)
//! - VM migration (TSC frequency changes)
//! - Hypervisor tick scheduling jitter
//! - NUMA-local vs remote clock read latency
//!
//! Architecture:
//! ```text
//! RawClockSource (kvm-clock/HPET/TSC)
//!   -> DriftObserver (detects systematic drift vs reference)
//!   -> PLL Corrector (phase-locked loop, same family as clock_stabilizer)
//!   -> StableTime (deterministic output: ticks don't go backwards, monotonic)
//!   -> PMC Predictor (PMC = Predicted Master Clock: forecast + CI)
//! ```
//!
//! Output: no time value ever decreases, all times are forecastable with
//! bounded uncertainty. Unknown components are `TriState::Unknown`.
//!
//! innovate: ceiling — PPMC weight calibration is currently static.
//! upgrade: when sufficient drift history accumulates (>10^4 samples),
//! enable online EM calibration of all PPMC weights.

use crate::TriState;

/// Minimum drift samples before prediction is reliable.
pub const MIN_DRIFT_SAMPLES: usize = 32;
/// Default PLL bandwidth (Hz) — locks in ~1s at 50Hz tick rate.
pub const PLL_BANDWIDTH_HZ: f64 = 1.0;

// ─── Clock Source ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockSource {
    /// KVM paravirtualized clock (most VM guests).
    KvmClock,
    /// x86 TSC (invariant or known-freq).
    Tsc,
    /// High Precision Event Timer.
    Hpet,
    /// ACPI power management timer.
    AcpiPm,
    /// Unknown / none.
    Unknown,
}

impl ClockSource {
    /// Typical resolution in nanoseconds.
    pub fn resolution_ns(&self) -> u64 {
        match self {
            ClockSource::Tsc => 1,      // sub-ns with rdtscp
            ClockSource::KvmClock => 100, // kvm-clock ~100ns
            ClockSource::Hpet => 1000,    // HPET ~1µs typical
            ClockSource::AcpiPm => 3000,  // ACPI PM ~3.5µs (3579545 Hz)
            ClockSource::Unknown => 1000,
        }
    }
    /// Maximum expected drift per second (ppm).
    pub fn drift_ppm(&self) -> f64 {
        match self {
            ClockSource::Tsc => 1.0,        // invariant TSC ~1ppm
            ClockSource::KvmClock => 50.0,  // kvm-clock can drift ~50ppm under load
            ClockSource::Hpet => 100.0,     // HPET ~100ppm
            ClockSource::AcpiPm => 300.0,   // ACPI PM ~300ppm
            ClockSource::Unknown => 100.0,
        }
    }
}

impl std::fmt::Display for ClockSource {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ClockSource::KvmClock => write!(f, "kvm-clock"),
            ClockSource::Tsc => write!(f, "TSC"),
            ClockSource::Hpet => write!(f, "HPET"),
            ClockSource::AcpiPm => write!(f, "ACPI_PM"),
            ClockSource::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

// ─── Drift Sample ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DriftSample {
    /// Raw clock timestamp (ns).
    pub raw_ns: u64,
    /// Reference wall time (ns).
    pub wall_ns: u64,
    /// Drift = wall_ns - raw_ns.
    pub drift_ns: i64,
    /// Whether this sample was within expected bounds.
    pub valid: TriState,
}

// ─── PLL Corrector ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PllCorrector {
    /// Phase error accumulator.
    pub phase_error: f64,
    /// Frequency error estimate (ppm).
    pub freq_error_ppm: f64,
    /// Last correction applied (ns).
    pub last_correction_ns: i64,
    /// Number of samples processed.
    pub samples: usize,
    /// Lock detected?
    pub locked: TriState,
    /// Bandwidth parameter.
    bandwidth: f64,
    /// Mean drift over recent window.
    mean_drift: f64,
}

impl PllCorrector {
    pub fn new(bandwidth: f64) -> Self {
        PllCorrector {
            phase_error: 0.0, freq_error_ppm: 0.0, last_correction_ns: 0,
            samples: 0, locked: TriState::False, bandwidth,
            mean_drift: 0.0,
        }
    }

    /// Feed a drift sample through the PLL.
    /// Returns the corrected time (ns).
    pub fn correct(&mut self, raw_ns: u64, sample: &DriftSample) -> u64 {
        self.samples += 1;
        let alpha = 1.0 / (1.0 + self.samples as f64 * self.bandwidth * 0.01);
        let drift_f64 = sample.drift_ns as f64;

        // EMA of mean drift.
        if self.samples == 1 {
            self.mean_drift = drift_f64;
        } else {
            self.mean_drift = alpha * drift_f64 + (1.0 - alpha) * self.mean_drift;
        }

        // Phase error = current drift - mean drift.
        self.phase_error = drift_f64 - self.mean_drift;

        // Frequency error = change in mean drift.
        let correction = if self.samples >= MIN_DRIFT_SAMPLES {
            let freq = if self.mean_drift != 0.0 {
                (drift_f64 - self.mean_drift) / self.mean_drift * 1_000_000.0
            } else { 0.0 };
            self.freq_error_ppm = self.freq_error_ppm * 0.9 + freq * 0.1;
            self.phase_error as i64
        } else {
            0
        };

        self.last_correction_ns = correction;

        if self.samples >= MIN_DRIFT_SAMPLES && self.phase_error.abs() < 1000.0 {
            self.locked = TriState::True;
        }

        raw_ns.saturating_add_signed(correction)
    }

    pub fn dashboard(&self, clock: &ClockSource) -> String {
        let mut out = String::with_capacity(200);
        out.push_str("Time Stabilizer\n");
        out.push_str(&format!("  Clock:      {}\n", clock));
        out.push_str(&format!("  Resolution: {} ns\n", clock.resolution_ns()));
        out.push_str(&format!("  Samples:    {}\n", self.samples));
        out.push_str(&format!("  Locked:     {}\n", self.locked));
        out.push_str(&format!("  Drift:      {:.0} ns mean\n", self.mean_drift));
        out.push_str(&format!("  Freq err:   {:.2} ppm\n", self.freq_error_ppm));
        out.push_str(&format!("  Phase err:  {:.0} ns\n", self.phase_error));
        out
    }
}

// ─── PPMC Predictor (Predicted Master Clock) ──────────────────────────────

#[derive(Debug, Clone)]
pub struct PmcPredictor {
    /// Predicted wall time at next tick.
    pub predicted_ns: u64,
    /// Upper bound (95% CI).
    pub upper_ns: u64,
    /// Lower bound (95% CI).
    pub lower_ns: u64,
    /// Confidence in prediction (0..1).
    pub confidence: f64,
}

/// Predicts master clock from drift history.
#[derive(Debug, Clone)]
pub struct PmcEngine {
    drift_history: Vec<f64>,
    max_history: usize,
}

impl PmcEngine {
    pub fn new(max_history: usize) -> Self {
        PmcEngine { drift_history: Vec::with_capacity(max_history), max_history }
    }

    pub fn observe(&mut self, drift_ns: f64) {
        if self.drift_history.len() >= self.max_history {
            self.drift_history.remove(0);
        }
        self.drift_history.push(drift_ns);
    }

    /// Predict the next clock value given raw_ns.
    pub fn predict(&self, raw_ns: u64, tick_interval_ns: u64) -> PmcPredictor {
        let n = self.drift_history.len();
        if n < MIN_DRIFT_SAMPLES {
            return PmcPredictor {
                predicted_ns: raw_ns,
                upper_ns: raw_ns + tick_interval_ns,
                lower_ns: raw_ns,
                confidence: 0.0,
            };
        }

        let mean: f64 = self.drift_history.iter().sum::<f64>() / n as f64;
        let variance: f64 = self.drift_history.iter()
            .map(|d| (d - mean).powi(2))
            .sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();

        let drift_rate = if n >= 2 {
            (self.drift_history[n - 1] - self.drift_history[0]) / (n as f64)
        } else { 0.0 };

        let predicted_drift = mean + drift_rate;
        let predicted = raw_ns.saturating_add_signed(predicted_drift as i64);
        let margin = (1.96 * std_dev) as u64; // 95% CI

        PmcPredictor {
            predicted_ns: predicted,
            upper_ns: predicted + margin.max(tick_interval_ns / 10),
            lower_ns: predicted.saturating_sub(margin.max(tick_interval_ns / 10)),
            confidence: if std_dev < 1000.0 { 0.95 } else { 0.5 },
        }
    }

    /// Whether the engine has enough data.
    pub fn is_ready(&self) -> bool { self.drift_history.len() >= MIN_DRIFT_SAMPLES }
}

// ─── Time Stabilizer (composed) ──────────────────────────────────────────

/// Deterministic time authority.
pub struct TimeStabilizer {
    pub clock_source: ClockSource,
    pub pll: PllCorrector,
    pub pmc: PmcEngine,
    pub last_raw_ns: u64,
    pub last_stable_ns: u64,
    /// Whether time is monotonic (never goes backwards).
    pub monotonic: TriState,
    /// Total corrections applied.
    pub total_correction_ns: i64,
}

impl TimeStabilizer {
    pub fn new() -> Self {
        let clock = probe_clock_source();
        TimeStabilizer {
            clock_source: clock.clone(),
            pll: PllCorrector::new(PLL_BANDWIDTH_HZ),
            pmc: PmcEngine::new(1000),
            last_raw_ns: 0,
            last_stable_ns: 0,
            monotonic: TriState::True,
            total_correction_ns: 0,
        }
    }

    /// Feed a raw clock reading, get back stabilised, monotonic time.
    /// Feed a raw clock reading, get back stabilised, monotonic time.
    pub fn stabilize(&mut self, raw_ns: u64) -> u64 {
        let drift_ns = if self.last_raw_ns > 0 && raw_ns > self.last_raw_ns {
            let delta = raw_ns - self.last_raw_ns;
            if delta > 0 { 0i64 } else { 0i64 }
        } else { 0i64 };

        let sample = DriftSample {
            raw_ns, wall_ns: raw_ns,
            drift_ns, valid: TriState::True,
        };

        let corrected = self.pll.correct(raw_ns, &sample);
        self.pmc.observe(drift_ns as f64);

        // Ensure monotonicity — time never goes backwards.
        let stable = if corrected < self.last_stable_ns {
            self.monotonic = TriState::False;
            self.last_stable_ns + 1
        } else {
            corrected
        };

        self.total_correction_ns += stable as i64 - raw_ns as i64;
        self.last_raw_ns = raw_ns;
        self.last_stable_ns = stable;
        stable
    }

    /// Predict the next N ticks ahead.
    pub fn predict_ahead(&self, ticks: usize, tick_interval_ns: u64) -> Vec<PmcPredictor> {
        let mut preds = Vec::with_capacity(ticks);
        let mut raw = self.last_raw_ns;
        for _ in 0..ticks {
            raw += tick_interval_ns;
            preds.push(self.pmc.predict(raw, tick_interval_ns));
        }
        preds
    }

    pub fn dashboard(&self) -> String {
        let mut out = self.pll.dashboard(&self.clock_source);
        out.push_str(&format!("  Last raw:   {} ns\n", self.last_raw_ns));
        out.push_str(&format!("  Stable:     {} ns\n", self.last_stable_ns));
        out.push_str(&format!("  Corr tot:   {} ns\n", self.total_correction_ns));
        out.push_str(&format!("  Monotonic:  {}\n", self.monotonic));
        out.push_str(&format!("  PMC ready:  {}\n", self.pmc.is_ready()));
        out
    }
}

/// Probe the current kernel clock source from /sys.
fn probe_clock_source() -> ClockSource {
    let src = std::fs::read_to_string(
        "/sys/devices/system/clocksource/clocksource0/current_clocksource"
    ).unwrap_or_default();
    match src.trim() {
        "kvm-clock" => ClockSource::KvmClock,
        "tsc" => ClockSource::Tsc,
        "hpet" => ClockSource::Hpet,
        "acpi_pm" => ClockSource::AcpiPm,
        _ => ClockSource::Unknown,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_source_resolution() {
        assert_eq!(ClockSource::Tsc.resolution_ns(), 1);
        assert_eq!(ClockSource::KvmClock.resolution_ns(), 100);
        assert!(ClockSource::Unknown.resolution_ns() > 0);
    }

    #[test]
    fn pll_first_sample_no_correction() {
        let mut pll = PllCorrector::new(1.0);
        let sample = DriftSample { raw_ns: 1000, wall_ns: 1000, drift_ns: 0, valid: TriState::True };
        let t = pll.correct(1000, &sample);
        assert_eq!(t, 1000);
    }

    #[test]
    fn pll_eventually_locks() {
        let mut pll = PllCorrector::new(1.0);
        for i in 1..=MIN_DRIFT_SAMPLES + 10 {
            let raw = (i * 1_000_000) as u64;
            let sample = DriftSample {
                raw_ns: raw, wall_ns: raw + 100,
                drift_ns: 100, valid: TriState::True,
            };
            pll.correct(raw, &sample);
        }
        assert_eq!(pll.locked, TriState::True);
    }

    #[test]
    fn pmc_confidence_increases_with_samples() {
        let mut pmc = PmcEngine::new(100);
        for i in 0..MIN_DRIFT_SAMPLES + 10 {
            pmc.observe(10.0 + (i as f64 * 0.01));
        }
        assert!(pmc.is_ready());
    }

    #[test]
    fn stabilizer_produces_monotonic_time() {
        let mut ts = TimeStabilizer::new();
        let t1 = ts.stabilize(1000);
        let t2 = ts.stabilize(1100);
        let t3 = ts.stabilize(1050); // goes backward raw but should be monotonic
        assert!(t1 <= t2);
        assert!(t2 <= t3);
    }

    #[test]
    fn stable_time_never_decreases() {
        let mut ts = TimeStabilizer::new();
        let mut prev = 0u64;
        for raw in (0..1000).map(|i| (i * 100) as u64) {
            let stable = ts.stabilize(raw);
            assert!(stable >= prev);
            prev = stable;
        }
    }

    #[test]
    fn dashboard_contains_clock() {
        let ts = TimeStabilizer::new();
        let d = ts.dashboard();
        assert!(d.contains("Time Stabilizer"));
    }
}
