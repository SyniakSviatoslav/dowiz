//! WALLET-LEG FINDINGS AS EXCEPTION ROWS, and their alert on FIRST sight.
//!
//! Conservation law 12 (`command::pay::legs`) says every wallet `Paid` has its
//! ledger leg and every spend its `Paid`. Its audit was pull-only: an owner
//! had to open `/api/owner/wallet/legs` to learn a guest ate on a wallet that
//! was never charged. Here each finding becomes a row of the exception report:
//!   * `wallet_leg_missing`    — a `Paid` whose leg is not in the ledger, and
//!     the repair WOULD write it;
//!   * `wallet_leg_refused`    — a missing leg the repair refuses (the wallet
//!     no longer covers it); `reason` is the refusal;
//!   * `wallet_leg_mismatched` — a leg whose wallet, amount or currency is not
//!     its `Paid`'s;
//!   * `wallet_leg_orphan`     — a spend naming no wallet `Paid`.
//!
//! THE RULES ARE NOT RE-IMPLEMENTED: the rows are laid out from
//! `legs::audit` and `legs::repair`, the same functions the audit route calls.
//! A row's `by` is the signer of the `Paid` (or of the spend) — a name on a
//! row, never a total; money integrity is a VENUE fact.
//!
//! NOT A THRESHOLD. A lost leg is money that did not move; the first one is
//! the one to hear about. [`first`] alerts on every finding the venue has not
//! been told about yet, and names the markers to keep so it is told ONCE.

use super::alert::{text, Voice};
use super::fold::Row;
use crate::command::pay::legs;
use crate::outbox::Entry;
use serde_json::Value;

pub const LEG_MISSING: &str = "wallet_leg_missing";
pub const LEG_REFUSED: &str = "wallet_leg_refused";
pub const LEG_MISMATCHED: &str = "wallet_leg_mismatched";
pub const LEG_ORPHAN: &str = "wallet_leg_orphan";
/// The integrity kinds, in the order the alert lists them.
pub const LEG_KINDS: [&str; 4] = [LEG_MISSING, LEG_REFUSED, LEG_MISMATCHED, LEG_ORPHAN];

/// The outbox record kind that remembers "this finding was alerted". The
/// drain reads only `outbox::KIND`, so a marker is never delivered and
/// outlives the message it stands for (the delivered entry is removed).
pub const MARK_KIND: &str = "exceptions.alerted";

fn signer(orders: &[Value], leg: &legs::Leg) -> String {
    orders
        .iter()
        .filter(|o| o.get("id").and_then(Value::as_str) == Some(leg.order_id.as_str()))
        .flat_map(|o| o.get("payments").and_then(Value::as_array).into_iter().flatten())
        .find(|p| p.get("method").and_then(Value::as_str) == Some("wallet") && p.get("at").and_then(Value::as_i64) == Some(leg.at))
        .and_then(|p| p.get("by").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string()
}

fn leg_row(kind: &'static str, leg: &legs::Leg, reason: Option<String>, by: String) -> Row {
    Row {
        at: leg.at, kind, order_id: Some(leg.order_id.clone()), till_id: None, reason, amount: leg.amount,
        currency: Some(leg.currency.clone()), by, tx_id: Some(leg.tx_id.clone()),
    }
}

/// Every wallet-leg finding for one venue. `ledger` is the ledger's records
/// OLDEST FIRST (what `legs::repair` needs). A ledger that does not replay is
/// an error, never "no findings".
pub fn leg_rows(orders: &[Value], ledger: &[String], venue: &str, venue_currency: &str) -> Result<Vec<Row>, String> {
    let audit = legs::audit(orders, ledger, venue, venue_currency);
    if audit.holds() {
        return Ok(Vec::new());
    }
    let plan = legs::repair(orders, ledger, venue, venue_currency).map_err(|r| r.message().to_string())?;
    let mut out = Vec::new();
    for leg in &audit.missing {
        let refused = plan.refused.iter().find(|(l, _)| l.tx_id == leg.tx_id).map(|(_, why)| why.clone());
        let kind = if refused.is_some() { LEG_REFUSED } else { LEG_MISSING };
        out.push(leg_row(kind, leg, refused, signer(orders, leg)));
    }
    let named = |found: &[String], id: &str| found.iter().any(|f| f.starts_with(&format!("{id}: ")));
    for leg in legs::expected(orders, venue, venue_currency).iter().filter(|l| named(&audit.mismatched, &l.tx_id)) {
        out.push(leg_row(LEG_MISMATCHED, leg, None, signer(orders, leg)));
    }
    for rec in ledger.iter().filter_map(|j| serde_json::from_str::<Value>(j).ok()).filter(legs::is_spend) {
        let id = rec.get("id").and_then(Value::as_str).unwrap_or_default();
        if !named(&audit.orphans, id) {
            continue;
        }
        // The spend's size: its credited posting (the venue's side).
        let credit = rec.get("postings").and_then(Value::as_array).into_iter().flatten()
            .filter(|p| p.get("minor").and_then(Value::as_i64).unwrap_or(0) > 0)
            .max_by_key(|p| p.get("minor").and_then(Value::as_i64).unwrap_or(0));
        out.push(Row {
            at: rec.get("at_ms").and_then(Value::as_i64).unwrap_or(0), kind: LEG_ORPHAN, order_id: None, till_id: None,
            reason: rec.get("memo").and_then(Value::as_str).filter(|m| !m.is_empty()).map(str::to_string),
            amount: credit.and_then(|p| p.get("minor").and_then(Value::as_i64)).unwrap_or(0),
            currency: credit.and_then(|p| p.get("currency").and_then(Value::as_str)).map(str::to_string),
            by: rec.get("by").and_then(Value::as_str).unwrap_or_default().to_string(),
            tx_id: Some(id.to_string()),
        });
    }
    Ok(out)
}

/// The folded orders as JSON, each carrying its `id` (a folded body may not
/// repeat it; `legs::expected` keys every leg by it).
pub fn orders_json(listed: &[crate::hubdo::OrderView]) -> Vec<Value> {
    listed
        .iter()
        .filter_map(|o| {
            let mut v: Value = serde_json::from_str(&o.order_json).ok()?;
            if v.get("id").is_none() {
                v["id"] = Value::String(o.order_id.clone());
            }
            Some(v)
        })
        .collect()
}

/// A finding's marker id: the kind and the leg's (derived, stable) tx id.
pub fn mark_of(r: &Row) -> String {
    format!("exceptions/first/{}/{}", r.kind, r.tx_id.as_deref().unwrap_or("-"))
}

/// The alerts owed on FIRST sight: one message per integrity kind listing the
/// findings not yet marked, and the markers to write beside it. `marked`
/// answers whether a marker id is already in the outbox. No threshold: a
/// chat is the only condition.
pub fn first(rows: &[Row], marked: &dyn Fn(&str) -> bool, now_ms: i64, venue: &Voice, chat: &str) -> (Vec<Entry>, Vec<String>) {
    let (mut entries, mut marks) = (Vec::new(), Vec::new());
    if chat.trim().is_empty() {
        return (entries, marks);
    }
    for kind in LEG_KINDS {
        let new: Vec<&Row> = rows.iter().filter(|r| r.kind == kind && !marked(&mark_of(r))).collect();
        let Some(since) = new.iter().map(|r| r.at).min() else { continue };
        let ids: Vec<String> = new.iter().map(|r| mark_of(r)).collect();
        entries.push(crate::notify::route::alert_entry(ids[0].clone(), chat.trim(), &|l| text(&venue.speaking(l), kind, &new, since), now_ms));
        marks.extend(ids);
    }
    (entries, marks)
}

#[cfg(test)]
mod tests;
