//! tracker.rs — std host shim. The pure tracking/logging (`TrackedError`,
//! `LoggedEvent`, `EventLog`, `TelemetryCollector`, `ReverseReplay`,
//! `InverseSimulator`) lives in `dowiz_core::tracker`. The clock-stamped
//! constructors (`TrackedError::new`, `LoggedEvent::new`) are wrapped here as
//! free functions that stamp `crate::now_ms()`.

pub use dowiz_core::tracker::*;

/// `TrackedError::new` stamped with the current wall clock.
pub fn tracked_error_new(
    module: &'static str,
    kind: &'static str,
    message: impl Into<String>,
) -> TrackedError {
    TrackedError::new(module, kind, message, crate::now_ms())
}

/// `LoggedEvent::new` stamped with the current wall clock.
pub fn logged_event_new(
    module: &'static str,
    event_type: &'static str,
    payload: Vec<u8>,
) -> LoggedEvent {
    LoggedEvent::new(module, event_type, payload, crate::now_ms())
}
