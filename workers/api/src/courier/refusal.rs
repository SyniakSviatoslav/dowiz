//! A REFUSAL WHOSE STATUS WAS COMPUTED CANNOT PANIC (W-FIX H3, 2026-09-27).
//!
//! THE DEFECT (W-AUDIT O13). `staff_any_at` answered `room_admits_any`'s
//! `(status, text)` with `Response::error(m, s).unwrap()`. worker's
//! `Response::error` is an `Err` for any status outside 400..=599, so the day
//! somebody returns a 3xx (or a 200 "not really") from the capability table,
//! the courier's and the room's doors panic -- and a panic in a Worker is the
//! isolate. Every code it returns today is 4xx; this makes that a property of
//! the code, not of the table's current rows.

use worker::{Response, ResponseBuilder};

/// The status a refusal is sent with: its own when it is an error status,
/// 500 otherwise -- a refusal that claims success is itself the fault.
pub(crate) fn status(s: u16) -> u16 {
    if (400..=599).contains(&s) {
        s
    } else {
        500
    }
}

/// `Response::error`, with no `Result` left to unwrap.
pub(crate) fn of(text: &str, s: u16) -> Response {
    ResponseBuilder::new().with_status(status(s)).fixed(text.as_bytes().to_vec())
}

#[cfg(test)]
mod tests;
