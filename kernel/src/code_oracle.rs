//! code_oracle.rs — std host shim. The pure change-prediction + ETA oracle
//! (`ChangeRecord`, `EtaOracle`) lives in `dowiz_core::code_oracle`.
//! `EtaOracle::record` takes `now_ms` explicitly; here it is wrapped as
//! `record_now` which stamps `crate::now_ms()`. (`EtaOracle::new` is clock-free
//! and used directly.)

pub use dowiz_core::code_oracle::*;

/// `EtaOracle::record` stamped with the current wall clock.
pub fn record_now(
    oracle: &mut EtaOracle,
    modules: &[&str],
    added: u64,
    removed: u64,
    eta_minutes: f64,
) {
    oracle.record(modules, added, removed, eta_minutes, crate::now_ms());
}
