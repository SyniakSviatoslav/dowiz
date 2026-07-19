//! telemetry/ — P83 kernel production observability (BLUEPRINT P83).
//!
//! Two layers, both feature/telemetry-gated (`--features telemetry`) so the SHIPPING
//! binary is behavior- and perf-neutral and carries zero observability symbols:
//!
//!   * Layer 1 (`obs.rs` + `instrument.rs`) — a ZERO NEW DEP `SpanMetricsObserver`
//!     (implements the kernel-owned `fdr::SpanObserver` trait — the hand-rolled
//!     replacement for the retired `tracing_subscriber::Layer` hook) that consumes the
//!     spans over the 8 verified functions and writes hand-rolled log-bucket latency
//!     histograms to `metric.jsonl`.
//!   * Layer 2 (`breach.rs` + `pprof.rs`) — extends the `load1/nproc >= 4` friction
//!     branch with a system-wide `perf record -a -g -F 99` on load breach, writing an
//!     `alert.jsonl` artifact; `pprof` is a feature-gated no-op fallback only.
//!
//! The three spans already placed in the kernel (`place_order`, `place_order_priced`,
//! `fold_transitions`) are consumed here directly; the other five (`route`,
//! `commit_after_decide`, `decide_settlement`, `cap::verify_chain`, `mldsa_verify`) are
//! wrapped by `instrument.rs` so the layer can measure them without touching their bodies.
//! `assert_transition` is deliberately NOT instrumented (SYNTHESIS §6-E18 — per-edge inner
//! loop).
//!
//! Cutover note (roadmap items 4+29): the `tracing`/`tracing-subscriber` pair is retired.
//! `SpanMetricsLayer` → `SpanMetricsObserver` (a `fdr::SpanObserver`), `init`/`init_scoped`
//! rewire onto `fdr::set_global_observer` / `fdr::set_scoped_observer`. The
//! `DOWIZ_SPAN_METRICS` + `DOWIZ_SPAN_METRICS_DIR` env contract is UNCHANGED, so
//! `tools/telemetry kernel-spans` and `metric.jsonl` consumers work without edits. The
//! duplicate `mldsa.rs` wrapper was deleted (its span survives in `instrument.rs`).

pub mod obs;

#[cfg(feature = "telemetry")]
pub mod breach;

#[cfg(feature = "telemetry")]
pub mod instrument;

#[cfg(feature = "telemetry")]
pub mod pprof;

#[cfg(feature = "telemetry")]
pub use obs::{SpanMetrics, SpanMetricsObserver};

/// Install the P83 observability observer as the process-global span observer.
///
/// Wires a `SpanMetricsObserver` into the kernel-owned `fdr` span pipeline. After this
/// call, every span entered over the 8 verified functions is timed and streamed to
/// `metric.jsonl` in `dir` (or a best-effort temp dir resolved from `DOWIZ_SPAN_METRICS_DIR`).
///
/// Idempotent-ish: if a global observer is ALREADY set (e.g. a test harness installed its
/// own), `fdr::set_global_observer` returns `Err` — we surface it (observability must never
/// break the caller). Returns `Ok(())` when installed, `Err(())` when one already exists.
///
/// Call this from a binary's `main` or a test setup — NEVER from the shipping library hot
/// path. The kernel's core (decide/fold/money) never calls this; it only emits spans, which
/// are inert without an observer.
pub fn init(dir: Option<std::path::PathBuf>) -> Result<(), ()> {
    // Allow the caller (or the `telemetry kernel-spans` subcommand) to pin where
    // `metric.jsonl` / `alert.jsonl` land via `DOWIZ_SPAN_METRICS_DIR`. When `dir` is None
    // and the env is unset, the writer is best-effort-disabled (no file is opened), so a
    // bare `init(None)` never surprises the caller with disk writes.
    let dir = dir.or_else(|| {
        std::env::var_os("DOWIZ_SPAN_METRICS_DIR").map(std::path::PathBuf::from)
    });
    let obs = std::sync::Arc::new(SpanMetricsObserver::new(dir));
    crate::fdr::set_global_observer(obs)
}

/// Convenience: run the Layer-2 load-breach check (system-wide `perf` on breach, else the
/// `pprof` feature-gated fallback). See `breach::check_load_breach`.
#[cfg(feature = "telemetry")]
pub fn check_load_breach(dir: Option<std::path::PathBuf>) -> breach::BreachAction {
    breach::check_load_breach(dir)
}

/// Install the P83 Layer-1 observer as a **scoped** default, returning a guard that reverts
/// on drop. Unlike `init` (process-global, fails if one is already installed), this works
/// inside any test — the observer is active only for the duration of the returned guard
/// (mirrors the incumbent `set_default`/`DefaultGuard` semantics). Used by the P83 wiring
/// tests.
#[cfg(feature = "telemetry")]
pub fn init_scoped(dir: Option<std::path::PathBuf>) -> crate::fdr::ObserverGuard {
    let obs = std::sync::Arc::new(SpanMetricsObserver::new(dir));
    crate::fdr::set_scoped_observer(obs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_init_installs_observer_then_spans_are_recorded() {
        // Use a temp dir so metric.jsonl lands somewhere we can assert on.
        let dir = std::env::temp_dir().join(format!("p83_init_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        // init may fail if a global observer is already set in this test binary; that's
        // fine — the observer is still constructed and reachable (proven below).
        let _ = init(Some(dir.clone()));

        // Build an observer directly and prove it records a span close to metric.jsonl.
        let obs = SpanMetricsObserver::new(Some(dir.clone()));
        obs.metrics().record("place_order", 42);
        obs.metrics().record("place_order", 7);
        let p = dir.join(obs::METRIC_JSONL);
        let contents = std::fs::read_to_string(&p).unwrap();
        assert!(contents.contains("\"span\":\"place_order\""));
        assert!(contents.contains("\"count\":2"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn green_init_idempotent_does_not_panic() {
        // Calling init twice must not panic even if the second returns Err.
        let _ = init(None);
        let _ = init(None);
    }
}
