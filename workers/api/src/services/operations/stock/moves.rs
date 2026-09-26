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
use dowiz_hub::stock::{PrepStage, StockError, StockEvent, StockLog, WasteReason};
use serde::Deserialize;
use serde_json::{json, Value};

/// One counted line of a session.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CountLineIn {
    pub item: String,
    pub observed: i64,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StockMoveIn {
    #[serde(default)]
    pub item: String,
    #[serde(default)]
    pub qty: Option<i64>,
    /// For a stocktake: what was actually counted.
    #[serde(default)]
    pub observed: Option<i64>,
    /// For waste: one of `WasteReason::allowed_words()`. Required.
    #[serde(default)]
    pub reason: Option<String>,
    // NO `by`. THE SIGNER IS WHO AUTHENTICATED, never a field the caller
    // fills in: a body that could name its signer is a write-off anybody can
    // put on somebody else. `deny_unknown_fields` turns a `by` into a 400.
    /// A delivery's price: minor units per `per` base units ...
    #[serde(default)]
    pub unit_cost: Option<i64>,
    #[serde(default)]
    pub per: Option<i64>,
    /// ... or the invoice line's total for the whole `qty`.
    #[serde(default)]
    pub total: Option<i64>,
    #[serde(default)]
    pub supplier: Option<String>,
    /// The invoice / delivery note number.
    #[serde(default)]
    pub doc: Option<String>,
    #[serde(default)]
    pub lot: Option<String>,
    /// `yyyy-mm-dd`, the label's date.
    #[serde(default)]
    pub expiry: Option<String>,
    /// A session's lines.
    #[serde(default)]
    pub lines: Option<Vec<CountLineIn>>,
    #[serde(default)]
    pub session: Option<String>,
    /// Prep: what came off the board, and after which stage.
    #[serde(default)]
    pub out: Option<i64>,
    #[serde(default)]
    pub stage: Option<String>,
    /// Prep into ANOTHER stocked supply (whole fish -> fillet).
    #[serde(default)]
    pub into: Option<String>,
    /// `as-is`: the dishes to link to their own piece (`as_is`).
    #[serde(default)]
    pub products: Option<Vec<String>>,
}

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
        "received" => body.qty.map(|qty| StockEvent::Received { item, qty }).ok_or((400, "how much?".into())),
        "wasted" => {
            let qty = body.qty.ok_or((400, "how much?".to_string()))?;
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
            .map(|observed| StockEvent::Stocktake { item, observed, stocktake_id: format!("st_{now_ms}"), by })
            .ok_or((400, "what was counted?".into())),
        "produced" => {
            let qty = body.qty.ok_or((400, "how much went onto the board?".to_string()))?;
            let out = body.out.ok_or((400, "how much came off it?".to_string()))?;
            let s = body.stage.as_deref().map(str::trim).unwrap_or("clean");
            let stage = PrepStage::from_str(s).ok_or((400, format!("{s:?} is not a stage: clean or cook")))?;
            let into = body.into.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
            Ok(StockEvent::Produced { item, qty, out, stage, into, by })
        }
        other => Err((400, format!("no such movement: {other}"))),
    }
}

/// The paper's price as `(unit_cost, per)`: the total for the whole quantity,
/// or a cost per basis. Half a price is refused, not guessed.
fn price(body: &StockMoveIn, qty: i64) -> Result<(Option<i64>, Option<i64>), Bad> {
    match (body.total, body.unit_cost, body.per) {
        (Some(t), None, None) if t >= 0 => Ok((Some(t), Some(qty))),
        (None, Some(c), Some(p)) if c >= 0 && p > 0 => Ok((Some(c), Some(p))),
        (None, None, None) => Ok((None, None)),
        _ => Err((400, "a price is the line's total, or a cost with what it is per -- not negative".into())),
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
    let base = Meta { at: Some(now_ms), lot: text(&body.lot)?, ..Meta::default() };
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
                if item.is_empty() || l.observed < 0 {
                    return Err((400, format!("{item:?}: a count is a supply and a number from 0")));
                }
                if out.iter().any(|(e, _): &(StockEvent, Meta)| e.item() == item) {
                    return Err((400, format!("{item} is counted twice in one session")));
                }
                let ev = StockEvent::Stocktake { item, observed: l.observed, stocktake_id: session.clone(), by: by.into() };
                out.push((ev, Meta { session: Some(session.clone()), ..base.clone() }));
            }
            out
        }
        "received" => {
            let (unit_cost, per) = price(&body, body.qty.unwrap_or(0))?;
            let meta = Meta {
                supplier: text(&body.supplier)?,
                doc: text(&body.doc)?,
                expiry: expiry(&body, today, shelf_days(body.item.trim()))?,
                unit_cost,
                per,
                by: Some(by.to_string()),
                ..base
            };
            vec![(movement(kind, body, by, now_ms)?, meta)]
        }
        "stocktake" => {
            let ev = movement(kind, body, by, now_ms)?;
            let session = match &ev {
                StockEvent::Stocktake { stocktake_id, .. } => stocktake_id.clone(),
                _ => unreachable!("movement answers the kind it was asked"),
            };
            vec![(ev, Meta { session: Some(session), ..base })]
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
        let led = log.ledger()?;
        let book = log.cost_book();
        let mut lines = self.lines.clone();
        let mut shown = Vec::new();
        let mut total: i64 = 0;
        for (ev, meta) in lines.iter_mut() {
            match ev {
                StockEvent::Stocktake { item, observed, .. } => {
                    let expected = led.level(item).on_hand;
                    let drift = *observed - expected;
                    let value = book.value_of(item, drift.abs()).map(|v| if drift < 0 { -v } else { v });
                    total += value.unwrap_or(0);
                    meta.expected = Some(expected);
                    shown.push(json!({ "item": item, "expected": expected, "observed": observed, "drift": drift, "value": value }));
                }
                StockEvent::Wasted { item, qty, .. } => {
                    meta.value = book.value_of(item, *qty);
                    shown.push(json!({ "item": item, "qty": qty, "value": meta.value }));
                }
                StockEvent::Received { item, qty } => {
                    let value = match (meta.unit_cost, meta.per) {
                        (Some(c), Some(p)) => dowiz_hub::stock::journal::priced(*qty, c, p),
                        _ => None,
                    };
                    shown.push(json!({ "item": item, "qty": qty, "value": value }));
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
