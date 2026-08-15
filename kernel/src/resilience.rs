//! resilience.rs — std host shim. The pure backup/failover logic lives in
//! `dowiz_core::resilience`. The clock-stamped entry points take `now_ms`
//! explicitly; this module has no kernel-side callers, so no free-function
//! wrappers are needed — a caller would just pass `crate::now_ms()`.

pub use dowiz_core::resilience::*;
