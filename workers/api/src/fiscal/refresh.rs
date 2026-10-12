//! PURE. A QUEUED DOCUMENT FOLLOWS ITS ORDER UNTIL IT IS SENT (D34, R-GAPS C12).
//!
//! The document is queued at placement (`hubdo/fiscal.rs`), and the order can
//! still change after that: a tip added at pay time, a comp, a discount. A
//! document sent as it was queued would then not be the sale -- its total is
//! not what the guest paid, and `ebills_body` refuses it by `Total`.
//!
//! SO THE SENDER REBUILDS IT FROM THE ORDER AS IT IS NOW, right before it
//! claims it (`hubdo/fiscal/send.rs`, `fiscal_plan`), under three rules:
//!   * ONE DOCUMENT PER ORDER. The entry is rewritten in place: its id (the
//!     document's uuid, derived from venue + order), its issue instant and so
//!     its 48 h deadline, its tries -- only `text` changes. A rebuild whose
//!     uuid differs is not taken.
//!   * NEVER A DOCUMENT THE PLATFORM MAY HOLD. Only an entry no firing ever
//!     POSTed is rewritten: no intent row, or a `Blocked` one (the body was
//!     refused here, nothing left) with no unanswered send and no sale id. A
//!     `Sending` row may be an invoice whose answer was lost, and the
//!     reconcile finds it by its marker AND its total; anything later is the
//!     platform's. A sent document is not in the queue at all; correcting it
//!     is the credit-note path (`document::corrective`), never a rewrite.
//!   * AN ORDER THAT NO LONGER OWES A DOCUMENT (refunded, unreadable) leaves
//!     the entry as it is: the cancel path and law N own that case.

use serde_json::Value;

use super::document::{document, Document};
use super::ebills_sender::{Intent, Stage};
use super::queue::KIND;
use crate::hubdo::OrderView;
use crate::outbox::Entry;

/// May the entry with this intent row still be rewritten?
pub fn rewritable(intent: Option<&Intent>) -> bool {
    match intent {
        None => true,
        Some(i) => i.stage == Stage::Blocked && i.unknowns == 0 && i.sale_id.is_none(),
    }
}

/// The entry rebuilt from `order` when it may be and something changed.
pub fn refreshed(e: &Entry, intent: Option<&Intent>, order: &Value, currency: &str) -> Option<Entry> {
    if !rewritable(intent) {
        return None;
    }
    let was: Document = serde_json::from_str(&e.text).ok()?;
    // Issued once: the rebuild keeps the instant the 48 h run from.
    let now = document(order, currency, was.issued_at_ms).ok()?;
    if now.uuid != was.uuid || now == was {
        return None;
    }
    Some(Entry { text: serde_json::to_string(&now).ok()?, ..e.clone() })
}

/// Every queued entry whose order moved, rebuilt; the rest are not returned.
pub fn refresh_all(entries: &[Entry], intents: &[Intent], orders: &[OrderView], currency: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    for e in entries.iter().filter(|e| e.kind == KIND) {
        let Some(o) = orders.iter().find(|o| o.order_id == e.to) else { continue };
        let Ok(order) = serde_json::from_str::<Value>(&o.order_json) else { continue };
        let intent = intents.iter().find(|i| i.uuid == e.id);
        out.extend(refreshed(e, intent, &order, currency));
    }
    out
}

#[cfg(test)]
mod tests;
