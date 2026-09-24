//! Every write to the bookings image, PURE over `Table`.
//!
//! The handlers call these INSIDE `hubstore::with_table`, so each verdict is
//! taken against the image about to be written: two requests a millisecond
//! apart cannot both pass a check done against an earlier read, and
//! `with_table` re-runs the closure when the generation moved. No Worker
//! runtime is needed, so `store/tests.rs` drives the real functions natively.
//!
//! APPEND-ONLY: a booking's record is re-put with a new status, its events are
//! only ever added. Nothing here removes a booking.

use dowiz_hub::table::Table;
use dowiz_hub::tables as floor;
use dowiz_kernel::reservation::{assert_transition, ReservationStatus};
use serde_json::{json, Value};

use super::floor::{held_tables, table_key, table_verdict};
use super::guest::{moves, Side, MAX_OPEN_PER_PHONE};
use super::{ev_key, events_of, fold_status, phone_index, user_key, K_EV, K_RSV};

/// A refusal: the HTTP status and the sentence.
pub(super) type Refusal = (u16, String);

/// Every index key a reservation record owns, DERIVED FROM THE RECORD ALONE.
///
/// ONE FUNCTION FOR BOTH WRITERS. `Table::put` replaces the keys a record owned
/// with exactly this list, so a transition that recomputed only some of them
/// would silently drop the rest -- a status change that took a booking off
/// its guest's cap, or kept a cancelled booking's table held. A booking in a
/// status that no longer holds its table simply stops writing the table key:
/// that IS how a table is released.
pub(super) fn index_of(id: &str, r: &Value) -> Vec<(String, String)> {
    let slot = r.get("slot_min").and_then(Value::as_i64).unwrap_or(0);
    let s = |k: &str| r.get(k).and_then(Value::as_str).filter(|v| !v.is_empty());
    let mut index = Vec::new();
    if let Some(u) = s("user_id") {
        index.push((user_key(u, slot, id), id.to_string()));
    }
    if let Some(p) = s("phone_key") {
        index.push((phone_index(p, slot, id), id.to_string()));
    }
    if floor::holds_table(s("status").unwrap_or("")) {
        if let (Some(z), Some(n)) = (s("zone_id"), r.get("table_n").and_then(Value::as_i64)) {
            index.push((table_key(z, n, slot), id.to_string()));
        }
    }
    index
}

/// A booking about to be written.
pub(super) struct NewBooking {
    pub id: String,
    pub venue: String,
    pub party: i64,
    pub slot_min: i64,
    pub occasion: String,
    pub name: String,
    pub phone: String,
    pub phone_key: Option<String>,
    pub user_id: Option<String>,
    pub table: Option<(String, i64)>,
    pub side: Side,
    pub now_ms: i64,
}

/// The live bookings one phone holds: a status that still holds a table, for
/// a sitting that has not yet ended at `now_min`.
pub(super) fn open_for_phone(t: &Table, phone_key: &str, now_min: i64) -> usize {
    t.scan(&format!("rsv.phone/{phone_key}/"))
        .into_iter()
        .filter_map(|(_, id)| t.get(K_RSV, &id))
        .filter_map(|j| serde_json::from_str::<Value>(&j).ok())
        .filter(|r| floor::holds_table(r.get("status").and_then(Value::as_str).unwrap_or("")))
        .filter(|r| r.get("slot_min").and_then(Value::as_i64).unwrap_or(0) + floor::DWELL_MIN >= now_min)
        .count()
}

/// Write a new booking and its first event(s), or refuse it.
///
/// A GUEST'S booking lands `REQUESTED` -- the venue answers it. The VENUE'S
/// own booking (a phone call taken at the console) lands `CONFIRMED`, as two
/// events signed `VENUE`: the venue asking itself and saying yes is exactly
/// what happened, and the fold replays it through the kernel like any other.
///
/// Answers the status and whether THIS call wrote it. A record already under
/// this id is the same request retried: answered with its status and `false`,
/// never overwritten -- and the caller mints no token for a write it did not
/// make.
pub(super) fn write_new(
    t: &mut Table,
    b: &NewBooking,
    plan: &floor::Plan,
    now_min: i64,
) -> Result<Result<(String, bool), Refusal>, String> {
    if let Some(r) = t.get(K_RSV, &b.id).and_then(|j| serde_json::from_str::<Value>(&j).ok()) {
        return Ok(Ok((r.get("status").and_then(Value::as_str).unwrap_or("").to_string(), false)));
    }
    if b.side == Side::Guest {
        if let Some(pk) = &b.phone_key {
            if open_for_phone(t, pk, now_min) >= MAX_OPEN_PER_PHONE {
                return Ok(Err((
                    429,
                    format!(
                        "this phone already holds {MAX_OPEN_PER_PHONE} bookings here; \
                         cancel one, or call the venue"
                    ),
                )));
            }
        }
    }
    if let Some((z, n)) = &b.table {
        if let Some(why) = table_verdict(plan, &held_tables(t), z, *n, b.party, b.slot_min, &b.id) {
            return Ok(Err((409, why)));
        }
    }
    let status = match b.side {
        Side::Guest => ReservationStatus::Requested,
        Side::Venue => ReservationStatus::Confirmed,
    };
    let rec = json!({
        "id": b.id, "location_id": b.venue, "party": b.party,
        "slot_min": b.slot_min, "occasion": b.occasion,
        "contact_name": b.name, "contact_phone": b.phone, "phone_key": b.phone_key,
        "status": status.as_str(), "created_at_ms": b.now_ms, "user_id": b.user_id,
        "zone_id": b.table.as_ref().map(|(z, _)| z.clone()),
        "table_n": b.table.as_ref().map(|(_, n)| *n),
    });
    let index = index_of(&b.id, &rec);
    let unique: Vec<String> =
        b.table.iter().map(|(z, n)| table_key(z, *n, b.slot_min)).collect();
    let u: Vec<&str> = unique.iter().map(String::as_str).collect();
    t.put(K_RSV, &b.id, &rec.to_string(), &index, &u).map_err(|x| format!("booking: {x}"))?;
    let mut chain = vec![ReservationStatus::Requested];
    if status != ReservationStatus::Requested {
        chain.push(status);
    }
    for (i, s) in chain.iter().enumerate() {
        let ev = json!({
            "to_status": s.as_str(), "seq": i as i64 + 1, "actor": b.side.actor(),
            "reason": "", "at_ms": b.now_ms,
        });
        t.put(K_EV, &ev_key(&b.id, i as i64 + 1), &ev.to_string(), &[], &[])
            .map_err(|x| format!("booking event: {x}"))?;
    }
    Ok(Ok((status.as_str().to_string(), true)))
}

/// Move a booking, or refuse. The legality is the kernel's, checked against
/// the FOLDED history read inside this turn -- never the cached column, and
/// never a fold taken before the write, which two concurrent moves could both
/// have passed. Answers the new event's `seq`.
pub(super) fn write_transition(
    t: &mut Table,
    id: &str,
    to: ReservationStatus,
    side: Side,
    by: &str,
    reason: &str,
    now_ms: i64,
) -> Result<Result<i64, Refusal>, String> {
    let Some(mut r) = t.get(K_RSV, id).and_then(|j| serde_json::from_str::<Value>(&j).ok()) else {
        return Ok(Err((404, "not found".into())));
    };
    let events = events_of(t, id);
    let current = match fold_status(&events) {
        Ok(s) => s,
        Err(why) => return Ok(Err((500, format!("reservation unreadable: {why}")))),
    };
    if let Err(e) = assert_transition(current, to) {
        // 409, not 400: well formed, and the booking is not where it can happen.
        return Ok(Err((409, e.message())));
    }
    let seq = events.iter().map(|e| e.seq).max().unwrap_or(0) + 1;
    let ev = json!({
        "to_status": to.as_str(), "seq": seq, "actor": side.actor(), "by": by,
        "reason": reason, "at_ms": now_ms,
    });
    t.put(K_EV, &ev_key(id, seq), &ev.to_string(), &[], &[])
        .map_err(|x| format!("booking event: {x}"))?;
    r["status"] = json!(to.as_str());
    r["updated_at_ms"] = json!(now_ms);
    let index = index_of(id, &r);
    t.put(K_RSV, id, &r.to_string(), &index, &[]).map_err(|x| format!("booking: {x}"))?;
    Ok(Ok(seq))
}

/// The venue's bookings with a slot in `[from_min, to_min)`, earliest first,
/// each with its FOLDED status and the moves the venue may make from it -- the
/// console's buttons, so no status chain is restated in the browser.
pub(super) fn day_rows(t: &Table, from_min: i64, to_min: i64) -> Vec<Value> {
    let mut rows: Vec<Value> = t
        .all(K_RSV)
        .into_iter()
        .filter_map(|(_, j)| serde_json::from_str::<Value>(&j).ok())
        .filter(|r| {
            let s = r.get("slot_min").and_then(Value::as_i64).unwrap_or(i64::MIN);
            s >= from_min && s < to_min
        })
        .map(|r| {
            let id = r.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let folded = fold_status(&events_of(t, &id));
            let s = |k: &str| r.get(k).cloned().unwrap_or(Value::Null);
            json!({
                "id": id, "slotMin": s("slot_min"), "party": s("party"),
                "name": s("contact_name"), "phone": s("contact_phone"),
                "occasion": s("occasion"), "zoneId": s("zone_id"), "tableN": s("table_n"),
                "createdAtMs": s("created_at_ms"),
                "status": folded.as_ref().map(|x| x.as_str().to_string())
                    .unwrap_or_else(|why| format!("UNREADABLE: {why}")),
                "next": folded.map(|x| moves(Side::Venue, x).iter().map(|m| m.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default(),
            })
        })
        .collect();
    rows.sort_by_key(|r| r.get("slotMin").and_then(Value::as_i64).unwrap_or(0));
    rows
}

#[cfg(test)]
mod tests;
