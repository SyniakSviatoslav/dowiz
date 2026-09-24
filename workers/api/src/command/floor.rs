//! FLOOR STATES AS FOLDS, plus one mark (BLUEPRINT-OPERATIONAL-BLIND-SPOTS
//! §2.7, P3-1): where each table on the owner's plan stands RIGHT NOW.
//!
//! THREE SOURCES, NO NEW ONE. The geometry is `dowiz_hub::tables`' plan (what
//! the owner saves through `booking::set_plan`); "booked" is that module's own
//! `holder` over the booking holds; everything else is `command::sitting`'s
//! fold over the order log. The only write is the `cleared` mark, a `Noted`
//! fact on the sitting's LAST round (`mark`) — no new event kind, no image.
//!
//! PURE. No I/O and no clock: `now_ms` is a parameter.

use serde_json::{json, Value};

use super::sitting::{self, Round};
use super::Refused;
use crate::hubdo::OrderView;
use dowiz_hub::tables::{self as plan, Held, Plan};

/// The field the mark writes on the last round: `{"cleared": {by, at}}`.
pub const CLEARED: &str = "cleared";

/// One table's state. Every table is in exactly one (see [`state_of`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloorState {
    Free,
    Booked,
    Ordering,
    Waiting,
    Paying,
    Dirty,
}

impl FloorState {
    pub const ALL: [FloorState; 6] = [
        FloorState::Free, FloorState::Booked, FloorState::Ordering,
        FloorState::Waiting, FloorState::Paying, FloorState::Dirty,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            FloorState::Free => "free",
            FloorState::Booked => "booked",
            FloorState::Ordering => "ordering",
            FloorState::Waiting => "waiting",
            FloorState::Paying => "paying",
            FloorState::Dirty => "dirty",
        }
    }
}

/// What one sitting's rounds say, reduced to what the floor reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seated {
    pub sitting_id: String,
    /// The table text the newest round carries (`fulfilment.table`).
    pub table: String,
    /// Status of the newest round that took money (`sitting::Round::billed`).
    pub newest: String,
    /// When that newest round was placed: which sitting at a table is current.
    pub since: i64,
    pub bill: i64,
    pub paid: i64,
    /// The latest `at` of any payment on a billed round; 0 when none.
    pub last_paid_at: i64,
    /// The LAST round (oldest-first order), which is where the mark goes.
    pub last_round: String,
    /// `at` of the `cleared` mark on the last round, if it carries one.
    pub cleared_at: Option<i64>,
}

/// The facts of one sitting. `None` when no round took money (every round
/// rejected or cancelled): such a sitting holds no table.
pub fn seated(sitting_id: &str, rounds: &[Round<'_>]) -> Option<Seated> {
    let newest = rounds.iter().rev().find(|r| r.billed())?;
    let last = rounds.last()?;
    let last_paid_at = rounds
        .iter()
        .filter(|r| r.billed())
        .flat_map(|r| r.order.get("payments").and_then(Value::as_array).cloned().unwrap_or_default())
        .filter_map(|p| p.get("at").and_then(Value::as_i64))
        .max()
        .unwrap_or(0);
    Some(Seated {
        sitting_id: sitting_id.to_string(),
        table: rounds
            .iter()
            .rev()
            .find_map(|r| r.order.pointer("/fulfilment/table").and_then(Value::as_str))
            .unwrap_or("")
            .trim()
            .to_string(),
        newest: newest.status().to_string(),
        since: newest.int("created_at_ms"),
        bill: sitting::bill(rounds),
        paid: sitting::paid(rounds),
        last_paid_at,
        last_round: last.view.order_id.clone(),
        cleared_at: last.order.pointer("/cleared/at").and_then(Value::as_i64),
    })
}

/// Does a booking hold this table at this minute? `tables::holder` with the
/// same `DWELL_MIN` the booking write refuses a collision with: a table a
/// walk-in could not be given now is a booked table.
pub fn booked_at(held: &[Held], zone: &str, n: i64, now_ms: i64) -> bool {
    plan::holder(held, zone, n, now_ms.div_euclid(60_000), plan::DWELL_MIN).is_some()
}

/// THE STATE OF ONE TABLE. Total and disjoint by construction: one `match`,
/// every arm returns, the last arm catches everything.
///
/// PRECEDENCE, top wins:
/// 1. The current sitting's newest billed round is PENDING, CONFIRMED or
///    SCHEDULED → `Ordering`.
/// 2. … is PREPARING → `Waiting`.
/// 3. … is anything later (READY and on, a refund included) and
///    Σ paid < bill → `Paying`.
/// 4. … Σ paid ≥ bill and no `cleared` mark at or after the last payment →
///    `Dirty`.
/// 5. Otherwise (no sitting, or a cleared one): a booking holds it → `Booked`.
/// 6. → `Free`.
///
/// A SITTING OUTRANKS A BOOKING: people at the table are what a waiter must
/// see, and a dirty table with a 20:00 booking still needs clearing first.
pub fn state_of(booked: bool, seat: Option<&Seated>) -> FloorState {
    match seat {
        Some(s) if matches!(s.newest.as_str(), "PENDING" | "CONFIRMED" | "SCHEDULED") => FloorState::Ordering,
        Some(s) if s.newest == "PREPARING" => FloorState::Waiting,
        Some(s) if s.paid < s.bill => FloorState::Paying,
        Some(s) if !s.cleared_at.is_some_and(|c| c >= s.last_paid_at) => FloorState::Dirty,
        _ if booked => FloorState::Booked,
        _ => FloorState::Free,
    }
}

/// Which table on the plan a sitting's free-text table names: `zone:n`,
/// `zone/n`, or a bare `n` when exactly one zone has a table `n`.
pub fn table_of(p: &Plan, text: &str) -> Option<(String, i64)> {
    let text = text.trim();
    if let Some((z, n)) = text.split_once([':', '/']) {
        let n: i64 = n.trim().parse().ok()?;
        return p.find(z.trim(), n).map(|_| (z.trim().to_string(), n));
    }
    let n: i64 = text.parse().ok()?;
    let mut hits = p.zones.iter().filter(|z| z.tables.iter().any(|t| t.n == n));
    match (hits.next(), hits.next()) {
        (Some(z), None) => Some((z.id.clone(), n)),
        _ => None,
    }
}

/// Every sitting in this venue's listed orders, with facts. `listed` is the
/// object's `orders_view`, already filtered to the venue.
pub fn sittings(listed: &[OrderView]) -> Vec<Seated> {
    let mut ids: Vec<String> = Vec::new();
    for v in listed {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        if let Some(s) = o.get("sitting_id").and_then(Value::as_str) {
            if !ids.iter().any(|x| x == s) {
                ids.push(s.to_string());
            }
        }
    }
    ids.iter().filter_map(|s| seated(s, &sitting::rounds(listed, s))).collect()
}

/// THE FLOOR: the plan, every table with its state, and the sittings whose
/// table text names no table on the plan (`unplaced`) — listed, never dropped.
pub fn floor(p: &Plan, held: &[Held], listed: &[OrderView], now_ms: i64) -> Value {
    // THE CURRENT SITTING PER TABLE is the one with the newest round; an older
    // sitting at the same table is history. Off-plan tables are keyed by
    // their text, so they too show one sitting each.
    let all = sittings(listed);
    let mut current: Vec<(Result<(String, i64), String>, &Seated)> = Vec::new();
    for s in &all {
        let k = table_of(p, &s.table).ok_or_else(|| s.table.clone());
        match current.iter_mut().find(|(c, _)| *c == k) {
            Some(slot) if slot.1.since < s.since => slot.1 = s,
            Some(_) => {}
            None => current.push((k, s)),
        }
    }
    let unplaced: Vec<Value> = current
        .iter()
        .filter(|(k, s)| k.is_err() && state_of(false, Some(s)) != FloorState::Free)
        .map(|(_, s)| json!({ "table": s.table, "sitting_id": s.sitting_id, "state": state_of(false, Some(s)).as_str() }))
        .collect();
    let zones: Vec<Value> = p.zones.iter().map(|z| json!({
        "id": z.id, "name": z.name,
        "tables": z.tables.iter().map(|t| {
            let seat = current.iter().find(|(k, _)| k.as_ref().is_ok_and(|k| k.0 == z.id && k.1 == t.n)).map(|(_, s)| *s);
            let st = state_of(booked_at(held, &z.id, t.n, now_ms), seat);
            json!({
                "n": t.n, "x": t.x, "y": t.y, "w": t.w, "h": t.h, "seats": t.seats,
                "shape": t.shape.as_str(), "state": st.as_str(),
                "sitting_id": seat.filter(|_| st != FloorState::Free && st != FloorState::Booked).map(|s| s.sitting_id.clone()),
            })
        }).collect::<Vec<_>>(),
    })).collect();
    json!({ "planW": plan::PLAN_W, "planH": plan::PLAN_H, "zones": zones, "unplaced": unplaced })
}

/// One held table, read back out of a booking index key
/// (`booking::table_key`: `rsv.tbl/{zone}/{n:06}/{slot:012}`). The test pins
/// this to that function so the two cannot drift.
pub fn held_of(key: &str, reservation: String) -> Option<Held> {
    let mut p = key.split('/');
    if p.next()? != "rsv.tbl" {
        return None;
    }
    let (zone, n, slot) = (p.next()?, p.next()?, p.next()?);
    Some(Held { zone: zone.to_string(), n: n.parse().ok()?, slot_min: slot.parse().ok()?, reservation })
}

/// WHICH ROUND TAKES THE MARK, and whether it needs one. Refused unless the
/// sitting is `Dirty` now: a table still eating or paying is not cleared.
/// `Ok((round, false))` = already cleared since the last payment; nothing to write.
pub fn clear_target(listed: &[OrderView], sitting_id: &str) -> Result<(String, bool), Refused> {
    let rounds = sitting::rounds(listed, sitting_id);
    let Some(s) = seated(sitting_id, &rounds) else { return Err(Refused::NotFound) };
    match state_of(false, Some(&s)) {
        FloorState::Dirty => Ok((s.last_round, true)),
        FloorState::Free => Ok((s.last_round, false)),
        other => Err(Refused::Conflict(format!("this table is {}, not waiting to be cleared", other.as_str()))),
    }
}

/// The mark itself: the order as it is after `{cleared: {by, at}}`. The
/// caller writes `fold::delta(old, new)` as ONE `Noted` event.
pub fn mark(old: &Value, by: &str, at: i64) -> Result<Value, Refused> {
    if by.trim().is_empty() {
        return Err(Refused::Invalid("a cleared mark must name who cleared".into()));
    }
    let mut o = old.clone();
    o[CLEARED] = json!({ "by": by, "at": at });
    Ok(o)
}

#[cfg(test)]
mod tests;
