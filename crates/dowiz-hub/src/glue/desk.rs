//! SHEAF S2: THE DECISIONS A GLUE LEFT FOR A PERSON ("decision needed", G.5 S2). Each one is a
//! `decide` note in the stock log (`merge.rs`): an obstruction the glue found (the last salmon
//! sold on two tablets, a count against a movement, one hold taken twice, a sale past rights),
//! or a tablet record the hub's ledger refused, kept verbatim. A decision is OPEN until a
//! `decided` note names its id; closing writes that note and nothing else -- the fix itself
//! (an amend, a refund, a count) goes through the doors that already exist.
//!
//! Both notes ride in the log, so the list is tamper-evident like the shelf, a tablet sees it
//! after its next sync, and a log without either note is the log it was.

use super::merge::DECIDE;
use crate::minijson::{esc, int_field, str_field};
use crate::stock::notes::is_note;
use crate::stock::{StockError, StockLog};

/// The note that closes a decision.
pub const DECIDED: &str = "decided";

/// One open decision, as the kitchen board shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decision {
    pub did: String,
    pub ctx: String,
    /// `oversold`, `recount`, `double`, `diverged`, `provisional`, `held`.
    pub kind: String,
    pub item: String,
    pub order: String,
    pub qty: Option<i64>,
    /// `held`: the ledger's reason, and the record itself.
    pub why: String,
    pub rec: String,
    pub at: Option<i64>,
}

fn decision(rec: &str) -> Option<Decision> {
    let s = |k: &str| str_field(rec, k).unwrap_or_default();
    Some(Decision {
        did: str_field(rec, "did").filter(|d| !d.is_empty())?,
        ctx: s("ctx"),
        kind: s("kind"),
        item: s("item"),
        order: s("order"),
        qty: int_field(rec, "qty"),
        why: s("why"),
        rec: s("rec"),
        at: int_field(rec, "at"),
    })
}

/// Every decision not yet closed, OLDEST FIRST.
pub fn open(log: &StockLog) -> Vec<Decision> {
    let mut all = Vec::new();
    let mut closed: Vec<String> = Vec::new();
    for rec in log.notes(DECIDE).iter().chain(log.notes(DECIDED).iter()) {
        if is_note(rec, DECIDED) {
            closed.extend(str_field(rec, "did"));
        } else if let Some(d) = decision(rec) {
            all.push(d);
        }
    }
    all.retain(|d| !closed.contains(&d.did));
    all
}

/// CLOSE decision `did`, signed by `by`. `Ok(false)`: no such open decision, nothing written.
/// Refused, nothing written: no signer.
pub fn close(log: &mut StockLog, did: &str, by: &str) -> Result<bool, StockError> {
    if by.trim().is_empty() {
        return Err(StockError::Unsigned);
    }
    if !open(log).iter().any(|d| d.did == did) {
        return Ok(false);
    }
    log.append_note(DECIDED, &format!(r#"{{"did":"{}","by_":"{}"}}"#, esc(did), esc(by.trim())))?;
    Ok(true)
}

#[cfg(test)]
#[path = "desk/tests.rs"]
mod tests;
