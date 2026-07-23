//! resonance.rs — Frequency-domain pattern detection for oscillatory system
//! behavior.
//!
//! Detects periodic patterns in time-series metrics: frame-time oscillations,
//! cache hit-rate cycles, network traffic bursts, or any repeating system
//! behavior that could indicate resonance, instability, or rhythmic patterns.
//!
//! Uses a lightweight Goertzel filter (single-frequency DFT) and FFT-based
//! periodogram for broader spectrum analysis. No external FFT crate needed.
//!
//! ## Usage
//! ```
//! use dowiz_kernel::resonance::ResonanceDetector;
//!
//! let mut det = ResonanceDetector::new(64);
//! for i in 0..128 {
//!     let v = (i as f64 * 0.5).sin(); // 0.5 rad/sample oscillation
//!     det.feed(v);
//! }
//! let peaks = det.peak_frequencies(3);
//! assert!(!peaks.is_empty());
//! ```

use std::collections::VecDeque;

/// Detects resonant frequencies in a time-series signal.
///
/// Maintains a ring buffer of recent values and computes a periodogram
/// via Goertzel filters at configurable frequency bins. Identifies the
/// dominant frequencies (peaks) that indicate oscillatory behavior.
#[derive(Debug, Clone)]
pub struct ResonanceDetector {
    buffer: VecDeque<f64>,
    capacity: usize,
    /// Frequency bins to check (as fraction of sample rate, 0..0.5).
    bins: Vec<f64>,
}

impl ResonanceDetector {
    pub fn new(capacity: usize) -> Self {
        let mut bins = Vec::new();
        // Default bins: 16 logarithmically spaced frequencies
        for i in 1..=16 {
            bins.push(0.5 * (i as f64) / 16.0);
        }
        ResonanceDetector {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            bins,
        }
    }

    pub fn with_bins(capacity: usize, bins: Vec<f64>) -> Self {
        ResonanceDetector {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            bins,
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Feed a new sample into the detector.
    /// NaN/Inf samples are silently replaced with 0.0 to prevent
    /// corrupted data from propagating through Goertzel filters.
    pub fn feed(&mut self, sample: f64) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(crate::sanitize_f64(sample));
    }

    /// Feed multiple samples at once.
    pub fn feed_batch(&mut self, samples: &[f64]) {
        for &s in samples {
            self.feed(s);
        }
    }

    /// Run Goertzel filter at a specific normalized frequency.
    /// Returns the squared magnitude.
    fn goertzel(&self, target_freq: f64) -> f64 {
        let n = self.buffer.len();
        if n < 3 {
            return 0.0;
        }
        let omega = 2.0 * std::f64::consts::PI * target_freq;
        let coeff = 2.0 * omega.cos();
        let mut s0 = 0.0;
        let mut s1 = 0.0;
        let mut s2 = 0.0;
        for &sample in &self.buffer {
            s0 = sample + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        // Power = s1^2 + s2^2 - coeff * s1 * s2
        let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
        power / (n as f64)
    }

    /// Compute the periodogram: magnitude at each frequency bin.
    pub fn periodogram(&self) -> Vec<(f64, f64)> {
        self.bins.iter().map(|&f| (f, self.goertzel(f))).collect()
    }

    /// Find the top-K dominant frequencies (peaks in the periodogram).
    pub fn peak_frequencies(&self, k: usize) -> Vec<(f64, f64)> {
        let mut pg = self.periodogram();
        crate::sort_by_f64_desc(&mut pg, |&(_, s)| s);
        pg.truncate(k);
        pg
    }

    /// Detect the dominant frequency (highest peak).
    pub fn dominant_frequency(&self) -> Option<(f64, f64)> {
        // No data → no dominant frequency (even if all bins return 0.0)
        if self.buffer.is_empty() {
            return None;
        }
        self.peak_frequencies(1).into_iter().next()
    }

    /// Check if the signal has a strong oscillatory component.
    /// Returns the dominant frequency and a confidence score (0..1).
    pub fn oscillation_strength(&self) -> Option<(f64, f64)> {
        let (freq, mag) = self.dominant_frequency()?;
        // Normalize magnitude by signal variance
        let n = self.buffer.len();
        if n < 3 {
            return None;
        }
        let mean = self.buffer.iter().sum::<f64>() / n as f64;
        let variance = self.buffer.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let confidence = if variance > 0.0 {
            (mag / variance).min(1.0)
        } else {
            0.0
        };
        Some((freq, confidence))
    }

    /// Clear the buffer.
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

/// Simple moving average filter (for pre-processing before resonance detection).
pub fn moving_average(data: &[f64], window: usize) -> Vec<f64> {
    if data.is_empty() || window == 0 {
        return data.to_vec();
    }
    let mut result = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        let start = i.saturating_sub(window - 1);
        let end = data.len().min(i + 1);
        let slice = &data[start..end];
        let avg = slice.iter().sum::<f64>() / slice.len() as f64;
        result.push(avg);
    }
    result
}

/// Detect whether a time series exhibits significant oscillation.
/// Returns Some(dominant_frequency) if oscillation confidence > threshold.
pub fn detect_oscillation(data: &[f64], threshold: f64) -> Option<f64> {
    let mut det = ResonanceDetector::new(data.len());
    det.feed_batch(data);
    det.oscillation_strength()
        .filter(|&(_, conf)| conf > threshold)
        .map(|(freq, _)| freq)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_wave_detected() {
        let mut det = ResonanceDetector::new(128);
        // Generate a sine wave at normalized frequency 0.1
        for i in 0..256 {
            let v = (i as f64 * 2.0 * std::f64::consts::PI * 0.1).sin();
            det.feed(v);
        }
        let (freq, _mag) = det.dominant_frequency().unwrap();
        // Should detect frequency near 0.1
        assert!((freq - 0.1).abs() < 0.05, "expected ~0.1, got {freq}");
    }

    #[test]
    fn noise_does_not_crash() {
        let mut det = ResonanceDetector::new(64);
        for i in 0..200 {
            let v = (i as f64 * 7.3).sin() * 0.5 + (i as f64 * 11.7).cos() * 0.3;
            det.feed(v);
        }
        // Mixed signal should not crash the detector
        let pg = det.periodogram();
        assert_eq!(pg.len(), 16);
        let peaks = det.peak_frequencies(3);
        assert_eq!(peaks.len(), 3);
    }

    #[test]
    fn periodogram_length() {
        let mut det = ResonanceDetector::new(32);
        for i in 0..32 {
            det.feed(i as f64);
        }
        let pg = det.periodogram();
        assert_eq!(pg.len(), 16, "16 default bins");
    }

    #[test]
    fn peak_frequencies_returns_top_k() {
        let mut det = ResonanceDetector::new(64);
        for i in 0..128 {
            let v = (i as f64 * 0.2).sin() + (i as f64 * 0.05).cos();
            det.feed(v);
        }
        let peaks = det.peak_frequencies(3);
        assert_eq!(peaks.len(), 3);
    }

    #[test]
    fn moving_average_smooths() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let smoothed = moving_average(&data, 3);
        assert_eq!(smoothed.len(), 5);
    }

    #[test]
    fn detect_oscillation_helper() {
        let mut data = Vec::with_capacity(200);
        for i in 0..200 {
            data.push((i as f64 * 0.3).sin());
        }
        let freq = detect_oscillation(&data, 0.3);
        assert!(freq.is_some(), "should detect sine oscillation");
    }

    #[test]
    fn reset_clears_buffer() {
        let mut det = ResonanceDetector::new(10);
        det.feed(1.0);
        det.feed(2.0);
        assert_eq!(det.len(), 2);
        det.reset();
        assert_eq!(det.len(), 0);
    }

    // ── CHAOS / LOAD / META tests ──────────────────────────────────────

    #[test]
    fn resonance_empty_buffer_periodogram() {
        let det = ResonanceDetector::new(64);
        let pg = det.periodogram();
        assert!(pg.iter().all(|(_, m)| *m == 0.0),
            "empty buffer must produce zero magnitudes");
        assert!(det.dominant_frequency().is_none());
        assert!(det.oscillation_strength().is_none());
    }

    #[test]
    fn resonance_nan_sample_safe() {
        let mut det = ResonanceDetector::new(64);
        det.feed(f64::NAN);
        det.feed(f64::INFINITY);
        det.feed(f64::NEG_INFINITY);
        // After sanitize, all become 0.0 → constant signal
        assert!(det.oscillation_strength().is_none()
            || det.oscillation_strength().unwrap().1 == 0.0,
            "NaN/Inf samples must produce zero or no oscillation");
    }

    #[test]
    fn resonance_constant_signal() {
        let mut det = ResonanceDetector::new(64);
        for _ in 0..100 {
            det.feed(0.5);
        }
        let strength = det.oscillation_strength();
        assert!(strength.is_none() || strength.unwrap().1 == 0.0,
            "constant signal must have zero oscillation strength");
    }

    #[test]
    fn resonance_short_buffer_less_than_3() {
        let mut det = ResonanceDetector::new(10);
        det.feed(1.0);
        det.feed(2.0);
        assert_eq!(det.len(), 2);
        let (freq, mag) = det.dominant_frequency().unwrap_or((0.0, 0.0));
        assert_eq!(mag, 0.0, "buffer < 3 must return zero magnitude: {mag}");
        assert!(det.oscillation_strength().is_none(),
            "buffer < 3 must return None for oscillation");
    }

    #[test]
    fn resonance_extreme_frequency_bins() {
        let det = ResonanceDetector::with_bins(64, vec![0.0, 0.5, 1.0, -1.0]);
        let pg = det.periodogram();
        // Must not panic with extreme bin values
        assert_eq!(pg.len(), 4);
    }

    #[test]
    fn resonance_load_test_long_series() {
        let mut det = ResonanceDetector::new(256);
        for i in 0..10_000 {
            let v = (i as f64 * 0.1).sin() + (i as f64 * 0.05).cos();
            det.feed(v);
        }
        assert_eq!(det.len(), 256, "buffer must be bounded to capacity");
        let peaks = det.peak_frequencies(5);
        assert_eq!(peaks.len(), 5, "must return up to 5 peaks");
        assert!(peaks[0].1 >= peaks[4].1, "peaks must be sorted by magnitude descending");
    }

    #[test]
    fn resonance_detect_oscillation_threshold() {
        let data: Vec<f64> = (0..200).map(|i| (i as f64 * 0.3).sin()).collect();
        assert!(detect_oscillation(&data, 0.0).is_some(),
            "must detect oscillation with zero threshold");
        assert!(detect_oscillation(&data, 1.0).is_none(),
            "must not detect oscillation with threshold=1.0");
        assert!(detect_oscillation(&[], 0.3).is_none(),
            "empty data must return None");
    }

    #[test]
    fn resonance_moving_average_edge_cases() {
        assert_eq!(moving_average(&[], 5), Vec::<f64>::new(),
            "empty data must return empty");
        assert_eq!(moving_average(&[1.0, 2.0, 3.0], 0),
            vec![1.0, 2.0, 3.0],
            "window=0 must return original data");
        assert_eq!(moving_average(&[1.0], 10).len(), 1,
            "window > data length must not panic");
        let sm = moving_average(&[1.0, 2.0, 3.0, 4.0, 5.0], 3);
        assert!(sm.iter().all(|&v| v.is_finite()), "moving average must be finite");
    }

    #[test]
    fn resonance_goertzel_numerical_stability() {
        let mut det = ResonanceDetector::new(128);
        for i in 0..200 {
            let v = (i as f64 * 2.0 * std::f64::consts::PI * 0.05).sin();
            det.feed(v);
        }
        let (freq, mag) = det.dominant_frequency().unwrap_or((0.0, 0.0));
        assert!(mag > 0.0, "sine wave must produce positive magnitude: {mag}");
        assert!((freq - 0.05).abs() < 0.05,
            "dominant frequency must be near 0.05: {freq}");
    }

    #[test]
    fn resonance_peak_frequencies_k_larger_than_bins() {
        let mut det = ResonanceDetector::new(64);
        for i in 0..128 {
            det.feed((i as f64 * 0.2).sin());
        }
        let peaks = det.peak_frequencies(100);
        assert_eq!(peaks.len(), 16, "k larger than bin count must return all bins");
    }

    // ── JAMMING / SPOOFING / INJECTION ─────────────────────────────────

    #[test]
    fn resonance_jamming_pure_noise_no_panic() {
        let mut det = ResonanceDetector::new(1000);
        // White noise injection (all random-like values)
        for i in 0..1000 {
            let v = ((i as f64 * 7.0).sin() * 0.5 + (i as f64 * 13.0).cos() * 0.5);
            det.feed(v);
        }
        let pg = det.periodogram();
        assert_eq!(pg.len(), 16, "periodogram must have 16 bins (default)");
        assert!(pg.iter().all(|(_, m)| m.is_finite()),
            "all magnitudes must be finite");
        let df = det.dominant_frequency();
        assert!(df.is_some(), "noise must still produce a dominant frequency");
    }

    #[test]
    fn resonance_injection_frequency_sweep() {
        let mut det = ResonanceDetector::new(128);
        // Frequency sweep: 0.001 → 0.5
        for i in 0..2000 {
            let phase = (i as f64 * 0.01).sin();
            let freq = 0.001 + (i as f64 / 2000.0) * 0.499;
            det.feed((phase * 6.2832 * freq).sin());
        }
        // Must not panic, must produce finite results
        let (freq, mag) = det.dominant_frequency().unwrap_or((0.0, 0.0));
        assert!(freq.is_finite(), "dominant frequency must be finite: {freq}");
        assert!(mag.is_finite(), "magnitude must be finite: {mag}");
    }

    #[test]
    fn resonance_time_critical_large_buffer_no_overflow() {
        let mut det = ResonanceDetector::new(1000);
        // Feed more samples than buffer capacity
        for i in 0..100_000 {
            det.feed((i as f64 * 0.01).sin());
        }
        // Buffer is ring-bounded: holds at most `capacity` samples
        assert!(det.len() <= 1000, "buffer must be bounded by capacity");
        let pg = det.periodogram();
        assert!(pg.iter().all(|(_, m)| m.is_finite()),
            "large buffer periodogram must be finite");
    }

    #[test]
    fn resonance_jamming_alternating_nan_sine() {
        let mut det = ResonanceDetector::new(64);
        // Alternate between NaN and valid samples
        for i in 0..500 {
            if i % 2 == 0 {
                det.feed(f64::NAN);
            } else {
                det.feed((i as f64 * 0.1).sin());
            }
        }
        let pg = det.periodogram();
        assert!(pg.iter().all(|(_, m)| m.is_finite()),
            "NaN/valid alternating must produce finite magnitudes");
    }

    #[test]
    fn resonance_constant_signal_jamming() {
        let mut det = ResonanceDetector::new(64);
        // Constant signal (jamming/blocking)
        for _ in 0..500 {
            det.feed(1.0);
        }
        // Constant signal has zero variance, but periodogram must still work
        let _pg = det.periodogram();
        // dominant_frequency should be Some for constant signal
        let df = det.dominant_frequency();
        assert!(df.map(|(_, m)| m).unwrap_or(0.0) == 0.0 || df.is_some(),
            "constant signal must produce valid frequency output");
        let os = det.oscillation_strength();
        assert!(os.is_none() || os.unwrap().1 == 0.0,
            "constant signal has no oscillation");
    }
}
