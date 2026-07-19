//! telemetry/mldsa.rs — P83 Layer 1 span wrapper for `mldsa verify`, behind `pq`.
//!
//! SYNTHESIS PERFORMANCE AUDIT 2026-07-18 §3.3-C4 names `mldsa verify` as the eighth
//! (and only `pq`-gated) of the eight instrumented functions. The REAL verify lives at
//! `crate::pq::dsa::verify` (FIPS 204 ML-DSA-65, KAT-gated byte-exact). We do NOT touch
//! that crypto primitive's body (red-line: never mutate PQ verify); we add a thin
//! `#[instrument]` wrapper so the `SpanMetricsLayer` can measure it when BOTH `telemetry`
//! AND `pq` are enabled. With `pq` off, this module compiles to nothing.

/// Wrapper: `pq::dsa::verify` (ML-DSA-65 signature verify) — behind `pq` only.
#[cfg(all(feature = "telemetry", feature = "pq"))]
#[tracing::instrument(name = "mldsa_verify", skip_all, level = "info")]
pub fn verify(
    pk: &crate::pq::dsa::MlDsa65Pk,
    msg: &[u8],
    sig: &crate::pq::dsa::MlDsa65Sig,
) -> bool {
    crate::pq::dsa::verify(pk, msg, sig)
}
