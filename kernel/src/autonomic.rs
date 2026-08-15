//! `autonomic` — re-exported from the no_std core.
//!
//! The pure half (bounded `BoundedRate` newtype + the gain-scheduling control
//! law `schedule`/`schedule_into_breaker`, `LAW_TABLE`, `FdrAdjustment`) lives in
//! `dowiz_core::autonomic`. The std FDR serialization (`emit`/`write_to_ring`)
//! is retired: route records through `fdr::emit_event`/`fdr::emit_alarm` hooks.

pub use dowiz_core::autonomic::*;
