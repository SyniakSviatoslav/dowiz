//! `kernel::wave` — spectral propagation + interference encoding.
//!
//! States propagate as WAVES through the system, not as discrete events.
//! Each state change creates a spectral signature that propagates through
//! the xyz phase space, interferes with other waves, and decays over time.
//!
//! # Wave logic replaces event logic
//! Event: "order placed at t=5" — binary, no context.
//! Wave:  order emits phase signature at t=5, amplitude decays as 1/r,
//!        interferes with nearby waves, composite spectrum is the state.
//!
//! ZERO deps. Uses kernel's existing spectral primitives.

use crate::trig::{Phase, Xyz, PhaseVector};

/// A spectral component — one frequency with amplitude and phase.
#[derive(Debug, Clone, Copy)]
pub struct SpectralComponent {
    pub freq: f64,      // angular frequency ω (rad/tick)
    pub amplitude: f64, // initial amplitude A ∈ [0, 1]
    pub phase: f64,     // initial phase φ ∈ (-π, π]
    pub decay: f64,     // damping coefficient λ (amplitude × e^(-λ·t))
}

impl SpectralComponent {
    pub fn new(freq: f64, amplitude: f64, phase: f64, decay: f64) -> Self {
        SpectralComponent { freq, amplitude: amplitude.clamp(0.0, 1.0), phase, decay: decay.max(0.0) }
    }

    /// Value at time t: A × e^(-λ·t) × cos(ω·t + φ).
    pub fn at(&self, t: f64) -> f64 {
        self.amplitude * (-self.decay * t).exp() * (self.freq * t + self.phase).cos()
    }

    /// XYZ encoding at time t: maps to 3D phase space.
    pub fn xyz_at(&self, t: f64) -> Xyz {
        let a = self.at(t);
        let b = self.at(t + std::f64::consts::PI / 4.0); // 45° offset
        Xyz::new(a, b, (a + b) / 2.0)
    }
}

/// A wave — one or more spectral components propagating together.
#[derive(Debug, Clone)]
pub struct Wave {
    pub source: String,       // what emitted this wave
    pub timestamp_ms: u64,    // when it was emitted
    pub components: Vec<SpectralComponent>,
}

impl Wave {
    pub fn new(source: &str, timestamp_ms: u64) -> Self {
        Wave { source: source.to_string(), timestamp_ms, components: Vec::new() }
    }

    pub fn add_component(&mut self, comp: SpectralComponent) {
        self.components.push(comp);
    }

    /// Single-component convenience.
    pub fn simple(source: &str, ts: u64, freq: f64, amp: f64, decay: f64) -> Self {
        let mut w = Wave::new(source, ts);
        w.add_component(SpectralComponent::new(freq, amp, 0.0, decay));
        w
    }

    /// Composite value at time t (elapsed from emission).
    pub fn at(&self, elapsed_ms: f64) -> f64 {
        let t = elapsed_ms / 1000.0; // ms → seconds
        self.components.iter().map(|c| c.at(t)).sum::<f64>() / self.components.len().max(1) as f64
    }

    /// Is this wave effectively dead? (amplitude below threshold).
    pub fn is_decayed(&self, elapsed_ms: f64, threshold: f64) -> bool {
        self.at(elapsed_ms).abs() < threshold
    }
}

/// An interference field — superposition of multiple waves at the same point.
#[derive(Debug, Clone)]
pub struct InterferenceField {
    pub waves: Vec<Wave>,
    pub timestamp_ms: u64,
}

impl InterferenceField {
    pub fn new() -> Self {
        InterferenceField { waves: Vec::new(), timestamp_ms: crate::now_ms() }
    }

    pub fn add_wave(&mut self, wave: Wave) {
        self.waves.push(wave);
        self.timestamp_ms = crate::now_ms();
    }

    /// Composite value at current time (superposition of all waves).
    pub fn composite(&self) -> f64 {
        let now = crate::now_ms();
        if self.waves.is_empty() { return 0.0; }
        let sum: f64 = self.waves.iter()
            .map(|w| w.at((now - w.timestamp_ms) as f64))
            .sum();
        sum / self.waves.len().max(1) as f64
    }

    /// XYZ state of the interference field (3D encoding of superposition).
    pub fn xyz_state(&self) -> Xyz {
        let now = crate::now_ms();
        let mut sx = 0.0f64; let mut sy = 0.0f64; let mut sz = 0.0f64;
        for (i, w) in self.waves.iter().enumerate() {
            let t = (now - w.timestamp_ms) as f64 / 1000.0;
            let offset = i as f64 * std::f64::consts::PI / 4.0;
            sx += w.at((now - w.timestamp_ms) as f64);
            sy += w.at(((now - w.timestamp_ms) as f64) + 100.0 * offset);
            sz += w.at(((now - w.timestamp_ms) as f64) + 200.0 * offset);
            let _ = t;
        }
        let n = self.waves.len().max(1) as f64;
        Xyz::new(
            (sx / n).clamp(-1.0, 1.0),
            (sy / n).clamp(-1.0, 1.0),
            (sz / n).clamp(-1.0, 1.0),
        )
    }

    /// Clean up decayed waves.
    pub fn prune_decayed(&mut self, threshold: f64) -> usize {
        let before = self.waves.len();
        let now = crate::now_ms();
        self.waves.retain(|w| !w.is_decayed((now - w.timestamp_ms) as f64, threshold));
        before - self.waves.len()
    }

    /// Number of active waves.
    pub fn active_count(&self) -> usize { self.waves.len() }
}

/// Encode a system state as a spectral fingerprint (8-component wave).
pub fn spectral_fingerprint(state_name: &str, intensity: f64, timestamp_ms: u64) -> Wave {
    let mut wave = Wave::new(state_name, timestamp_ms);
    // 8 harmonics: fundamental + 7 overtones
    let base_freq = 1.0;
    for k in 1..=8 {
        let freq = base_freq * k as f64;
        let amp = intensity / k as f64;  // harmonic decay 1/k
        wave.add_component(SpectralComponent::new(freq, amp, 0.0, 0.05 * k as f64));
    }
    wave
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectral_component_decay() {
        let c = SpectralComponent::new(1.0, 1.0, 0.0, 0.5);
        assert!((c.at(0.0) - 1.0).abs() < 1e-10);
        assert!(c.at(10.0).abs() < 0.01); // e^(-5) ≈ 0.0067
    }

    #[test]
    fn wave_at_zero_is_amplitude() {
        let w = Wave::simple("test", crate::now_ms(), 1.0, 0.8, 0.1);
        assert!((w.at(0.0) - 0.8).abs() < 1e-10);
    }

    #[test]
    fn wave_decay_detection() {
        let w = Wave::simple("test", crate::now_ms(), 1.0, 0.5, 1.0);
        assert!(w.is_decayed(5000.0, 0.01)); // e^(-5) × 0.5 ≈ 0.003
    }

    #[test]
    fn interference_field_composite() {
        let mut field = InterferenceField::new();
        let now = crate::now_ms();
        field.add_wave(Wave::simple("a", now, 1.0, 0.5, 0.0));
        field.add_wave(Wave::simple("b", now, 2.0, 0.3, 0.0));
        let c = field.composite();
        assert!(c >= -1.0 && c <= 1.0);
    }

    #[test]
    fn spectral_fingerprint_has_8_components() {
        let w = spectral_fingerprint("order_placed", 1.0, crate::now_ms());
        assert_eq!(w.components.len(), 8);
    }

    #[test]
    fn prune_removes_decayed() {
        let mut field = InterferenceField::new();
        let now = crate::now_ms();
        // Create a wave with very old timestamp (will be decayed)
        let mut old_wave = Wave::simple("old", now - 100000, 1.0, 0.5, 0.5);
        field.add_wave(old_wave);
        let pruned = field.prune_decayed(0.01);
        assert!(pruned >= 1);
        assert_eq!(field.active_count(), 0);
    }
}
