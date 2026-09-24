//! FORGETTING A GUEST IN THE BOOKINGS IMAGE (G8 of the 2026-09-24 audit, D15).
//!
//! THE RESERVATION STAYS, THE PERSON GOES -- the order log's rule
//! (`services::customers::forget`). A booking is event-folded like an order:
//! its events (who moved it, when, to what) are the venue's history and the
//! kernel replays them, so they are never touched. What is emptied is the one
//! place the person is written: `contact_name`, `contact_phone`, and the
//! `phone_key` whose index row (`rsv.phone/<key>/...`) would otherwise still
//! say "this person booked here". `store::index_of` derives the index from the
//! record alone, so re-putting the record without `phone_key` IS how that row
//! goes.
//!
//! PURE over `Table`: the Worker (which holds the secret the keys are HMACs
//! under) picks the ids with [`ids_of`]; the object redacts them with
//! [`redact`] in the forget turn. Both idempotent.

use std::collections::BTreeSet;

use dowiz_hub::table::Table;
use serde_json::{json, Value};

use super::store::index_of;
use super::K_RSV;

/// The bookings that are the person's: their `phone_key` is in `keys` (the
/// alias circle), or their typed phone hashes into it under any of the
/// spellings `key_of` yields (a booking made before `phone_key` existed has
/// only the phone). Sorted, so the object's input is deterministic.
pub fn ids_of(t: &Table, keys: &BTreeSet<String>, key_of: impl Fn(&str) -> Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for (id, j) in t.all(K_RSV) {
        let Ok(r) = serde_json::from_str::<Value>(&j) else { continue };
        let s = |k: &str| r.get(k).and_then(Value::as_str).filter(|v| !v.is_empty());
        let by_key = s("phone_key").is_some_and(|k| keys.contains(k));
        let by_phone = s("contact_phone").is_some_and(|p| key_of(p).iter().any(|k| keys.contains(k)));
        if by_key || by_phone {
            out.push(id);
        }
    }
    out.sort();
    out
}

/// Every key a typed phone may be filed under: its raw spelling's customer
/// key, and the canonical (E.164) one `guest::phone_key` writes on a booking.
pub fn keys_of_phone(secret: &[u8], phone: &str) -> Vec<String> {
    let mut out = vec![crate::services::customers::handlers::customer_key(secret, phone)];
    out.extend(super::guest::phone_key(secret, phone));
    out
}

/// Empty the person out of each booking in `ids`. Returns how many records
/// changed; a booking already emptied, or not in the image, counts nothing.
pub fn redact(t: &mut Table, ids: &[String]) -> Result<usize, String> {
    let mut n = 0;
    for id in ids {
        let Some(mut r) = t.get(K_RSV, id).and_then(|j| serde_json::from_str::<Value>(&j).ok()) else { continue };
        let holds = |r: &Value, k: &str| r.get(k).is_some_and(|v| !v.is_null() && *v != json!(""));
        if !["contact_name", "contact_phone", "phone_key"].iter().any(|k| holds(&r, k)) {
            continue;
        }
        r["contact_name"] = json!("");
        r["contact_phone"] = json!("");
        r["phone_key"] = Value::Null;
        let index = index_of(id, &r);
        t.put(K_RSV, id, &r.to_string(), &index, &[]).map_err(|x| format!("booking {id}: {x}"))?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests;
