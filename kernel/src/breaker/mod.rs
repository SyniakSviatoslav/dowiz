//! `breaker` — re-exported from the no_std core.
//!
//! The pure fault-containment circuit breaker (state machine, signal fitting,
//! transition graph + golden signature, audit chain, replay store) lives in
//! `dowiz_core::breaker`. The kernel-side durable FDR ring mirror is a future
//! seam: implement `dowiz_core::breaker::AuditMirror` for `fdr::RingHandle` and
//! construct via `AuditChain::with_mirror(...)` when a Tier-1 flight-recorder
//! mirror is wired back in.

pub use dowiz_core::breaker::*;
