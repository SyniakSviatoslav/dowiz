//! SHEAF S2b: THE KITCHEN'S ORDER RAIL, OFFLINE, THROUGH THE SAME GLUE (R-BEBOP-LEVERS §3.3 item
//! 2, §3.4; operator 2026-10-27 Q3). S2 made the stock sheet work with the uplink down; this makes
//! the two taps a pass lives on -- "seen" (the kitchen ack) and the bump (Advance) -- work too.
//!
//! WHAT THE TABLET HOLDS IS NOT THE ORDER LOG. The order log carries guests' own words and
//! addresses; a shared kitchen device holds none of it (AX7). The tablet keeps a RAIL -- per open
//! ticket its id, status and whether it was seen, exactly what `GET /api/staff/kitchen` already
//! shows it -- and a QUEUE of the taps it took while offline, each one OP LINE (`Op::line`). The
//! queue is the tail; it travels up as G.2's Z-set with weights +1, and the hub is the one writer.
//!
//! THE WRITE DOOR ON THE TABLET is the order machine itself (`dowiz_core::order_machine`, the FSM
//! the kernel's `apply_event_logic` runs): `Rail::record` refuses an illegal edge by the kernel's
//! own code and words before anything is queued, and moves the ticket on screen at once.
//!
//! THE REBASE ON THE HUB (`rebase`), op by op in the order the tablet took them:
//!   ack      -- already seen: DONE (one fact, `command::kitchen_ack`); else APPLY.
//!   advance  -- the order's history already reached `next`: DONE. The FSM is a DAG
//!               (`order_machine::topological_order`), so "reached `next`" is monotone and the
//!               same tail sent twice is adopted once -- no note, no key, no clock;
//!               else the FSM must allow `current -> next` and `current` must be what the tablet
//!               saw (`from`), or the op is HELD by name; else APPLY.
//! APPLY is the caller's: the Worker passes the venue object's own deciders (`command::advance`,
//! `command::kitchen_ack`), so an offline tap is decided by exactly the code an online tap is, and
//! their refusal is HELD with their words. Nothing here writes an order.
//!
//! NOTHING THE TABLET SENT IS LOST (`Outcome::law`, S2's law): `sent == merged + held`. A held op
//! becomes a `decide` note in the stock log (`hold_notes`), so it lands on the "decision needed"
//! strip the kitchen already has (`glue::desk`), once: its id is the tablet's op id.

use dowiz_core::order_machine::{assert_transition, OrderStatus};
use serde_json::{json, Value};

use super::merge::DECIDE;
use crate::minijson::{esc, str_field};
use crate::room::delta::fold_one;
use crate::stock::{StockError, StockLog};
use crate::Hub;

/// NO-TRACKING: the only keys an op line may carry. No guest, no staff profile, no device id.
pub const KEYS: [&str; 6] = ["op", "oid", "order", "from", "next", "at"];
/// The longest op line, and the most ops one sync carries (a busy day is ~150 taps).
pub const REC_MAX: usize = 400;
pub const TAIL_MAX: usize = 400;
/// The edges a pass may take offline: the four bumps (`feed.js` `BUMPS`). Reject and cancel stay
/// online: they can owe a guest money back (`command::advance` D4).
pub const NEXT: [&str; 4] = ["CONFIRMED", "PREPARING", "READY", "PICKED_UP"];
/// A tablet's clock is not trusted (G.2: never LWW by clock): an ack's `at` is clamped to the
/// last day before the hub's own now.
pub const MAX_OFFLINE_MS: i64 = 86_400_000;

/// What the tap was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Ack,
    Advance { from: String, next: String },
}

/// One tap taken on the tablet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    /// The tablet's own id for the tap: `<at>.<n>`.
    pub oid: String,
    pub order: String,
    pub kind: Kind,
    /// The tablet's clock (ms) when it was taken.
    pub at: i64,
}

/// Why an op line was refused as a whole tail (the hub writes nothing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bad {
    /// Not an op this codec reads (`pos` counts from the OLDEST).
    NotAnOp { pos: usize },
    /// NO-TRACKING: a key no op carries.
    Tracking { pos: usize, key: String },
    /// A line longer than `REC_MAX`, or a tail longer than `TAIL_MAX`.
    TooLong { pos: usize },
}

fn plain(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

impl Op {
    /// The line as it travels and as it is kept in a held note.
    pub fn line(&self) -> String {
        match &self.kind {
            Kind::Ack => json!({"op": "ack", "oid": self.oid, "order": self.order, "at": self.at}),
            Kind::Advance { from, next } => json!({"op": "advance", "oid": self.oid, "order": self.order, "from": from, "next": next, "at": self.at}),
        }
        .to_string()
    }

    /// Read one line; `pos` is only for the refusal.
    pub fn parse(rec: &str, pos: usize) -> Result<Op, Bad> {
        if rec.len() > REC_MAX {
            return Err(Bad::TooLong { pos });
        }
        let not = Bad::NotAnOp { pos };
        let v: Value = serde_json::from_str(rec).map_err(|_| not.clone())?;
        let o = v.as_object().ok_or(not.clone())?;
        if let Some(k) = o.keys().find(|k| !KEYS.contains(&k.as_str())) {
            return Err(Bad::Tracking { pos, key: k.clone() });
        }
        let s = |k: &str| o.get(k).and_then(Value::as_str);
        let (Some(oid), Some(order), Some(at)) = (s("oid"), s("order"), o.get("at").and_then(Value::as_i64)) else { return Err(not) };
        if !plain(oid, 40) || !plain(order, 64) || at < 0 {
            return Err(not);
        }
        let kind = match (s("op"), s("from"), s("next")) {
            (Some("ack"), None, None) => Kind::Ack,
            (Some("advance"), Some(from), Some(next)) if OrderStatus::from_str(from).is_some() && NEXT.contains(&next) => Kind::Advance { from: from.into(), next: next.into() },
            _ => return Err(not),
        };
        Ok(Op { oid: oid.into(), order: order.into(), kind, at })
    }

    /// What the decision strip names it by: the order's first six characters and the tap.
    fn label(&self) -> String {
        let short: String = self.order.chars().take(6).collect();
        match &self.kind {
            Kind::Ack => format!("{short} seen"),
            Kind::Advance { next, .. } => format!("{short} {next}"),
        }
    }
}

/// A whole tail, checked before one op is believed. Refused whole: nothing is half-adopted.
pub fn read_tail(recs: &[String]) -> Result<Vec<Op>, Bad> {
    if recs.len() > TAIL_MAX {
        return Err(Bad::TooLong { pos: TAIL_MAX });
    }
    recs.iter().enumerate().map(|(i, r)| Op::parse(r, i)).collect()
}

/// What the hub does with one op.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// Its effect is in the log already: merged, nothing written.
    Done,
    /// The caller's decider writes it.
    Apply,
    /// Not applied: why, for a person.
    Hold(String),
}

/// The order folded from the log, and every status it has held, OLDEST FIRST (the projection's
/// rules: order events only, this id only).
pub fn history(hub: &Hub, order: &str) -> Option<(Value, Vec<String>)> {
    let (mut state, mut seen) = (Value::Null, Vec::new());
    for e in hub.events_oldest_first() {
        if !e.kind.is_order() || e.order_id != order {
            continue;
        }
        state = fold_one(state, &e.order_json);
        if let Some(s) = state.get("status").and_then(Value::as_str) {
            if seen.last().map(String::as_str) != Some(s) {
                seen.push(s.to_string());
            }
        }
    }
    (!state.is_null()).then_some((state, seen))
}

/// THE HUB'S PLAN for one op at venue `loc` (another venue's order is not found, never forbidden).
pub fn plan(hub: &Hub, loc: &str, op: &Op) -> Plan {
    let Some((o, statuses)) = history(hub, &op.order) else { return Plan::Hold("order not found".into()) };
    if o.get("location_id").and_then(Value::as_str) != Some(loc) {
        return Plan::Hold("order not found".into());
    }
    match &op.kind {
        Kind::Ack if o.pointer("/kitchen/seen/at").is_some() => Plan::Done,
        Kind::Ack => Plan::Apply,
        Kind::Advance { from, next } => {
            if statuses.iter().any(|s| s == next) {
                return Plan::Done;
            }
            let cur = o.get("status").and_then(Value::as_str).unwrap_or_default();
            let (Some(a), Some(b)) = (OrderStatus::from_str(cur), OrderStatus::from_str(next)) else { return Plan::Hold(format!("unknown status {cur}")) };
            if let Err(e) = assert_transition(a, b) {
                return Plan::Hold(format!("{}: {}", e.code(), e.message()));
            }
            if cur != from {
                return Plan::Hold(format!("stale: the order is {cur}, the tablet saw {from}"));
            }
            Plan::Apply
        }
    }
}

/// What one sync did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Ops whose effect is in the log now (applied, or there already).
    pub merged: usize,
    /// Of those, the ones this sync wrote.
    pub applied: usize,
    /// `(position in the tail, why)`.
    pub held: Vec<(usize, String)>,
}

impl Outcome {
    /// THE MERGE LAW: every op the tablet sent is merged or held.
    pub fn law(&self, sent: usize) -> bool {
        sent == self.merged + self.held.len()
    }
}

/// REBASE the tablet's ops onto the hub's log, in the order they were taken. `apply` is the
/// decider that writes one op (the venue object's own); its `Err` is held with its words.
pub fn rebase<F>(hub: &mut Hub, loc: &str, ops: &[Op], mut apply: F) -> Outcome
where
    F: FnMut(&mut Hub, &Op) -> Result<(), String>,
{
    let mut out = Outcome::default();
    for (pos, op) in ops.iter().enumerate() {
        match plan(hub, loc, op) {
            Plan::Done => out.merged += 1,
            Plan::Hold(why) => out.held.push((pos, why)),
            Plan::Apply => match apply(hub, op) {
                Ok(()) => {
                    out.merged += 1;
                    out.applied += 1;
                }
                Err(why) => out.held.push((pos, why)),
            },
        }
    }
    out
}

/// The decision id of a held op: the tablet's name and its op id.
pub fn did_of(ctx: &str, op: &Op) -> String {
    format!("o-{ctx}-{}", op.oid)
}

/// WRITE EACH HELD OP as a `decide` note (kind `held`) in the stock log, where the kitchen's
/// decision strip reads it -- once: a did already noted (open or closed) is not written again, so
/// a retried tail adds nothing. Answers how many notes were written.
pub fn hold_notes(stock: &mut StockLog, ctx: &str, ops: &[Op], held: &[(usize, String)]) -> Result<usize, StockError> {
    let have: Vec<String> = stock.notes(DECIDE).iter().filter_map(|r| str_field(r, "did")).collect();
    let mut n = 0;
    for (pos, why) in held {
        let Some(op) = ops.get(*pos) else { continue };
        let did = did_of(ctx, op);
        if have.contains(&did) {
            continue;
        }
        let body = format!(
            r#"{{"did":"{}","ctx":"{}","kind":"held","item":"{}","order":"{}","why":"{}","rec":"{}"}}"#,
            esc(&did),
            esc(ctx),
            esc(&op.label()),
            esc(&op.order),
            esc(why),
            esc(&op.line())
        );
        stock.append_note(DECIDE, &body)?;
        n += 1;
    }
    Ok(n)
}

/// The tablet's side: the rail and the queue.
#[path = "orders/rail.rs"]
pub mod rail;
pub use rail::{Local, Rail, Ticket};

#[cfg(test)]
#[path = "orders/tests.rs"]
mod tests;
