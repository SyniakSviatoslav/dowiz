//! `dowiz_core::sanitize` — fail-closed f64/f32 boundary sanitizers.
//!
//! Every public API that accepts raw `f64`/`f32` MUST call these at the boundary
//! so a NaN/Inf from a degenerate computation (or untrusted input) can never
//! propagate through the kernel. Non-finite → `0.0`; `sanitize_normalized`
//! additionally clamps to `[0, 1]` for normalized metric values.
//!
//! Pure `core` — no alloc, no std. This is the extracted form of the crate-root
//! `sanitize_f64`/`sanitize_f32`/`sanitize_normalized` free functions that
//! `dowiz-kernel` previously defined in `lib.rs`; the kernel re-exports them so
//! `crate::sanitize_f64` keeps resolving unchanged.

/// Sanitize a raw f64: NaN/Inf → 0.0 (fail-closed for system stability).
pub fn sanitize_f64(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

/// Sanitize a raw f32: NaN/Inf → 0.0.
pub fn sanitize_f32(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// Sanitize AND clamp to [0, 1] — for normalized metric values.
/// NaN/Inf → 0.0; values outside range clamped.
pub fn sanitize_normalized(v: f64) -> f64 {
    if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_f64_normal_values() {
        assert_eq!(sanitize_f64(0.0), 0.0);
        assert_eq!(sanitize_f64(1.0), 1.0);
        assert_eq!(sanitize_f64(-1.0), -1.0);
        assert_eq!(sanitize_f64(f64::MAX), f64::MAX);
        assert_eq!(sanitize_f64(f64::MIN), f64::MIN);
    }

    #[test]
    fn sanitize_f64_nan_inf() {
        assert_eq!(sanitize_f64(f64::NAN), 0.0, "NaN must become 0.0");
        assert_eq!(sanitize_f64(f64::INFINITY), 0.0, "Inf must become 0.0");
        assert_eq!(sanitize_f64(f64::NEG_INFINITY), 0.0, "-Inf must become 0.0");
    }

    #[test]
    fn sanitize_f64_output_always_finite() {
        assert_eq!(sanitize_f64(f64::NAN), 0.0);
        assert_eq!(sanitize_f64(f64::INFINITY), 0.0);
        assert_eq!(sanitize_f64(f64::NEG_INFINITY), 0.0);
        for &v in &[0.0, 1.0, -1.0, f64::MAX, f64::MIN, 42.0, -0.5, 1e308, -1e-308] {
            let out = sanitize_f64(v);
            assert!(out.is_finite(), "sanitize_f64({v:e}) = {out:e} must be finite");
            assert_eq!(out, v, "sanitize_f64({v:e}) must preserve finite value, got {out:e}");
        }
    }

    #[test]
    fn sanitize_normalized_clamps() {
        assert_eq!(sanitize_normalized(0.5), 0.5);
        assert_eq!(sanitize_normalized(0.0), 0.0);
        assert_eq!(sanitize_normalized(1.0), 1.0);
        assert_eq!(sanitize_normalized(1.5), 1.0, "above range clamps to 1");
        assert_eq!(sanitize_normalized(-0.5), 0.0, "below range clamps to 0");
        assert_eq!(sanitize_normalized(f64::NAN), 0.0);
        assert_eq!(sanitize_normalized(f64::INFINITY), 0.0);
    }

    #[test]
    fn sanitize_f32_nan_inf() {
        assert_eq!(sanitize_f32(f32::NAN), 0.0);
        assert_eq!(sanitize_f32(f32::INFINITY), 0.0);
        assert_eq!(sanitize_f32(f32::NEG_INFINITY), 0.0);
        assert_eq!(sanitize_f32(3.5), 3.5);
    }
}
