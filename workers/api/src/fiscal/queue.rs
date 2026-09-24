//! PURE. The fiscal outbox (BLIND-SPOTS §2.8): every `Document` waits as an
//! ordinary `outbox::Entry` of kind `"fiscal"` — REUSED, not forked — and owes
//! the tax authority its registration within 48 hours of issue (Law 87/2019
//! art. 29 p.2, the stricter reading: per invoice, from issue).
//!
//! THE ENTRY: `id` is the document's uuid (so enqueuing twice is one entry and
//! every retry re-sends the same key), `to` is the order id, `text` is the
//! document as JSON rendered at issue time, and `queued_at_ms` IS the issue
//! instant — the deadline is derived from it, never stored beside it.
//!
//! THE DRAIN DOES NOT HEAD-BLOCK. Documents go in issue order; a document the
//! platform refuses is `Verdict::Abandon` + an exception row + the corrective
//! path, and the ones behind it are still sent in the same pass.
//!
//! A FISCAL ENTRY IS NEVER ABANDONED FOR COUNTING ITS TRIES. `outbox::MAX_TRIES`
//! is a kitchen message's lifetime (half an hour); an invoice is owed until it
//! is registered. It keeps retrying at the longest backoff, and when its 48 h
//! pass it is OVERDUE — named by `health` and by conservation law N — not gone.

use serde::Serialize;

use super::document::{corrective, uuid_text, Document};
use super::sender::{Codes, FiscalSender, SendResult};
use crate::exceptions::fold::Row;
use crate::outbox::{after_attempt, backoff_ms, due, Entry, Verdict};

pub const KIND: &str = "fiscal";
/// Art. 29 p.2: 48 hours, per invoice, from issue.
pub const DEADLINE_MS: i64 = 48 * 3600 * 1000;

/// The outbox entry for a freshly issued document.
pub fn entry(doc: &Document) -> Entry {
    let text = serde_json::to_string(doc).unwrap_or_default();
    Entry::new(uuid_text(&doc.uuid), KIND, doc.order_id.clone(), text, doc.issued_at_ms)
}

fn deadline_of(e: &Entry) -> i64 {
    e.queued_at_ms.saturating_add(DEADLINE_MS)
}

/// What one pass of the drain came to. `verdicts` has the shape
/// `outbox::rails` already applies to the image, one per attempted entry.
#[derive(Debug, Default)]
pub struct Drained {
    pub verdicts: Vec<(String, Verdict)>,
    /// (order id, codes): each becomes a `Noted{fiscal}` on its order.
    pub sent: Vec<(String, Codes)>,
    /// One per refused or unreadable document — the owner's exception report.
    pub exceptions: Vec<Row>,
    /// The art. 32 document for each refusal, to be issued in its turn.
    pub corrective: Vec<Document>,
    /// Entries left untouched because no sender is configured.
    pub held: usize,
}

/// One pass over the fiscal entries that are due, oldest issue first.
pub fn drain(entries: &[Entry], sender: &dyn FiscalSender, now_ms: i64) -> Drained {
    let mut out = Drained::default();
    let fiscal: Vec<Entry> = entries.iter().filter(|e| e.kind == KIND).cloned().collect();
    for e in due(&fiscal, now_ms) {
        let Ok(doc) = serde_json::from_str::<Document>(&e.text) else {
            out.exceptions.push(row(e, now_ms, "the queued document does not parse".into()));
            out.verdicts.push((e.id.clone(), Verdict::Abandon { after: e.tries + 1 }));
            continue;
        };
        let verdict = match sender.send(&doc) {
            SendResult::Sent(codes) => {
                out.sent.push((doc.order_id.clone(), codes));
                after_attempt(e, true, now_ms)
            }
            SendResult::Retriable(_) => match after_attempt(e, false, now_ms) {
                Verdict::Abandon { after } => {
                    Verdict::Retry { tries: after, next_at_ms: now_ms + backoff_ms(after) }
                }
                v => v,
            },
            SendResult::Refused(problem) => {
                out.exceptions.push(row(e, now_ms, problem));
                out.corrective.push(corrective(&doc, now_ms));
                Verdict::Abandon { after: e.tries + 1 }
            }
            SendResult::NotConfigured => {
                out.held += 1;
                continue;
            }
            // Stays queued, untouched, and SAID: never resent automatically.
            SendResult::Held(why) => {
                out.exceptions.push(Row { kind: "fiscal.held", ..row(e, now_ms, why) });
                out.held += 1;
                continue;
            }
        };
        out.verdicts.push((e.id.clone(), verdict));
    }
    out
}

fn row(e: &Entry, now_ms: i64, reason: String) -> Row {
    Row {
        at: now_ms,
        kind: "fiscal.refused",
        order_id: Some(e.to.clone()),
        till_id: None,
        reason: Some(reason),
        amount: 0,
        currency: None,
        by: "fiscal".into(),
        tx_id: None,
    }
}

/// One waiting document as the health pane and law N read it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Waiting {
    pub order_id: String,
    pub uuid: String,
    pub issued_at: i64,
    pub deadline: i64,
}

/// `health.fiscal` (BLIND-SPOTS §2.8). `waiting` lists every entry so law N
/// can name the order; `overdue` is the subset whose 48 h have passed.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Health {
    pub backlog: usize,
    pub oldest_issued_at: Option<i64>,
    pub first_deadline: Option<i64>,
    pub waiting: Vec<Waiting>,
    pub overdue: Vec<Waiting>,
}

/// The fiscal entries' health at `now_ms`. Other kinds in the image are not counted.
pub fn health(entries: &[Entry], now_ms: i64) -> Health {
    let mut waiting: Vec<Waiting> = entries
        .iter()
        .filter(|e| e.kind == KIND)
        .map(|e| Waiting {
            order_id: e.to.clone(),
            uuid: e.id.clone(),
            issued_at: e.queued_at_ms,
            deadline: deadline_of(e),
        })
        .collect();
    waiting.sort_by_key(|w| (w.issued_at, w.uuid.clone()));
    // A deadline AT `now` has passed: law N asks for one in the future.
    let overdue = waiting.iter().filter(|w| w.deadline <= now_ms).cloned().collect();
    Health {
        backlog: waiting.len(),
        oldest_issued_at: waiting.first().map(|w| w.issued_at),
        first_deadline: waiting.iter().map(|w| w.deadline).min(),
        waiting,
        overdue,
    }
}

#[cfg(test)]
mod tests;
