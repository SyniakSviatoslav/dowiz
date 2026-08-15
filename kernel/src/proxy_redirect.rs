//! proxy_redirect.rs — std host shim (pure pool/rotation logic lives in
//! `dowiz_core::proxy_redirect`; the wall-clock `now_us()` seam stays here).
//!
//! The no_std core injects timestamps as explicit `now_us_val` parameters;
//! this shim supplies the real microsecond wall clock so callers can stamp
//! `record_success`/`record_failure` with live time.

pub use dowiz_core::proxy_redirect::*;

/// Monotonic timestamp in microseconds (wall clock, platform-specific).
pub fn now_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}
