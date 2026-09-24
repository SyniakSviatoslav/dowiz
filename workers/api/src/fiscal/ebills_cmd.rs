//! WHAT CROSSES BETWEEN THE CRON AND THE VENUE'S OBJECT for the eBills
//! sender (`/fold/ebills/fiscal_*`), and the owner's view -- pure, so the
//! pane's every word is tested natively.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::ebills_arm::{Arming, CancelEntry, CONFIRM};
use super::ebills_fire::Batch;
use super::ebills_sender::{Intent, Outcome};
use super::queue::health;
use crate::ebills::client::Session;
use crate::outbox::Entry;

/// The sender's own record in the `fiscal` image: how its last firing went.
pub const K_STATE: &str = "fiscal.sender";
pub const ONE: &str = "venue";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanIn {
    pub now_ms: i64,
}

/// The object's answer to `fiscal_plan`: the batch and the link's
/// credentials when the venue is armed; otherwise every reason it is not.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanOut {
    pub ready: bool,
    pub why: Vec<String>,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub secret: String,
    #[serde(default)]
    pub session: Option<Session>,
    #[serde(default)]
    pub batch: Batch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerIn {
    pub now_ms: i64,
    pub outcomes: Vec<(String, Outcome)>,
    /// The link's session, when this firing changed it.
    #[serde(default)]
    pub session: Option<Session>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SenderState {
    pub last_ok_ms: i64,
    pub last_error: Option<(i64, String)>,
    pub sent: u64,
}

impl SenderState {
    /// A firing that sent nothing because it could not (every outcome
    /// `NotSent`) is an error; one that reached the till is a success.
    pub fn after(&mut self, outcomes: &[(String, Outcome)], now_ms: i64) {
        let sent = outcomes.iter().filter(|(_, o)| matches!(o, Outcome::Sent { .. })).count() as u64;
        self.sent += sent;
        match outcomes.iter().find_map(|(_, o)| match o {
            Outcome::NotSent(w) => Some(w.clone()),
            _ => None,
        }) {
            Some(why) if sent == 0 => self.last_error = Some((now_ms, why)),
            _ if !outcomes.is_empty() => (self.last_ok_ms, self.last_error) = (now_ms, None),
            _ => {}
        }
    }
}

/// The owner's fiscal pane. `floor` is the tables the till's floor lists
/// (the sale-unit picker), `items` the till codes seen (the fee picker).
#[allow(clippy::too_many_arguments)]
pub fn view(
    ready: &[String],
    a: &Arming,
    since_ms: Option<i64>,
    entries: &[Entry],
    intents: &[Intent],
    cancels: &[CancelEntry],
    st: &SenderState,
    floor: Vec<String>,
    items: Vec<(String, String)>,
    now_ms: i64,
) -> Value {
    let h = health(entries, now_ms);
    let stage = |uuid: &str| intents.iter().find(|i| i.uuid == uuid);
    let waiting: Vec<Value> = h
        .waiting
        .iter()
        .map(|w| {
            let i = stage(&w.uuid);
            json!({
                "order_id": w.order_id, "uuid": w.uuid, "issued_at": w.issued_at, "deadline": w.deadline,
                "overdue": w.deadline <= now_ms, "stage": i.map_or(json!("queued"), |i| json!(i.stage)),
                "why": i.and_then(|i| i.why.clone()), "sale_id": i.and_then(|i| i.sale_id),
            })
        })
        .collect();
    json!({
        "armed": ready.is_empty(), "why": ready, "arming": a, "confirm": CONFIRM,
        "marker": "dowiz:<order id>", "since_ms": since_ms,
        "backlog": h.backlog, "overdue": h.overdue.len(), "first_deadline": h.first_deadline,
        "waiting": waiting, "cancels": cancels,
        "cancel_note": "cancel sending is not built: the allow-list has no cancel path and the shape is unverified",
        "last_ok_ms": st.last_ok_ms, "last_error": st.last_error, "sent": st.sent,
        "floor": floor, "items": items.into_iter().map(|(code, name)| json!({ "code": code, "name": name })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests;
