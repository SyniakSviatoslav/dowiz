//! WHAT AN ORDER TURN OWES THE PHONES, PURE (the producer).
//!
//! The venue's object calls this in the turn that wrote the event, with the
//! subscriptions it holds, and writes the answer into the outbox in the same
//! turn (`hubdo/push_turn.rs`). Nobody subscribed = nothing written, so a
//! venue that never turned push on pays no outbox write for it.
//!
//!   placed               -> every staff device       "New order"
//!   status S (not PENDING) -> the order's customer devices  "Ready", ...
//!   READY with a courier -> that courier's devices    "Ready to collect"
//!   assigned to courier  -> that courier's devices    "An order was assigned to you"
//!
//! ONE ENTRY PER (event, device), id `{order}/push/{event}/{kind}/{record}`:
//! a replayed command writes over its own entry, and one dead phone retries
//! alone. The entry carries the device's keys, so the drain needs no second
//! read, and the customer's records can be removed when the order ends
//! without losing the "Delivered" that is already queued.

use serde_json::json;

use super::subs::{Sub, K_COURIER, K_CUSTOMER, K_STAFF};
use super::words;
use crate::outbox::Entry;

/// The outbox entry kind the drain sends (`push::rail`).
pub const KIND: &str = "push";

/// An order event a phone may be told about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ev<'a> {
    Placed { order: &'a str },
    Status { order: &'a str, status: &'a str, courier: Option<&'a str> },
    Assigned { order: &'a str, courier: &'a str },
}

/// The subscriptions a venue holds: `(kind, record id, sub)`.
pub type Held<'a> = [(&'a str, String, Sub)];

/// What the turn writes: outbox entries, and push records to remove.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Owed {
    pub entries: Vec<Entry>,
    pub remove: Vec<(&'static str, String)>,
}

/// Terminal statuses (`OrderStatus::is_terminal`): the customer's devices go after the last word.
pub fn is_terminal(status: &str) -> bool {
    crate::services::orders::status::is_terminal(status)
}

/// THE PRODUCER.
pub fn owed(ev: Ev, held: &Held, now_ms: i64) -> Owed {
    let mut out = Owed::default();
    let mut tell = |kind: &'static str, key: &str, tag: &str, order: &str, body: &dyn Fn(&str) -> String, url: &str| {
        let prefix = format!("{key}/");
        for (k, id, sub) in held.iter().filter(|(k, id, _)| *k == kind && id.starts_with(&prefix)) {
            let msg = json!({
                "title": words::title(&sub.lang, order),
                "body": body(&sub.lang),
                "url": url,
                "tag": format!("{}-{}", words::short(order), tag),
            });
            let text = json!({ "k": k, "id": id, "p256dh": sub.p256dh, "auth": sub.auth, "msg": msg });
            out.entries.push(Entry::new(format!("{order}/push/{tag}/{k}/{id}"), KIND, sub.endpoint.clone(), text.to_string(), now_ms));
        }
    };
    match ev {
        Ev::Placed { order } => {
            // Staff are keyed by person; every staff device of the venue hears it.
            for key in staff_keys(held) {
                tell(K_STAFF, &key, "new", order, &|l| words::new_order(l).to_string(), "/admin/");
            }
        }
        Ev::Status { order, status, courier } => {
            if words::speaks(status) {
                let url = format!("/?order={order}");
                tell(K_CUSTOMER, order, &status.to_ascii_lowercase(), order, &|l| words::status(l, status).unwrap_or("").to_string(), &url);
            }
            if let (Some(c), "READY") = (courier, status) {
                tell(K_COURIER, c, "ready", order, &|l| words::ready_to_collect(l).to_string(), "/courier/");
            }
            if is_terminal(status) {
                let prefix = format!("{order}/");
                out.remove.extend(held.iter().filter(|(k, id, _)| *k == K_CUSTOMER && id.starts_with(&prefix)).map(|(_, id, _)| (K_CUSTOMER, id.clone())));
            }
        }
        Ev::Assigned { order, courier } => {
            tell(K_COURIER, courier, "assigned", order, &|l| words::assigned(l).to_string(), "/courier/");
        }
    }
    out
}

/// The distinct people with a staff device.
fn staff_keys(held: &Held) -> Vec<String> {
    let mut keys: Vec<String> = held
        .iter()
        .filter(|(k, _, _)| *k == K_STAFF)
        .filter_map(|(_, id, _)| id.rsplit_once('/').map(|(key, _)| key.to_string()))
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

#[cfg(test)]
mod tests;
