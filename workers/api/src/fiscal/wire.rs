//! PURE. Where the fiscal seam meets the running platform: whether a venue is
//! fiscalising, what a placement queues, and what `health.fiscal` says.
//!
//! CONFIGURED MEANS ONE DECLARED KEY, `fiscal.since_ms` (`settings/known.rs`):
//! the instant from which every order that takes money owes a document. Empty
//! is NOT CONFIGURED -- nothing is queued, and conservation law N reports
//! "n/a", which is neither a pass nor a fail. A value that is not an integer is
//! also not configured, but it is said out loud (`health.fiscal.error`), because
//! an owner who typed a date and got silence would believe they were covered.
//!
//! THE QUEUE IS ITS OWN IMAGE, `fiscal`, holding ordinary `outbox::Entry`s of
//! kind `"fiscal"`. Not the shared `outbox` image, deliberately: with the
//! production sender (`NotConfigured`) nothing ever leaves this queue, and a
//! backlog that shared an image with the kitchen's messages would one day fill
//! it and refuse a ticket. A full fiscal image refuses a document, loudly, and
//! law N names the order; the kitchen never notices.
//!
//! NOTHING HERE SENDS. See `sender.rs`: the HARD LIMIT stands.

use serde_json::{json, Value};

use super::document::{document, Refusal};
use super::queue::{entry, health};
use crate::outbox::Entry;

/// The one declared key.
pub const SETTING: &str = "fiscal.since_ms";
/// The venue's fiscal queue image, and its ceiling.
pub const IMAGE: &str = "fiscal";
pub const CEILING: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Config {
    /// The key is empty: fiscalisation is not configured.
    Off,
    /// Every order that takes money at or after this instant owes a document.
    From(i64),
    /// The key holds something that is not an instant. Not configured, and loud.
    Invalid(String),
}

pub fn config(raw: &str) -> Config {
    let v = raw.trim();
    if v.is_empty() {
        return Config::Off;
    }
    match v.parse::<i64>() {
        Ok(ms) if ms >= 0 => Config::From(ms),
        _ => Config::Invalid(format!(
            "{SETTING} is {v:?}: it must be epoch milliseconds, e.g. 1790000000000"
        )),
    }
}

/// What a placement came to, fiscally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtPlacement {
    /// A document was issued; this entry goes into the `fiscal` image.
    Queued(Entry),
    /// The venue does not fiscalise. Nothing is queued, and that is not a failure.
    NotConfigured,
    /// The order is older than `fiscal.since_ms`.
    Before,
    /// Not owed a document at all: it came from the platform (`external.fic`),
    /// its price is untrusted, or it took no money (TAX §3.7 rules 2-4).
    NotOwed,
    /// The order owes a document and none could be built (e.g. `NoTax`). The
    /// caller says so loudly; law N names the order.
    Refused(Refusal),
}

/// The rule, for one order the object has just written.
///
/// "Placed" is read from the order's own `created_at_ms` -- the field law N
/// reads -- so the hook and the law can never disagree about which side of
/// `since` an order is on.
pub fn at_placement(cfg: &Config, order: &Value, venue_currency: &str, now_ms: i64) -> AtPlacement {
    let Config::From(since) = cfg else { return AtPlacement::NotConfigured };
    let placed = order.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now_ms);
    if placed < *since {
        return AtPlacement::Before;
    }
    match document(order, venue_currency, now_ms) {
        Ok(doc) => AtPlacement::Queued(entry(&doc)),
        // Those three are not owed a document at all (TAX §3.7 rules 2-4), and
        // law N skips them by the same fields; they are not refusals to shout.
        Err(Refusal::AlreadyFiscalised { .. } | Refusal::Untrusted | Refusal::NotTaken) => {
            AtPlacement::NotOwed
        }
        Err(r) => AtPlacement::Refused(r),
    }
}

/// `health.fiscal`. `entries` is the `fiscal` image's content, or why it could
/// not be read -- an unreadable queue is never answered as an empty one.
pub fn health_json(cfg: &Config, entries: Result<&[Entry], String>, now_ms: i64) -> Value {
    match cfg {
        Config::Off => json!({
            "configured": false,
            "said": format!("{SETTING} is empty: fiscalisation is not configured, nothing is queued"),
        }),
        Config::Invalid(why) => json!({ "configured": false, "error": why }),
        Config::From(since) => match entries {
            Err(e) => json!({ "configured": true, "since_ms": since, "error": e }),
            Ok(es) => {
                let h = health(es, now_ms);
                json!({
                    "configured": true,
                    "since_ms": since,
                    // HARD LIMIT: the only production sender refuses every send.
                    "sender": "not_configured",
                    "backlog": h.backlog,
                    "oldest_issued_at": h.oldest_issued_at,
                    "first_deadline": h.first_deadline,
                    "waiting": h.waiting,
                    "overdue": h.overdue,
                })
            }
        },
    }
}

#[cfg(test)]
mod tests;
