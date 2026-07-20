//! Shared wasm-safe monotonic clock (item 60, gaps G3 + G11; the item-62 shared
//! wasm-clock leg, one design).
//!
//! **Native:** `std::time::Instant` (monotonic). The first call captures a start
//! instant; every later call returns elapsed microseconds since that start.
//!
//! **wasm32 (`wasm32-unknown-unknown`):** `Instant::now()` *panics* on this
//! target (the same trap the FDR module guards, `fdr/mod.rs:216-224`). The engine
//! crate is **offline-clean** — zero external crates by design — so it cannot
//! import `performance.now()` without pulling a dependency (the `dowiz-wasm`
//! cdylib owns the `wasm-bindgen` `performance.now()` binding per item 62's
//! shared design, which is not in this tree). Per PROCEDURE step 9 / item-62, the
//! engine states its wasm leg as a **named absence**: `now_micros()` returns
//! `None` on wasm, so the timing path takes *no* `Instant::now()` and the default
//! (non-`telemetry`) engine build stays untimed-but-accounted on wasm. This is
//! the blueprint's explicitly permitted "named `Absence` where a surface
//! genuinely cannot time on wasm" — not a fabricated `0`.
//!
//! Uniform `Option<u64>` return so callers are identical across targets: `Some`
//! carries real elapsed microseconds on native; `None` is the named absence on
//! wasm. Callers MUST treat `None` as "untimed" and never coerce it to `0`.

use std::time::Instant;

/// Capture the monotonic start instant once (native only). Confined to the
/// non-wasm cfg so the wasm build contains no `Instant` reference at all.
#[cfg(not(target_arch = "wasm32"))]
fn epoch() -> Instant {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// Monotonic microseconds since the first call, or `None` on wasm (named
/// absence — engine is offline-clean, no `performance.now()` dep; the wasm cdylib
/// owns that binding). Never calls `Instant::now()` on wasm.
#[cfg(not(target_arch = "wasm32"))]
pub fn now_micros() -> Option<u64> {
    Some(epoch().elapsed().as_micros() as u64)
}

/// wasm leg of the shared clock: named absence. No `Instant`, no dependency.
#[cfg(target_arch = "wasm32")]
pub fn now_micros() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Native: the clock is monotonic and advances (same instant twice differs
    // only by elapsed >= 0; two reads a hair apart are non-decreasing).
    #[test]
    fn clock_advances_on_native() {
        let a = now_micros();
        // On wasm this is None (named absence) — still a valid contract.
        if let Some(a_us) = a {
            let b = now_micros().expect("clock must keep returning Some on native");
            assert!(b >= a_us, "monotonic clock must not go backwards");
        }
    }
}
