//! `openobserve.rs` — std shim over the pure no_std core.
//!
//! The columnar metric store, log store, trace parser, and [`ObservabilityHub`] live in
//! `dowiz_core::openobserve` and are re-exported here. This shim adds ONLY the std
//! sampling entry point ([`snapshot_now`]) — samples `/proc` + stamps the wall clock before
//! delegating to the no_std core.

pub use dowiz_core::openobserve::*;

/// Take a telemetry snapshot (samples `/proc` + stamps the wall clock, then delegates to
/// the no_std aggregator core).
pub fn snapshot_now(
    hub: &mut ObservabilityHub,
) -> crate::telemetry_aggregator::TelemetrySnapshot {
    let timestamp_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;
    let cpu = crate::typed_metrics::proc_cpu_sample_from_proc_self();
    let mem = crate::typed_metrics::mem_sample_from_proc_self();
    hub.snapshot(timestamp_us, cpu, mem)
}
