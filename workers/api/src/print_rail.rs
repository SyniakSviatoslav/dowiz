//! THE KITCHEN PRINT RAIL: a printer that PHONES US (BLUEPRINT-LAST-MILE
//! §3.1 steps 1 and 3; OPERATIONAL-BLIND-SPOTS §2.6).
//!
//! THE SHAPE. A ticket is a third outbox kind, `print`, enqueued in the same
//! object turn as `Placed` beside the Telegram entry (`hubdo::enqueue_bell`)
//! when the venue has set `print.kitchen`. The cron never sends it
//! (`outbox/rails.rs`): a CloudPRNT-style printer POLLS `/api/print/poll`,
//! is handed a job token, GETs the text, prints, and DELETEs the job. The
//! DELETE is the delivery, so "printed" is an HTTP request WE receive -- a
//! record by construction, not a bridge's good behaviour.
//!
//! THE FOUR STATES (§3.1 step 3), each a record:
//! * `queued`   -- the entry is in the outbox, no token handed out;
//! * `printing` -- a printer was handed its token less than `LEASE_MS` ago;
//! * `printed`  -- the DELETE arrived with a 2xx code: the entry is removed and
//!                 the order carries `kitchen.printed {at, printer, code}`;
//! * `failed`   -- `outbox::MAX_TRIES` failed codes: the entry is removed, the
//!                 order carries `kitchen.print_failed {at, code, tries}`, and
//!                 the venue's error log says so (the health pane reads it).
//!
//! PRINTED IS NOT SEEN. §2.6 makes "sent · seen · printed" three records, and
//! a ticket on the rail of an empty pass has been printed and seen by nobody.
//! `command::kitchen_ack::unconfirmed` therefore still lists a printed ticket
//! nobody tapped -- and names that it WAS printed, which is the difference
//! between "check the printer" and "check the pass".
//!
//! AUTH (§3.1 (c)): the printer's User Name / Password fields hold a venue API
//! key (`services/identity/keys.rs`). The key resolves the venue; nothing in
//! the request names one, so a printer can only ever see its own venue's jobs.
//!
//! PURE. Everything here is a function of its arguments; the object turn is
//! `hubdo/print.rs` and the routes `services/orders/print.rs`.

use crate::outbox::{after_attempt, Entry, Verdict};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use serde::Serialize;
use serde_json::{json, Value};

/// The outbox kind.
pub const KIND: &str = "print";
/// The setting naming the kitchen's printer. Empty = no printer = no tickets.
pub const SETTING: &str = "print.kitchen";
/// How long a handed-out job is the printer's before it is offered again.
///
/// SIXTY SECONDS: the recommended poll is 5 s (§3.1 "The latency") and a
/// ticket prints in a few; a printer that took a token and did not DELETE in
/// a minute lost power or paper mid-job. Re-offering then risks a duplicate
/// ticket, which a kitchen survives; not re-offering risks none, which is
/// the defect this rail exists for.
pub const LEASE_MS: i64 = 60_000;
/// What the printer is told it can fetch. `text/plain` today (§3.1 (b)).
pub const MEDIA: &str = "text/plain";

/// The ticket for one order, or `None` when the venue has no printer.
pub fn entry_for(printer: &str, order_id: &str, text: &str, now_ms: i64) -> Option<Entry> {
    let printer = printer.trim();
    (!printer.is_empty())
        .then(|| Entry::new(format!("{order_id}/{KIND}"), KIND, printer.to_string(), text.to_string(), now_ms))
}

/// The order a print entry is for.
pub fn order_of(entry_id: &str) -> &str {
    entry_id.strip_suffix(&format!("/{KIND}")).unwrap_or(entry_id)
}

/// A job token is the entry id, URL-safe. Reversible, so the object needs no
/// token table; unguessability is not its job -- the venue key is.
pub fn token_of(entry_id: &str) -> String {
    URL_SAFE_NO_PAD.encode(entry_id)
}

pub fn id_of(token: &str) -> Option<String> {
    let id = String::from_utf8(URL_SAFE_NO_PAD.decode(token).ok()?).ok()?;
    id.ends_with(&format!("/{KIND}")).then_some(id)
}

/// Is this job the printer's right now?
fn leased(e: &Entry, now_ms: i64) -> bool {
    e.handed_ms.is_some_and(|h| now_ms - h < LEASE_MS)
}

/// The job to hand the polling printer: the oldest print entry that is due
/// and not already leased. Oldest first, for the reason `outbox::due` gives.
pub fn next_job(entries: &[Entry], now_ms: i64) -> Option<&Entry> {
    entries
        .iter()
        .filter(|e| e.kind == KIND && e.next_at_ms <= now_ms && !leased(e, now_ms))
        .min_by(|a, b| (a.queued_at_ms, &a.id).cmp(&(b.queued_at_ms, &b.id)))
}

/// CloudPRNT result codes lead with three digits; 2xx is printed.
pub fn printed_ok(code: &str) -> bool {
    code.trim().get(..3).and_then(|d| d.parse::<u16>().ok()).is_some_and(|n| (200..300).contains(&n))
}

/// The verdict on a DELETE: printed, or one more failed attempt.
pub fn after_print(entry: &Entry, code: &str, now_ms: i64) -> Verdict {
    if printed_ok(code) {
        return Verdict::Sent;
    }
    after_attempt(entry, false, now_ms)
}

/// The venue key from the printer's `Authorization`: `Bearer dowiz_...`, or
/// Basic with the key as the password (or as the user name when the password
/// field is empty). Anything that is not a `dowiz_` key is `None`.
pub fn key_of(authorization: &str) -> Option<String> {
    let a = authorization.trim();
    let key = if let Some(t) = a.strip_prefix("Bearer ").or_else(|| a.strip_prefix("bearer ")) {
        t.trim().to_string()
    } else {
        let b = a.strip_prefix("Basic ").or_else(|| a.strip_prefix("basic "))?;
        let pair = String::from_utf8(STANDARD.decode(b.trim()).ok()?).ok()?;
        let (user, pass) = pair.split_once(':').unwrap_or((pair.as_str(), ""));
        if pass.is_empty() { user.to_string() } else { pass.to_string() }
    };
    key.starts_with("dowiz_").then_some(key)
}

/// A ticket's state, for the console (§3.1 step 3). `None` = no ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Queued,
    Printing,
    Printed,
    Failed,
}

pub fn state_of(entry: Option<&Entry>, order: &Value, now_ms: i64) -> Option<State> {
    if order.pointer("/kitchen/printed/at").is_some() {
        return Some(State::Printed);
    }
    match entry {
        Some(e) if leased(e, now_ms) => Some(State::Printing),
        Some(_) => Some(State::Queued),
        None if order.pointer("/kitchen/print_failed/at").is_some() => Some(State::Failed),
        None => None,
    }
}

/// `order` with `kitchen.<key> = value`, and the delta body to append as a
/// `Noted`. Other `kitchen` facts (the "seen") are kept.
pub fn noted(order: &Value, key: &str, value: Value) -> (Value, String) {
    let mut next = order.clone();
    let mut kitchen = next.get("kitchen").cloned().filter(Value::is_object).unwrap_or_else(|| json!({}));
    kitchen[key] = value;
    next["kitchen"] = kitchen;
    let body = crate::fold::delta(order, &next).to_string();
    (next, body)
}

// ── the object's commands, on the wire (`/fold/print/<what>`) ──────────────

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct PollIn {
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct JobIn {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct AckIn {
    pub token: String,
    /// The printer's result code, e.g. "200 OK"; a DELETE with none is "200".
    pub code: String,
    pub now_ms: i64,
}

/// What an ack did. `abandoned` is the one the Worker writes to the error log.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct AckOut {
    pub state: State,
    pub tries: u32,
    pub abandoned: bool,
    /// True when this DELETE repeats one already recorded: nothing was written.
    pub replay: bool,
}

/// What the object must do to the outbox entry after an ack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Keep,
    Put(Entry),
    Remove,
}

/// The whole ack, decided: the `Noted` to append (order id, body, seq), the
/// outbox change, and the answer. `None` = no such job (404).
#[derive(Debug, Clone)]
pub struct AckPlan {
    pub note: Option<(String, String, u64)>,
    pub change: Change,
    pub out: AckOut,
}

/// THE DELETE, pure. `order` is the order as folded, with its seq.
///
/// A REPEATED DELETE IS ONE FACT: the entry is gone and the order already
/// says what happened, so the answer is that record and nothing is written.
/// A `printed` already on the order is never written twice either (the
/// outbox write after the log write can be lost; the retry must not stack).
pub fn decide_ack(entry: Option<&Entry>, order: Option<(&Value, u64)>, input: &AckIn) -> Option<AckPlan> {
    let id = id_of(&input.token)?;
    let oid = order_of(&id).to_string();
    let code = input.code.trim().to_string();
    let now = input.now_ms;
    let Some(e) = entry.filter(|e| e.id == id) else {
        let o = order?.0;
        let state = if o.pointer("/kitchen/printed/at").is_some() {
            State::Printed
        } else if o.pointer("/kitchen/print_failed/at").is_some() {
            State::Failed
        } else {
            return None;
        };
        return Some(AckPlan { note: None, change: Change::Keep, out: AckOut { state, tries: 0, abandoned: false, replay: true } });
    };
    let note = |key: &str, value: Value| {
        order
            .filter(|(o, _)| o.pointer("/kitchen/printed/at").is_none())
            .map(|(o, seq)| (oid.clone(), noted(o, key, value).1, crate::command::amend::next_seq(seq, now)))
    };
    Some(match after_print(e, &code, now) {
        Verdict::Sent => AckPlan {
            note: note("printed", json!({ "at": now, "printer": e.to, "code": code })),
            change: Change::Remove,
            out: AckOut { state: State::Printed, tries: e.tries, abandoned: false, replay: false },
        },
        Verdict::Retry { tries, next_at_ms } => {
            let mut n = e.clone();
            (n.tries, n.next_at_ms, n.handed_ms, n.code) = (tries, next_at_ms, None, Some(code));
            AckPlan { note: None, change: Change::Put(n), out: AckOut { state: State::Queued, tries, abandoned: false, replay: false } }
        }
        Verdict::Abandon { after } => AckPlan {
            note: note("print_failed", json!({ "at": now, "code": code, "tries": after })),
            change: Change::Remove,
            out: AckOut { state: State::Failed, tries: after, abandoned: true, replay: false },
        },
    })
}

/// THE POLL, pure: the job to hand out, already marked as handed.
pub fn decide_poll(entries: &[Entry], now_ms: i64) -> Option<Entry> {
    let mut e = next_job(entries, now_ms)?.clone();
    e.handed_ms = Some(now_ms);
    Some(e)
}

#[cfg(test)]
mod tests;
