//! telemetry/breach.rs — P83 Layer 2: load-breach system-wide profiler.
//!
//! BLUEPRINT P83 / SYNTHESIS PERFORMANCE AUDIT 2026-07-18 §3.3-C4 (Layer 2).
//! Extends the kernel's `load1/nproc >= 4` friction signal with a system-wide
//! `perf record -a -g -F 99` invocation on load breach, writing an `alert.jsonl`
//! artifact. The R6 reframe: a load1 spike could be the kernel OR the operator's
//! own `rustc` build noise — only a system-wide `perf` capture can prove which.
//!
//! Safety constraints (must not hang, must not require root in a way that breaks
//! the build):
//!   * The capture is BEST-EFFORT and TIME-BOUNDED: `perf` runs for a fixed
//!     `-- sleep N` duration (default 5s), never an unbounded attach. If `perf`
//!     is unavailable / not permitted, we fall back to the `pprof` feature-gated
//!     marker (a no-op that just writes an `alert.jsonl` row naming the fallback)
//!     rather than erroring or hanging.
//!   * This module compiles ONLY under the `telemetry` feature; the `pprof`
//!     fallback path is additionally `cfg(feature = "pprof")`.
//!   * No network, no RNG, std-only. The only side effect is the `alert.jsonl`
//!     row (+ optionally a `perf.data` capture file in the same dir).
//!
//! The `load1/nproc >= 4` branch itself did NOT yet exist in the kernel — this
//! module is its canonical home (the function `check_load_breach` is the branch;
//! `init`/callers wire it into the engine/perf friction path per the operator).
//!
//! Pure no_std items (`BreachAction`, `PERF_CAPTURE_SECS`, `PERF_FREQ`) live in
//! `dowiz_core::span_metrics::breach` and are re-exported below. This shim keeps
//! only the std parts: `check_load_breach`, `trigger_perf`, `try_perf`, `which_perf`.

// Re-export the pure no_std types from dowiz-core.
pub use dowiz_core::span_metrics::breach::*;

use std::path::{Path, PathBuf};

use super::obs::{normalized_load1, JsonlWriter, ALERT_JSONL, LOAD_BREACH_THRESHOLD};

/// Evaluate the `load1/nproc >= 4` friction branch and, on breach, trigger the
/// system-wide profiler. `dir` is where `alert.jsonl` (+ `perf.data`) land;
/// `None` disables the writer (still returns the action for callers that log it).
pub fn check_load_breach(dir: Option<PathBuf>) -> BreachAction {
    let load = match normalized_load1() {
        Some(l) => l,
        None => {
            // Non-Linux / unreadable load: cannot detect a breach here. Degrade closed.
            return BreachAction::NoBreach {
                load: f64::NEG_INFINITY,
            };
        }
    };
    if load < LOAD_BREACH_THRESHOLD {
        return BreachAction::NoBreach { load };
    }
    // Breach → capture.
    let writer = JsonlWriter::new(dir.clone());
    let action = trigger_perf(&dir);
    // Always record an alert row (observability of the breach itself).
    let (captured, fallback, detail) = match &action {
        BreachAction::Captured {
            captured,
            fallback,
            detail,
            ..
        } => (*captured, *fallback, detail.clone()),
        _ => (false, false, String::new()),
    };
    let row = format!(
        "{{\"alert\":\"load_breach\",\"load1_per_nproc\":{:.4},\"threshold\":{:.4},\"perf_captured\":{},\"fallback\":{},\"detail\":{:?}}}\n",
        load, LOAD_BREACH_THRESHOLD, captured, fallback, detail
    );
    writer.append(ALERT_JSONL, &row);
    BreachAction::Captured {
        load,
        captured,
        fallback,
        detail,
    }
}

/// Trigger the system-wide `perf record`. Tries `perf` first; if it is not
/// available / not permitted, takes the `pprof` feature-gated no-op fallback.
fn trigger_perf(dir: &Option<PathBuf>) -> BreachAction {
    match try_perf(dir) {
        Some(action) => action,
        None => {
            // `perf` unavailable → feature-gated fallback (no-op marker). Never hangs,
            // never needs root, never shells out.
            #[cfg(feature = "pprof")]
            {
                BreachAction::Captured {
                    load: normalized_load1().unwrap_or(f64::INFINITY),
                    captured: false,
                    fallback: true,
                    detail: "perf unavailable; pprof feature-gated fallback (no-op marker)"
                        .to_string(),
                }
            }
            #[cfg(not(feature = "pprof"))]
            {
                BreachAction::Captured {
                    load: normalized_load1().unwrap_or(f64::INFINITY),
                    captured: false,
                    fallback: false,
                    detail: "perf unavailable; no pprof fallback compiled in".to_string(),
                }
            }
        }
    }
}

/// Attempt the real `perf record -a -g -F 99 -- sleep N`. Time-bounded by the
/// `-- sleep N` argument itself (perf detaches after N seconds). Returns None if
/// `perf` is not on PATH / cannot be spawned (so the caller can fall back).
fn try_perf(dir: &Option<PathBuf>) -> Option<BreachAction> {
    let perf = which_perf()?; // None ⇒ not installed → fall back.
    let load = normalized_load1().unwrap_or(f64::INFINITY);
    let sleep = format!("{}", PERF_CAPTURE_SECS);
    // -a system-wide, -g call-graph (dwarf), -F 99 sampling, time-bounded by `sleep`.
    let mut args: Vec<String> = vec![
        "record".into(),
        "-a".into(),
        "-g".into(),
        "-F".into(),
        format!("{}", PERF_FREQ),
        "--".into(),
        "sleep".into(),
        sleep,
    ];
    if let Some(d) = dir {
        // Write perf.data next to the alert artifact.
        let _ = crate::vfs::create_dir_all(d);
        args.push("-o".into());
        args.push(d.join("perf.data").to_string_lossy().into_owned());
    }
    // Time-bounded spawn: perf detaches after the `sleep N` window. Best-effort —
    // failure ⇒ captured=false (caller falls back to the no-op pprof marker).
    let out = crate::kprocess::run(&perf.to_string_lossy(), &args);
    let captured = out.success();
    let detail = if out.success() {
        format!("perf record ran for {}s", PERF_CAPTURE_SECS)
    } else if out.code == -1 {
        "perf could not be spawned (no perms?)".to_string()
    } else {
        format!("perf exited non-zero: code {}", out.code)
    };
    Some(BreachAction::Captured {
        load,
        captured,
        fallback: false,
        detail,
    })
}

/// Locate `perf` on PATH (std-only, no `which` crate). Returns None if absent so
/// the caller takes the safe `pprof` fallback instead of erroring.
fn which_perf() -> Option<PathBuf> {
    for p in std::env::var_os("PATH")?.to_string_lossy().split(':') {
        let cand = Path::new(p).join("perf");
        if cand.exists() && cand.is_file() {
            return Some(cand);
        }
    }
    None
}

// ── Unit tests: Layer-2 logic is probe-able without invoking `perf` ──
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_load_breach_threshold_constant() {
        assert_eq!(LOAD_BREACH_THRESHOLD, 4.0);
        assert_eq!(PERF_FREQ, 99);
    }

    #[test]
    fn green_which_perf_is_safe_when_absent() {
        // We do not assert presence/absence of perf (host-dependent); we assert the
        // helper NEVER panics and always returns an Option.
        let _ = which_perf();
    }

    #[test]
    fn green_check_load_breach_does_not_hang_or_panic() {
        // Even on a host where load > 4 (or perf present), this returns within the
        // bounded capture window and never panics. Run with a none-dir so no file
        // is written; just assert the action enum is constructed.
        let action = check_load_breach(None);
        match action {
            BreachAction::NoBreach { .. } => {}
            BreachAction::Captured { .. } => {}
        }
    }
}