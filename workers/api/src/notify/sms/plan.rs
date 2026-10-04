//! WHAT AN ORDER TURN OWES THE CUSTOMER'S PHONE, PURE (the producer).
//!
//! The venue's object calls this in the turn that wrote the status
//! (`hubdo/sms_turn.rs`) and writes the answer into the outbox in the same
//! turn, so "the order moved" and "the customer is owed a text" are one write.
//!
//! NOTHING IS OWED unless ALL of: the order carries the `sms` stamp (the box
//! was ticked, `checkout::stamp`), the venue has SMS on, the status is one a
//! customer is texted about (`words::texts`), and the stamped number is E.164.
//!
//! ONE ENTRY PER (order, status), id `{order}/sms/{status}`: a replayed
//! command writes over its own entry. The entry carries the RENDERED text and
//! the customer's consent key; the drain re-asks consent with that key.

use serde_json::{json, Value};

use super::{phone, words};
use crate::outbox::Entry;

/// The outbox entry kind the drain sends (`sms::rail`).
pub const KIND: &str = "sms";

/// What an `sms` entry's text carries.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Payload {
    /// The customer's consent key (`customer_key`): the drain's fold reads it.
    pub key: String,
    /// The rendered text.
    pub body: String,
}

/// The stamp a placement puts on the order (`checkout::stamp`), read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    pub key: String,
    pub lang: String,
    pub venue: String,
    /// The number in E.164, normalised AT PLACEMENT where the venue's
    /// currency says which country a national spelling belongs to.
    pub to: String,
}

pub fn stamp_of(order: &Value) -> Option<Stamp> {
    let s = order.get("sms")?;
    let get = |k: &str| s.get(k).and_then(Value::as_str).map(str::to_string);
    let to = get("to")?;
    // Re-checked: a stamp is data in a log, and a log is not a promise.
    phone::e164(&to, "")?;
    Some(Stamp { key: get("key")?, lang: get("lang").unwrap_or_else(|| "en".into()), venue: get("venue").unwrap_or_default(), to })
}

/// THE PRODUCER. `order` is the order as the turn left it.
pub fn owed(order_id: &str, order: &Value, status: &str, now_ms: i64) -> Option<Entry> {
    let st = stamp_of(order)?;
    let kind = order.get("fulfilment").and_then(|f| f.get("kind")).and_then(Value::as_str).unwrap_or("delivery");
    // A TABLE IS IN THE ROOM: nobody texts a guest the waiter is facing.
    if kind == "dine_in" || !words::texts(status, kind) {
        return None;
    }
    let body = words::text(&st.lang, &st.venue, order_id, status)?;
    let text = json!({ "key": st.key, "body": body }).to_string();
    Some(Entry::new(format!("{order_id}/sms/{}", status.to_ascii_lowercase()), KIND, st.to, text, now_ms))
}

#[cfg(test)]
mod tests;
