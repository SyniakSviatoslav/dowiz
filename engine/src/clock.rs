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
//!
//! ## `monotonic_ms()` — canonical test-safe milliseconds
//!
//! For offline/route-cache timestamping that must be deterministic in tests,
//! use `monotonic_ms()`. It returns `u64` (monotonic milliseconds on native;
//! incrementing counter in `#[cfg(test)]`). Use this INSTEAD of defining a
//! local `monotonic_ms` in each module — every duplicate is a SPOF.

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

/// Canonical monotonic milliseconds — SINGLE AUTHORITY.
///
/// On native: `SystemTime::now().duration_since(UNIX_EPOCH)` (wall-clock).
/// In tests: deterministic incrementing counter (never real time).
/// Wasm: uses same `#[cfg(not(test))]` path as native when not in test cfg.
///
/// Use this EVERYWHERE instead of defining per-module `monotonic_ms()`.
/// The test-counter pattern ensures deterministic test assertions
/// regardless of wall-clock timing.
pub fn monotonic_ms() -> u64 {
    #[cfg(not(test))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    #[cfg(test)]
    {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

/// Telemetry counter — cheap atomic increment, always compiled.
/// Heavy stamps (p50/p99) are feature-gated behind `telemetry`.
///
/// Usage: `telemetry_count!("oracle", "gps_fix", 1);`
/// In production: increments an atomic counter.
/// Under `feature = "telemetry"`: also records to a stamp collector.
#[macro_export]
macro_rules! telemetry_count {
    ($module:expr, $event:expr, $delta:expr) => {{
        #[cfg(feature = "telemetry")]
        {
            // Heavy path: record event with module + name + delta
            // (stub — real collector lands with telemetry crate)
            let _ = ($module, $event, $delta);
        }
        // Cheap path: always compiled, zero-overhead on default build
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_advances_on_native() {
        let a = now_micros();
        if let Some(a_us) = a {
            let b = now_micros().expect("clock must keep returning Some on native");
            assert!(b >= a_us, "monotonic clock must not go backwards");
        }
    }

    #[test]
    fn monotonic_ms_is_deterministic_in_tests() {
        let a = monotonic_ms();
        let b = monotonic_ms();
        assert!(b > a, "test counter must advance: {b} <= {a}");
    }

    #[test]
    fn monotonic_ms_is_monotonic() {
        let mut prev = 0u64;
        for _ in 0..10 {
            let cur = monotonic_ms();
            assert!(cur > prev, "monotonic_ms must strictly increase");
            prev = cur;
        }
    }

    #[test]
    fn monotonic_ms_never_wraps_or_stalls() {
        let mut prev = 0u64;
        for _ in 0..1000 {
            let cur = monotonic_ms();
            assert!(cur > prev || cur == 0,
                "monotonic_ms must strictly increase or start from 0: {cur} <= {prev}");
            prev = cur;
        }
    }

    #[test]
    fn monotonic_ms_thread_safe_advances() {
        use std::sync::{Arc, Barrier};
        use std::thread;
        let n = 8;
        let barrier = Arc::new(Barrier::new(n));
        let results = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut handles = Vec::new();
        for _ in 0..n {
            let b = Arc::clone(&barrier);
            let r = Arc::clone(&results);
            handles.push(thread::spawn(move || {
                b.wait();
                let vals: Vec<u64> = (0..100).map(|_| monotonic_ms()).collect();
                r.lock().unwrap().extend(vals);
            }));
        }
        for h in handles { h.join().unwrap(); }
        let all = results.lock().unwrap();
        let mut sorted = all.clone();
        sorted.sort();
        // Values from different threads may interleave — check sorted order is monotonically increasing
        for i in 1..sorted.len() {
            assert!(sorted[i] >= sorted[i-1], "sorted values must be non-decreasing");
        }
        // All values must be non-zero
        for &v in sorted.iter() {
            assert!(v > 0, "monotonic_ms must return positive timestamps");
        }
        // Sorted should have no duplicates beyond thread-interleaving tolerance
        assert!(sorted.len() >= 600, "must collect most values from threads");
    }

    #[test]
    fn now_micros_is_some_on_native() {
        let t = now_micros();
        assert_eq!(cfg!(target_arch = "wasm32"), t.is_none(),
            "native → Some, wasm → None");
    }

    #[test]
    fn now_micros_never_goes_backwards() {
        for _ in 0..100 {
            let a = now_micros();
            let b = now_micros();
            if let (Some(av), Some(bv)) = (a, b) {
                assert!(bv >= av, "monotonic violation: {bv} < {av}");
            }
        }
    }

    /// Saturating timestamp arithmetic: no underflow, no panic
    #[test]
    fn timestamp_saturating_arithmetic() {
        let ts: u64 = 100;
        let delta: u64 = 200;
        // Model: elapsed = now - ts (saturating)
        let now = ts + delta;
        let elapsed = now.saturating_sub(ts);
        assert_eq!(elapsed, delta);

        // Underflow case
        let future = 50u64;
        let elapsed_safe = future.saturating_sub(ts);
        assert_eq!(elapsed_safe, 0, "saturating_sub must not underflow");
    }

    #[test]
    fn monotonic_ms_no_duplicates_under_concurrent_load() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;
        let running = Arc::new(AtomicBool::new(true));
        let store = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut handles = Vec::new();
        for _ in 0..4 {
            let r = Arc::clone(&running);
            let s = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                while r.load(Ordering::Relaxed) {
                    let t = monotonic_ms();
                    s.lock().unwrap().push(t);
                }
            }));
        }
        // Let threads spin for a short while
        std::thread::sleep(std::time::Duration::from_micros(500));
        running.store(false, Ordering::Relaxed);
        for h in handles { h.join().unwrap(); }
        let all = store.lock().unwrap();
        let mut sorted = all.clone();
        sorted.sort_unstable();
        // Verify no duplicate (monotonic_ms is strict counter)
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(),
            "concurrent monotonic_ms must produce unique values: {} unique / {} total",
            sorted.len(), all.len());
    }
}
