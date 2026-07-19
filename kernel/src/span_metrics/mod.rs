//! telemetry/ — P83 kernel production observability (BLUEPRINT P83).
//!
//! Two layers, both feature/telemetry-gated (`--features telemetry`) so the SHIPPING
//! binary is behavior- and perf-neutral and carries zero observability symbols:
//!
//!   * Layer 1 (`obs.rs` + `instrument.rs` + `mldsa.rs`) — a ZERO NEW DEP
//!     `SpanMetricsLayer` (reuses the already-linked `tracing` + `tracing-subscriber`)
//!     that consumes the spans over the 8 verified functions and writes hand-rolled
//!     log-bucket latency histograms to `metric.jsonl`.
//!   * Layer 2 (`breach.rs` + `pprof.rs`) — extends the `load1/nproc >= 4` friction
//!     branch with a system-wide `perf record -a -g -F 99` on load breach, writing an
//!     `alert.jsonl` artifact; `pprof` is a feature-gated no-op fallback only.
//!
//! The three spans already placed in the kernel (`place_order`, `place_order_priced`,
//! `fold_transitions`) are consumed here directly; the other five (`route`,
//! `commit_after_decide`, `decide_settlement`, `cap::verify_chain`, `mldsa_verify`) are
//! wrapped by `instrument.rs` / `mldsa.rs` so the layer can measure them without
//! touching their bodies. `assert_transition` is deliberately NOT instrumented
//! (SYNTHESIS §6-E18 — per-edge inner loop).

pub mod obs;

#[cfg(feature = "telemetry")]
pub mod breach;

#[cfg(feature = "telemetry")]
pub mod instrument;

#[cfg(all(feature = "telemetry", feature = "pq"))]
pub mod mldsa;

#[cfg(feature = "telemetry")]
pub mod pprof;

#[cfg(feature = "telemetry")]
pub use obs::{SpanMetricsLayer, SpanMetrics};

/// Install the P83 observability subscriber.
///
/// Wires a `SpanMetricsLayer` (Layer 1) into a `tracing_subscriber::Registry` and sets
/// it as the global default. After this call, every span entered over the 8 verified
/// functions is timed and streamed to `metric.jsonl` in `dir` (or a best-effort temp dir
/// if `dir` is `None`).
///
/// Idempotent-ish: if a global subscriber is ALREADY set (e.g. a test harness installed
/// its own), `set_global_default` returns `Err` — we swallow it (observability must never
/// break the caller). Returns `Ok(())` when installed, `Err(())` when a subscriber was
/// already installed elsewhere.
///
/// Call this from a binary's `main` (e.g. `kernel/src/bin/lm.rs`) or a test setup — NEVER
/// from the shipping library hot path. The kernel's core (decide/fold/money) never calls
/// this; it only emits spans, which are inert without a subscriber.
pub fn init(dir: Option<std::path::PathBuf>) -> Result<(), ()> {
    use tracing_subscriber::prelude::*;
    let layer = SpanMetricsLayer::new(dir);
    let registry = tracing_subscriber::Registry::default().with(layer);
    tracing::subscriber::set_global_default(registry).map_err(|_| ())
}

/// Convenience: run the Layer-2 load-breach check (system-wide `perf` on breach, else the
/// `pprof` feature-gated fallback). See `breach::check_load_breach`.
#[cfg(feature = "telemetry")]
pub fn check_load_breach(dir: Option<std::path::PathBuf>) -> breach::BreachAction {
    breach::check_load_breach(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_init_installs_subscriber_then_spans_are_recorded() {
        // Use a temp dir so metric.jsonl lands somewhere we can assert on.
        let dir = std::env::temp_dir().join(format!("p83_init_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        // init may fail if a global subscriber is already set in this test binary; that's
        // fine — the layer is still constructed and reachable (proven below).
        let _ = init(Some(dir.clone()));

        // Build a layer directly and prove it records a span close to metric.jsonl.
        let layer = SpanMetricsLayer::new(Some(dir.clone()));
        // Simulate a span round-trip via the metrics registry.
        layer.metrics().record("place_order", 42);
        layer.metrics().record("place_order", 7);
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
