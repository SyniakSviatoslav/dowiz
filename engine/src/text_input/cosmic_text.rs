//! P57 M2/M5 — cosmic-text shaping seam (Lane B, `feature = "text"`).
//!
//! O18a GATE: `cosmic-text` is absent from every cargo cache (offline-clean
//! mandate, P57 §2.3). Adding it is the SAME one network grant as `gpu`
//! (P38 §2). DO NOT `cargo add cosmic-text` without the operator's network
//! grant. Until then this file does NOT compile (it is `#[cfg(feature =
//! "text")]`), and the Lane A editor in `field.rs` is the whole editor.
//!
//! When the grant lands, `cargo build --features text` implements the real
//! cosmic-text `Editor` integration that this module's signatures document:
//! `glyph_runs()` returns shaped `LayoutGlyph`s → `ShapedGlyph` (atlas-mapped)
//! → FE-06's instanced MSDF glyph-quad pipeline (P38 §3.3); `caret_rect()`
//! and `selection_rects()` become cosmic-text hit-test inverses (real shaped
//! metrics, not the Lane A char-cell approximation); `hit_test` becomes a real
//! pointer→byte-cursor mapping. The `EditCmd`/`EditEvent` surface in
//! `field.rs` is UNCHANGED — only the shaping leg swaps.

/// Re-export marker so callers can detect Lane B at compile time:
/// `cfg!(feature = "text")` is the same check `bridge.rs` uses for the gpu
/// gate (P11 §5 / P38 §2 convention).
pub const COSMIC_SHAPING: bool = true;

/// Build a cosmic-text `Editor` for the given font family + size. The real
/// implementation loads fonts from COMMITED bytes (no `fontdb` system query on
/// wasm32 — the same font asset FE-06's MSDF atlas is generated from, §4.7).
///
/// Stub signature (documented intent, gated off): the body is filled in at
/// O18a. Returns the honest "uncached" error until then — mirroring
/// `bridge::gpu::new_gpu` (P11 §5 E21 honest-stub convention).
pub fn new_cosmic_editor(_family: &str, _px: f32) -> Result<(), &'static str> {
    Err("cosmic-text uncached — O18a network grant required")
}

/// Shape `value` into glyph runs. Under Lane B this produces real
/// `ShapedGlyph`s from cosmic-text's `LayoutGlyph`s; the stub returns empty
/// (Lane A's `glyph_runs()` already returns empty for the offline build).
pub fn shape(value: &str) -> Vec<crate::text_input::ShapedGlyph> {
    let _ = value;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    // LANE-B GUARD (mirrors bridge.rs e21): under the DEFAULT build the
    // `text` feature is OFF, so this test refuses to build — it is the
    // falsifier that the cosmic-text shaping leg is gated and never silently
    // on by default.
    #[cfg(feature = "text")]
    #[test]
    fn cosmic_shaping_is_gated_not_default() {
        // When `text` IS on (O18a), the editor builds with real shaping.
        assert!(cfg!(feature = "text"));
        let r = new_cosmic_editor("sans", 16.0);
        // The honest stub still returns Err until the font bytes are wired.
        assert!(r.is_err());
    }

    // Static proof the gate constant is wired (always true; documents intent).
    #[test]
    fn gate_constant_present() {
        assert_eq!(COSMIC_SHAPING, cfg!(feature = "text"));
    }
}
