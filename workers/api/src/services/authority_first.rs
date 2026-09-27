//! AUTHORITY BEFORE THE BODY (W-FIX O9, 2026-09-27).
//!
//! THE DEFECT (W-AUDIT O9). These routes parsed the request body -- untyped
//! `Value`s, lenient `unwrap_or`s, a whole backup bundle -- BEFORE asking who
//! the caller was. Tens of megabytes of JSON from anybody cost the Worker its
//! CPU (10 ms on the Free plan) before the 401 was ever reached. None of them
//! needs the body to find its venue, so the door comes first.
//!
//! Routes that DO read their venue out of the body (`location_id` in it, then
//! `staff_at`/`owner_at` against it) are not listed: for them the body is the
//! only place the venue is named. The order is locked by `tests.rs`, which
//! reads the handlers' own source.

#[cfg(test)]
mod tests;
