//! Reservations and entry passes — the transport for a kernel domain.
//!
//! **This module decides nothing.** Whether a booking may be made, whether a
//! transition is legal, whether a pass is valid — every one of those answers
//! comes from `dowiz_kernel::reservation` and `dowiz_kernel::pass`. What is here
//! is storage, HTTP and the venue's key: bytes in, bytes out.
//!
//! # Events, not state
//!
//! `status` on a reservation record is a cache. The authority is the event
//! log, appended once per transition and never updated, and [`fold_status`]
//! replays it through the kernel. When the two disagree the events win — and
//! `detail` says so out loud instead of quietly trusting the column.
//!
//! # Append-only
//!
//! Nothing here erases a booking. "Delete" in the owner console is the FSM's
//! `CANCELLED_BY_VENUE`, one more event; the record and its history stay.
//!
//! # The files (F14, along the rulers this file used to carry)
//!
//! * [`create`] — a guest (no account) or the venue books a table;
//! * [`reservations`] — a booking read and moved by whoever holds it;
//! * [`venue`] — the owner console: the day's list and the venue's moves;
//! * [`store`] — every write, PURE over the image, so it is tested natively;
//! * [`guest`] — who may do what, and the guest's contact, PURE;
//! * [`floor`] — the plan, the tables, the availability;
//! * [`pass`] — entry passes and the venue's key.

use serde::Deserialize;

use dowiz_kernel::reservation::{self, ReservationStatus};

mod create;
mod floor;
/// G8: the guest's contact emptied out of their bookings on forget.
pub(crate) mod forget;
mod guest;
mod hours;
mod pass;
mod reservations;
mod store;
mod venue;

pub use create::create;
pub(crate) use floor::table_key;
pub use floor::{availability, get_plan, set_plan};
pub use pass::{issue_pass, verify_pass};
pub use reservations::{action, detail, list};
pub use venue::{venue_action, venue_day};

/// Minutes since the Unix epoch — the kernel's unit for a slot.
///
/// TAKES THE INSTANT RATHER THAN READING ONE. Every caller is a handler that
/// already has the request's own `now`, and a slot computed from a SECOND read
/// of the clock can land a minute away from the one the rest of the handler
/// used — which for a booking window is the difference between accepted and
/// refused.
fn now_min(now_ms: i64) -> i64 {
    now_ms / 60_000
}

/// A stable 64-bit id from a string, for the kernel types that take integers.
/// FNV-1a: not a hash anybody's security rests on, only a way to carry a TEXT
/// primary key into a `u64` field without inventing a second id space.
fn id64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[derive(Deserialize)]
struct EventRow {
    to_status: String,
    seq: i64,
    actor: String,
    reason: String,
    at_ms: i64,
}

#[derive(Deserialize)]
struct ReservationRow {
    id: String,
    // `location_id` is on the record and not read: the venue is already
    // decided by the object this record came out of.
    party: i64,
    slot_min: i64,
    occasion: String,
    contact_name: String,
    contact_phone: String,
    status: String,
    created_at_ms: i64,
    /// The table, when the guest chose one. `default` because every booking
    /// written before the floor existed has neither field, and a read model
    /// that refused those would make the venue's history unreadable.
    #[serde(default)]
    zone_id: Option<String>,
    #[serde(default)]
    table_n: Option<i64>,
    /// The guest's phone as the venue's customer key (`guest::phone_key`).
    /// Absent on every booking made before a guest could book.
    #[serde(default)]
    phone_key: Option<String>,
}

/// Replay a reservation's events through the kernel.
///
/// Returns the folded status. An event naming a status the kernel does not know
/// stops the replay — a booking whose history cannot be read must not be shown
/// as if it could.
fn fold_status(events: &[EventRow]) -> std::result::Result<ReservationStatus, String> {
    let mut steps = Vec::with_capacity(events.len());
    for e in events {
        let s = ReservationStatus::from_str(&e.to_status)
            .ok_or_else(|| format!("unknown status {:?} in event log", e.to_status))?;
        steps.push(s);
    }
    // The first event IS the creation, so the fold starts at the state it names
    // and replays the rest.
    let Some((first, rest)) = steps.split_first() else {
        return Err("reservation has no events".into());
    };
    reservation::fold_transitions(*first, rest)
        .map_err(|(e, reached)| format!("{} (stopped at {})", e.message(), reached.as_str()))
}

/// The venue's bookings: the reservations and their events, in ONE image.
///
/// ONE AGGREGATE, ONE TRANSACTION. A reservation and its event log are the same
/// thing changing — the status is the FOLD of the events — so a transition is
/// one write, and there is no window in which a reservation says one status
/// and its events another.
pub const IMAGE_BOOKINGS: &str = "bookings";
pub const BOOKINGS_BYTES: usize = dowiz_hub::CEILING_BYTES;

const K_RSV: &str = "rsv";
const K_EV: &str = "ev";

/// An event's key. ZERO-PADDED, because keys sort as strings and `.../10` must
/// not come before `.../2` — that would replay a reservation's history in the
/// wrong order and the fold would reach a different status.
fn ev_key(reservation_id: &str, seq: i64) -> String {
    format!("{reservation_id}/{seq:012}")
}

/// The complement base for "newest slot first" scans: keys sort ascending
/// and the answer wants descending. Minutes in ten thousand years, derived
/// here rather than written so the next person can check it.
const SLOT_MAX: i64 = 10_000 * 365 * 24 * 60;

/// The index that answers "this user's bookings, newest slot first".
fn user_key(user_id: &str, slot_min: i64, id: &str) -> String {
    format!("rsv.user/{user_id}/{:012}/{id}", SLOT_MAX - slot_min)
}

/// The index that answers "this phone's bookings" -- what the guest cap
/// counts. The key is the venue's HMAC customer key, never the digits.
fn phone_index(phone_key: &str, slot_min: i64, id: &str) -> String {
    format!("rsv.phone/{phone_key}/{:012}/{id}", SLOT_MAX - slot_min)
}

async fn load_bookings(place: &crate::hubstore::Place) -> worker::Result<dowiz_hub::table::Table> {
    Ok(crate::hubstore::load_table(place, IMAGE_BOOKINGS, BOOKINGS_BYTES).await?.table)
}

/// One reservation, or nothing. The image is this venue's object, so there is
/// no other venue's booking in it to exclude.
fn reservation_of(t: &dowiz_hub::table::Table, id: &str) -> Option<ReservationRow> {
    t.get(K_RSV, id).and_then(|j| serde_json::from_str(&j).ok())
}

/// A reservation's events, oldest first, which is the order the kernel replays.
fn events_of(t: &dowiz_hub::table::Table, reservation_id: &str) -> Vec<EventRow> {
    let prefix = format!("{reservation_id}/");
    let mut rows: Vec<EventRow> = t
        .all(K_EV)
        .into_iter()
        .filter(|(k, _)| k.starts_with(&prefix))
        .filter_map(|(_, j)| serde_json::from_str(&j).ok())
        .collect();
    rows.sort_by_key(|e: &EventRow| e.seq);
    rows
}

#[cfg(test)]
mod tests;
