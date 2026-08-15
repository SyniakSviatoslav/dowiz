//! wave.rs — std host shim (pure `SpectralComponent`/`Wave`/`spectral_fingerprint`
//! live in `dowiz_core::wave`; `InterferenceField` — whose methods stamp/read the
//! wall clock via `crate::now_ms` — stays here).

pub use dowiz_core::wave::*;

use alloc::vec::Vec;

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
    pub fn xyz_state(&self) -> crate::trig::Xyz {
        let now = crate::now_ms();
        let mut sx = 0.0f64; let mut sy = 0.0f64; let mut sz = 0.0f64;
        for (i, w) in self.waves.iter().enumerate() {
            let t = (now - w.timestamp_ms) as f64 / 1000.0;
            let offset = i as f64 * core::f64::consts::PI / 4.0;
            sx += w.at((now - w.timestamp_ms) as f64);
            sy += w.at(((now - w.timestamp_ms) as f64) + 100.0 * offset);
            sz += w.at(((now - w.timestamp_ms) as f64) + 200.0 * offset);
            let _ = t;
        }
        let n = self.waves.len().max(1) as f64;
        crate::trig::Xyz::new(
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn prune_removes_decayed() {
        let mut field = InterferenceField::new();
        let now = crate::now_ms();
        // Create a wave with very old timestamp (will be decayed)
        let old_wave = Wave::simple("old", now - 100000, 1.0, 0.5, 0.5);
        field.add_wave(old_wave);
        let pruned = field.prune_decayed(0.01);
        assert!(pruned >= 1);
        assert_eq!(field.active_count(), 0);
    }
}
