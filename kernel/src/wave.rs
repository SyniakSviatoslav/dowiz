//! wave.rs — std host shim. The pure types (`SpectralComponent`, `Wave`,
//! `spectral_fingerprint`, `InterferenceField`) live in `dowiz_core::wave`.
//! The three `InterferenceField` methods that sample the wall clock
//! (`composite`, `xyz_state`, `prune_decayed`) are wrapped here as free
//! functions that stamp `crate::now_ms()`.

pub use dowiz_core::wave::*;

/// `InterferenceField::composite` at the current wall-clock time.
pub fn field_composite(field: &InterferenceField) -> f64 {
    field.composite(crate::now_ms())
}

/// `InterferenceField::xyz_state` at the current wall-clock time.
pub fn field_xyz_state(field: &InterferenceField) -> crate::trig::Xyz {
    field.xyz_state(crate::now_ms())
}

/// `InterferenceField::prune_decayed` at the current wall-clock time.
pub fn field_prune_decayed(field: &mut InterferenceField, threshold: f64) -> usize {
    field.prune_decayed(threshold, crate::now_ms())
}
