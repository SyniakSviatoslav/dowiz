//! THE eBills SENDER'S BOOKKEEPING, PURE (card L70; EBILLS-WRITE-PATH §3, §7).
//! The network half is `ebills_fire.rs`; the object that runs these is
//! `hubdo/fiscal_send.rs`. Three object turns and one Worker step per firing:
//!
//!   1. PLAN (object): the due documents, ≤ `BATCH`, and an INTENT row for each
//!      one that may be POSTed -- written to the `fiscal` image BEFORE the
//!      Worker is handed the batch. A Worker that dies mid-send leaves a
//!      visible "sending, answer unknown" row, never a silent retry (§3 (a)).
//!   2. FIRE (Worker): reconcile, build, POST, read the answer.
//!   3. ANSWER (object): the outcomes replayed through `FiscalSender` into the
//!      queue's own `drain` (non-head-blocking), the intents settled, and a
//!      `Noted{fiscal}` on each order the tax authority registered.
//!
//! AN UNANSWERED SEND IS RECONCILED BEFORE ANY RETRY (§3 (c)): its intent
//! stays `Sending`, and the next firing reads the allow-listed sale list for
//! the order's `notes` marker before it POSTs again. Two unanswered sends
//! stop the document (`Inconclusive`) for the owner: a timeout must never
//! turn into two invoices.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::document::{uuid_text, Document};
use super::queue::KIND;
use super::sender::{Codes, FiscalSender, SendResult};
use crate::command::amend::next_seq;
use crate::command::Refused;
use crate::hubdo::OrderView;
use crate::outbox::{due, Entry};

/// The intent rows' record kind in the venue's `fiscal` image.
pub const INTENT: &str = "fiscal.intent";
/// Documents a firing may take (card: "≤ N per firing").
pub const BATCH: usize = 5;
/// How long a blocked document (an unmapped dish) or an unfiscalised sale
/// waits before it is looked at again.
pub const LOOK_AGAIN_MS: i64 = 15 * 60_000;
/// Unanswered sends before a document is stopped for the owner.
pub const MAX_UNKNOWNS: u32 = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Handed to a firing: a POST may be in flight or its answer lost.
    Sending,
    /// The sale exists at the platform, NOT fiscalised: re-read, never re-sent.
    Unfiscalised,
    /// The platform refused the create: nothing exists; never re-sent by itself.
    Refused,
    /// Not sent, for a cause the owner can fix (a dish with no till item).
    Blocked,
    /// The owner must look at the till: an answer nothing here could settle.
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Intent {
    pub uuid: String,
    pub order_id: String,
    pub stage: Stage,
    pub at_ms: i64,
    #[serde(default)]
    pub unknowns: u32,
    #[serde(default)]
    pub sale_id: Option<i64>,
    #[serde(default)]
    pub why: Option<String>,
    /// The stage before this firing claimed it: restored when the firing
    /// never reached the POST (`Outcome::NotSent`).
    #[serde(default)]
    pub prev: Option<Stage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Send,
    /// An earlier send's answer is unknown: find it in the list first.
    ReconcileThenSend,
    /// Read this sale by id: has the platform fiscalised it since?
    Recheck(i64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Item {
    pub uuid: String,
    pub order_id: String,
    pub doc: String,
    pub action: Action,
}

/// What one document came to in one firing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Sent { sale_id: i64, codes: Codes },
    Unfiscalised { sale_id: i64, fault: String },
    Refused(String),
    Blocked(String),
    /// A POST went and its answer did not come back.
    Unknown(String),
    Inconclusive(String),
    /// This firing never reached the POST for it.
    NotSent(String),
}

/// THE PLAN: which due entries this firing takes, and the intent rows to
/// write before it does. Oldest first; a held document is passed over, so
/// the ones behind it are not blocked.
pub fn plan(entries: &[Entry], intents: &[Intent], now_ms: i64) -> (Vec<Item>, Vec<Intent>) {
    let fiscal: Vec<Entry> = entries.iter().filter(|e| e.kind == KIND).cloned().collect();
    let (mut items, mut writes) = (Vec::new(), Vec::new());
    for e in due(&fiscal, now_ms) {
        if items.len() >= BATCH {
            break;
        }
        let was = intents.iter().find(|i| i.uuid == e.id);
        let waited = was.map_or(i64::MAX, |i| now_ms - i.at_ms) >= LOOK_AGAIN_MS;
        let action = match was.map(|i| (i.stage, i.sale_id)) {
            None => Action::Send,
            Some((Stage::Sending, _)) => Action::ReconcileThenSend,
            Some((Stage::Blocked, _)) if waited => Action::Send,
            Some((Stage::Unfiscalised, Some(id))) if waited => Action::Recheck(id),
            _ => continue,
        };
        if !matches!(action, Action::Recheck(_)) {
            let base = was.cloned().unwrap_or(Intent {
                uuid: e.id.clone(), order_id: e.to.clone(), stage: Stage::Sending, at_ms: now_ms,
                unknowns: 0, sale_id: None, why: None, prev: None,
            });
            writes.push(Intent { stage: Stage::Sending, at_ms: now_ms, prev: was.map(|i| i.stage), ..base });
        }
        items.push(Item { uuid: e.id.clone(), order_id: e.to.clone(), doc: e.text.clone(), action });
    }
    (items, writes)
}

/// An intent row to write, or one to remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentWrite {
    Put(Intent),
    Remove(String),
}

/// The intents after a firing's outcomes.
pub fn settle(intents: &[Intent], outcomes: &[(String, Outcome)], now_ms: i64) -> Vec<IntentWrite> {
    let mut out = Vec::new();
    for (uuid, o) in outcomes {
        let Some(was) = intents.iter().find(|i| &i.uuid == uuid).cloned() else { continue };
        let put = |stage: Stage, why: &str| IntentWrite::Put(Intent { stage, at_ms: now_ms, why: Some(why.to_string()), prev: None, ..was.clone() });
        out.push(match o {
            Outcome::Sent { .. } => IntentWrite::Remove(uuid.clone()),
            Outcome::Unfiscalised { sale_id, fault } => {
                IntentWrite::Put(Intent { stage: Stage::Unfiscalised, at_ms: now_ms, sale_id: Some(*sale_id), why: Some(fault.clone()), prev: None, ..was.clone() })
            }
            Outcome::Refused(why) => put(Stage::Refused, why),
            Outcome::Blocked(why) => put(Stage::Blocked, why),
            Outcome::Inconclusive(why) => put(Stage::Inconclusive, why),
            Outcome::Unknown(why) => {
                let unknowns = was.unknowns + 1;
                let stage = if unknowns >= MAX_UNKNOWNS { Stage::Inconclusive } else { Stage::Sending };
                IntentWrite::Put(Intent { stage, unknowns, at_ms: now_ms, why: Some(why.clone()), prev: None, ..was.clone() })
            }
            Outcome::NotSent(why) => match was.prev {
                None if was.stage == Stage::Sending && was.unknowns == 0 && was.sale_id.is_none() => IntentWrite::Remove(uuid.clone()),
                prev => IntentWrite::Put(Intent { stage: prev.unwrap_or(was.stage), why: Some(why.clone()), prev: None, ..was.clone() }),
            },
        });
    }
    out
}

/// A FIRING'S ANSWERS, REPLAYED: the one `FiscalSender` that talks to
/// ebills.al, as the queue's `drain` sees it. Its answers were gathered by
/// `ebills_fire` (async, over the network); a document the firing did not
/// take answers `NotConfigured` and is left exactly as it was.
pub struct EbillsSender(pub Vec<(String, Outcome)>);

impl FiscalSender for EbillsSender {
    fn send(&self, doc: &Document) -> SendResult {
        let uuid = uuid_text(&doc.uuid);
        match self.0.iter().find(|(u, _)| *u == uuid).map(|(_, o)| o) {
            Some(Outcome::Sent { codes, .. }) => SendResult::Sent(codes.clone()),
            Some(Outcome::Unfiscalised { sale_id, fault }) => SendResult::Held(format!("ebills sale {sale_id} exists, not fiscalised: {fault}")),
            Some(Outcome::Refused(w)) => SendResult::Held(format!("ebills refused the sale: {w}")),
            Some(Outcome::Blocked(w)) => SendResult::Held(format!("not sent: {w}")),
            Some(Outcome::Inconclusive(w)) => SendResult::Held(format!("stopped for the owner: {w}")),
            Some(Outcome::Unknown(w)) => SendResult::Retriable(w.clone()),
            Some(Outcome::NotSent(_)) | None => SendResult::NotConfigured,
        }
    }
}

/// `Noted{fiscal}` on the order the tax authority registered -- the field
/// law N reads (`order.fiscal.fic`). An order that already carries a `fic`
/// writes nothing. Pure over the log in memory, as `kitchen_ack::decide`.
pub fn note(hub: &mut dowiz_hub::Hub, current: Option<&OrderView>, sale_id: i64, codes: &Codes, now_ms: i64) -> Result<Option<(String, u64)>, Refused> {
    let Some(current) = current else { return Err(Refused::NotFound) };
    let old: Value = serde_json::from_str(&current.order_json).map_err(|e| Refused::Append(format!("order json unreadable: {e}")))?;
    if old.pointer("/fiscal/fic").is_some_and(|f| !f.is_null()) {
        return Ok(None);
    }
    let mut order = old.clone();
    order["fiscal"] = json!({
        "iic": codes.iic, "fic": codes.fic, "inv_ord_num": codes.inv_ord_num,
        "sale_id": sale_id, "at": now_ms, "by": "ebills",
    });
    let body = crate::fold::delta(&old, &order).to_string();
    let seq = next_seq(current.seq, now_ms);
    hub.append(dowiz_hub::EventKind::Noted, &current.order_id, &body, seq, [0u8; 32])
        .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    Ok(Some((body, seq)))
}

#[cfg(test)]
pub(crate) mod tests;
