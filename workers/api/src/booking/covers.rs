//! PURE. THE GUESTS A KITCHEN PREPS FOR (W-PREP, K14 of the kitchen
//! report): the party sizes of the bookings whose slot falls in `[from, to)`
//! and that are still coming or sitting -- requested, confirmed, seated.
//! A booking declined, cancelled, finished or missed is nobody to cook for.
//!
//! The status is the FOLD of the booking's events (`fold_status`), never the
//! cached column; a booking whose history does not replay is not counted
//! and is counted as unreadable instead, so the prep list can say so.

use dowiz_hub::table::Table;
use dowiz_kernel::reservation::ReservationStatus;
use serde_json::Value;

use super::{events_of, fold_status, K_RSV};

/// Guests to cook for, and bookings whose history could not be read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Covers {
    pub guests: i64,
    pub unreadable: i64,
}

/// Is a booking in this state someone the kitchen still cooks for?
pub fn coming(s: ReservationStatus) -> bool {
    matches!(s, ReservationStatus::Requested | ReservationStatus::Confirmed | ReservationStatus::Seated)
}

/// The covers of the bookings in `[from_min, to_min)` (slot minutes).
pub fn between(t: &Table, from_min: i64, to_min: i64) -> Covers {
    let mut c = Covers::default();
    for (id, j) in t.all(K_RSV) {
        let Ok(r) = serde_json::from_str::<Value>(&j) else {
            c.unreadable += 1;
            continue;
        };
        let slot = r.get("slot_min").and_then(Value::as_i64).unwrap_or(i64::MIN);
        if slot < from_min || slot >= to_min {
            continue;
        }
        match fold_status(&events_of(t, &id)) {
            Ok(s) if coming(s) => c.guests += r.get("party").and_then(Value::as_i64).unwrap_or(0).max(0),
            Ok(_) => {}
            Err(_) => c.unreadable += 1,
        }
    }
    c
}

/// The bookings image's bytes as the venue's object holds them; `None`
/// (no image yet) is no bookings. An unreadable image is an error.
pub fn of_image(bytes: Option<&[u8]>, from_min: i64, to_min: i64) -> Result<Covers, String> {
    let Some(b) = bytes else { return Ok(Covers::default()) };
    let t = Table::load(b, super::BOOKINGS_BYTES).map_err(|_| "the bookings image is unreadable".to_string())?;
    Ok(between(&t, from_min, to_min))
}

#[cfg(test)]
#[path = "covers/tests.rs"]
mod tests;
