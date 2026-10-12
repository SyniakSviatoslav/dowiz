//! WHAT A MOVEMENT REQUEST BECOMES, pure: the body, the signer and the clock
//! in; the events with everything their paper says out (research 2026-09-26
//! R1 R2 R3 R5 R8 R9). No Worker, no image -- `apply` runs against a real
//! `StockLog`, so every refusal is tested on the ledger that makes it.
//!
//! Five kinds, one door (`POST /api/owner/stock/:kind`, no new route):
//!   received  -- a delivery: qty, and optionally its price (per basis, or the
//!                invoice line's total), supplier, invoice, lot, expiry;
//!   wasted    -- a write-off with a reason, optionally its lot; valued at
//!                the average cost of the moment and the value stored;
//!   stocktake -- one supply counted, `expected` stored beside it;
//!   count     -- a STOCKTAKE SESSION: every line one decision, one id;
//!   produced  -- prep: raw onto the board, cleaned or cooked off it.

use dowiz_hub::stock::meta::{day_number, day_of_number, parse_day, Meta};
use crate::recipe::Qty;
use dowiz_hub::prep::MICRO;
use dowiz_hub::stock::{PrepStage, StockError, StockEvent, StockLog, WasteReason};
use serde_json::{json, Value};

/// The request body, split out of this file at the 300-line line (W-STORE).
#[path = "moves/input.rs"]
mod input;
pub use input::StockMoveIn;
/// A quantity read exactly, and its price (W-QTYHUB H3).
#[path = "moves/exact.rs"]
mod exact;
use exact::{price, uq_of};
pub use exact::whole;

/// A movement, decided but not yet applied: `expected` and `value` are the
/// ledger's to fill in, at the moment of the write.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub kind: String,
    pub lines: Vec<(StockEvent, Meta)>,
}

type Bad = (u16, String);
const TEXT_MAX: usize = 80;

fn text(v: &Option<String>) -> Result<Option<String>, Bad> {
    match v.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) if s.chars().count() > TEXT_MAX => Err((400, format!("{s:.20}...: at most {TEXT_MAX} characters"))),
        other => Ok(other.map(str::to_string)),
    }
}

/// The event a single-line movement is, signed by `by` -- the AUTHENTICATED
/// principal. A waste reason outside the closed set, or none, is a 400 naming
/// the allowed words: never a default, or the report says the kitchen spoils
/// everything (§2.1).
pub fn movement(kind: &str, body: StockMoveIn, by: &str, now_ms: i64) -> Result<StockEvent, Bad> {
    let item = body.item.trim().to_string();
    if item.is_empty() {
        return Err((400, "which ingredient?".into()));
    }
    let by = by.to_string();
    match kind {
        // A FRACTION IS BOOKED BY THE HUB (W-QTYHUB H3): the whole `qty` here is
        // the typed one rounded, and `plan` puts the exact `uq` beside it.
        "received" => body.qty.map(|q| StockEvent::Received { item, qty: q.rounded() }).ok_or((400, "how much?".into())),
        "wasted" => {
            let qty = body.qty.ok_or((400, "how much?".to_string()))?.rounded();
            let reason = match body.reason.as_deref().map(str::trim) {
                None | Some("") => {
                    return Err((400, format!("a write-off says why: one of {}", WasteReason::allowed_words())))
                }
                Some(r) => WasteReason::from_str(r).ok_or_else(|| {
                    (400, format!("{r:?} is not a waste reason: one of {}", WasteReason::allowed_words()))
                })?,
            };
            Ok(StockEvent::Wasted { item, qty, reason, by })
        }
        "stocktake" => body
            .observed
            .map(|q| StockEvent::Stocktake { item, observed: q.rounded(), stocktake_id: format!("st_{now_ms}"), by })
            .ok_or((400, "what was counted?".into())),
        "produced" => {
            let qty = whole(body.qty.ok_or((400, "how much went onto the board?".to_string()))?)?;
            let out = body.out.ok_or((400, "how much came off it?".to_string()))?;
            let s = body.stage.as_deref().map(str::trim).unwrap_or("clean");
            let stage = PrepStage::from_str(s).ok_or((400, format!("{s:?} is not a stage: clean or cook")))?;
            let into = body.into.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
            Ok(StockEvent::Produced { item, qty, out, stage, into, by })
        }
        other => Err((400, format!("no such movement: {other}"))),
    }
}

/// `today + shelf_days` when the paper names no date; a named one must be a day.
fn expiry(body: &StockMoveIn, today: i64, shelf_days: Option<i64>) -> Result<Option<i64>, Bad> {
    match body.expiry.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => parse_day(s).map(Some).ok_or((400, format!("{s:?} is not a date (yyyy-mm-dd)"))),
        None => Ok(shelf_days.filter(|d| *d > 0).map(|d| day_of_number(day_number(today) + d))),
    }
}

/// The whole request as a [`Plan`]. `shelf_days` answers a supply's default
/// shelf life; `today` is the venue's local day, `yyyymmdd`.
pub fn plan(
    kind: &str,
    body: StockMoveIn,
    by: &str,
    now_ms: i64,
    today: i64,
    shelf_days: impl Fn(&str) -> Option<i64>,
) -> Result<Plan, Bad> {
    if kind == "removed" {
        return super::removed::plan(&body, by, now_ms);
    }
    let store = text(&body.store)?;
    if let Some(st) = store.as_deref().filter(|st| !dowiz_hub::stock::storages::valid_id(st)) {
        return Err((400, format!("{st:?} is not a storage id")));
    }
    let base = Meta { at: Some(now_ms), lot: text(&body.lot)?, store, ..Meta::default() };
    let lines = match kind {
        "count" => {
            let lines = body.lines.clone().unwrap_or_default();
            if lines.is_empty() {
                return Err((400, "a count has at least one line".into()));
            }
            let session = text(&body.session)?.unwrap_or_else(|| format!("st_{now_ms}"));
            let mut out = Vec::with_capacity(lines.len());
            for l in lines {
                let item = l.item.trim().to_string();
                if item.is_empty() || l.observed < 0i64 {
                    return Err((400, format!("{item:?}: a count is a supply and a number from 0")));
                }
                if out.iter().any(|(e, _): &(StockEvent, Meta)| e.item() == item) {
                    return Err((400, format!("{item} is counted twice in one session")));
                }
                let ev = StockEvent::Stocktake { item, observed: l.observed.rounded(), stocktake_id: session.clone(), by: by.into() };
                out.push((ev, Meta { session: Some(session.clone()), uq: uq_of(Some(l.observed)), ..base.clone() }));
            }
            out
        }
        "received" => {
            let (unit_cost, per) = price(&body, body.qty.unwrap_or_default())?;
            let meta = Meta {
                supplier: text(&body.supplier)?,
                doc: text(&body.doc)?,
                treated: text(&body.treated)?,
                expiry: expiry(&body, today, shelf_days(body.item.trim()))?,
                unit_cost,
                per,
                by: Some(by.to_string()),
                uq: uq_of(body.qty),
                ..base
            };
            vec![(movement(kind, body, by, now_ms)?, meta)]
        }
        "stocktake" => {
            let uq = uq_of(body.observed);
            let ev = movement(kind, body, by, now_ms)?;
            let session = match &ev {
                StockEvent::Stocktake { stocktake_id, .. } => stocktake_id.clone(),
                _ => unreachable!("movement answers the kind it was asked"),
            };
            vec![(ev, Meta { session: Some(session), uq, ..base })]
        }
        "wasted" => {
            let uq = uq_of(body.qty);
            vec![(movement(kind, body, by, now_ms)?, Meta { uq, ..base })]
        }
        "produced" => {
            let exp = expiry(&body, today, None)?;
            vec![(movement(kind, body, by, now_ms)?, Meta { expiry: exp, ..base })]
        }
        _ => vec![(movement(kind, body, by, now_ms)?, base)],
    };
    Ok(Plan { kind: kind.to_string(), lines })
}

impl Plan {
    /// Every supply the plan names (the input and a prep's output).
    pub fn items(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for (ev, _) in &self.lines {
            v.push(ev.item().to_string());
            if let StockEvent::Produced { into: Some(t), .. } = ev {
                v.push(t.clone());
            }
        }
        v
    }

    /// Write the plan AS ONE DECISION, filling in what only the ledger knows:
    /// a count's `expected` (the shelf at the write) and a write-off's value
    /// (at the average then). Answers the drift per line for the screen.
    pub fn apply(&self, log: &mut StockLog) -> Result<Value, StockError> {
        if self.kind == "removed" {
            return super::removed::apply(self, log);
        }
        // THE EXACT SHELF (W-QTYHUB H3): the booked level less the carried
        // remainder, so a count of 0.75 compares with 0.75, not with 1.
        let (led, carry) = log.shelf_exact()?;
        let book = log.cost_book();
        let mut lines = self.lines.clone();
        // P12: a named storage is one this venue has, open; a count IN a
        // storage is that storage's level, so the shelf's total is told the
        // counted storage plus every other one (`storages::Stores`).
        let rooms = super::storages::Rooms::of(log, &lines)?;
        for (ev, meta) in lines.iter_mut() {
            if let (StockEvent::Stocktake { item, observed, .. }, Some(st)) = (ev, meta.store.as_deref()) {
                let total = rooms.total_for(item, st, *observed);
                // The other storages' share is whole; the counted one keeps its fraction.
                meta.uq = meta.uq.map(|u| u.saturating_add((total - *observed).saturating_mul(MICRO)));
                *observed = total;
            }
        }
        let shown_qty = |uq: Option<i64>, whole: i64| uq.map_or(json!(whole), |u| Qty::of_micro(u).json());
        let mut shown = Vec::new();
        let mut total: i64 = 0;
        for (ev, meta) in lines.iter_mut() {
            match ev {
                StockEvent::Stocktake { item, observed, .. } => {
                    let expected = led.level(item).on_hand;
                    // In millionths: what the shelf holds exactly, what was counted exactly.
                    let (exp_u, obs_u) = (carry.exact(item, expected), meta.uq.unwrap_or(observed.saturating_mul(MICRO)));
                    let drift = obs_u - exp_u;
                    let value = book.value_of_micro(item, drift.abs()).map(|v| if drift < 0 { -v } else { v });
                    total += value.unwrap_or(0);
                    meta.expected = Some(expected);
                    // A fractional count stores its own value: the journal's whole-unit drift would round it.
                    if meta.uq.is_some_and(|u| u % MICRO != 0) {
                        meta.value = value;
                    }
                    let d = Qty::of_micro(drift);
                    shown.push(json!({ "item": item, "expected": Qty::of_micro(exp_u).json(), "observed": Qty::of_micro(obs_u).json(),
                        "drift": d.json(), "driftMilli": d.milli, "value": value }));
                }
                StockEvent::Wasted { item, qty, .. } => {
                    meta.value = match meta.uq {
                        Some(u) => book.value_of_micro(item, u),
                        None => book.value_of(item, *qty),
                    };
                    shown.push(json!({ "item": item, "qty": shown_qty(meta.uq, *qty), "value": meta.value }));
                }
                StockEvent::Received { item, qty } => {
                    // Priced on what was TYPED: 2.5 pieces at 300 is 750, not 3 x 300.
                    let value = match (meta.unit_cost, meta.per, meta.uq) {
                        (Some(c), Some(p), Some(u)) if p > 0 => {
                            let n = i128::from(u) * i128::from(c);
                            let d = i128::from(p) * i128::from(MICRO);
                            i64::try_from((n + d / 2) / d).ok()
                        }
                        (Some(c), Some(p), _) => dowiz_hub::stock::journal::priced(*qty, c, p),
                        _ => None,
                    };
                    shown.push(json!({ "item": item, "qty": shown_qty(meta.uq, *qty), "value": value }));
                }
                StockEvent::Produced { item, qty, out, .. } => {
                    let pm = if *qty > 0 { *out * 1000 / *qty } else { 0 };
                    shown.push(json!({ "item": item, "qty": qty, "out": out, "yieldPm": pm }));
                }
                _ => {}
            }
        }
        match self.kind.as_str() {
            "received" => {
                let (ev, meta) = &lines[0];
                if let StockEvent::Received { item, qty } = ev {
                    log.receive_with(item, *qty, meta)?;
                }
            }
            _ => log.append_all_with(&lines)?,
        }
        Ok(json!({ "ok": true, "kind": self.kind, "lines": shown, "value": total }))
    }
}

#[cfg(test)]
#[path = "moves/tests.rs"]
mod tests;
