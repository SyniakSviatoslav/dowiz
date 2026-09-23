//! FORGETTING A CUSTOMER (§3.3 of `BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22`).
//!
//! WHO THEY ARE is decided here, in the Worker, because the key is an HMAC
//! under `AUTH_SIGNING_KEY` and the object does not hold the secret: the
//! handler folds the hot log and every archive, keeps the orders whose phone
//! hashes to the key, and sends the object their IDS (pseudonymous) and the
//! legacy card ids. WHAT IS DONE to them happens in the object, in one turn
//! (`hubdo/forget.rs`), with the pure functions below:
//!
//!   1. `people`: the card and every other id it was filed under (§3.1's
//!      legacy `sha256_hex(raw phone)`) -- [`forget_people`];
//!   2. `consent`: redacted in place and stopped on every channel --
//!      `dowiz_hub::consent::forget`;
//!   3. the hot log, then each archive: the person's fields emptied in place
//!      (§2.2 B) and ONE `Forgotten` declaration in the hot log -- [`erase`].
//!
//! THE ORDER OF WRITES, AND WHAT A FAILURE BETWEEN THEM LEAVES. People and
//! consent first: both are idempotent and neither is counted by any law, so a
//! failure after them leaves a card gone and orders intact, and the retry
//! finishes. The HOT LOG next, carrying the declaration for every tombstone
//! this erasure makes, archives included. Archives LAST: a failure there
//! leaves the declaration naming more than exists, which is law 9 RED and
//! stays red -- an erasure half-done must be loud -- until the owner retries.
//! THE RETRY CONVERGES and never double-declares: [`erase`] declares
//! `tombstones(key's orders) - declared(cust:<key>)`, and the orders and
//! archives a previous run declared are read back from its declaration,
//! because by then their phones are gone and no fold can find them again.

use std::collections::BTreeSet;

use dowiz_hub::{EventKind, Hub};
use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::hubdo::OrderView;
use crate::services::customers::handlers::{customer_key, signing_secret};

/// The statuses of an order still on its way. MIRRORS `hubstore::LIVE`
/// (private there): an order in one of these still needs its address.
pub const LIVE: [&str; 5] = ["PENDING", "CONFIRMED", "PREPARING", "READY", "IN_DELIVERY"];

/// Whose orders these are, by the fold. `phones` are the spellings seen, for
/// the legacy card ids; `live` names an order that is still moving.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub orders: BTreeSet<String>,
    pub phones: BTreeSet<String>,
    pub live: Option<String>,
}

/// PURE. The orders of `loc` whose contact phone hashes to `key`.
pub fn find(views: &[OrderView], loc: &str, key: &str, secret: &[u8], found: &mut Found) {
    for v in views {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        if !crate::services::orders::mine::belongs_to(&o, loc) {
            continue;
        }
        let Some(phone) = o.pointer("/contact/phone").and_then(Value::as_str).filter(|p| !p.is_empty()) else {
            continue;
        };
        if customer_key(secret, phone) != key {
            continue;
        }
        found.orders.insert(v.order_id.clone());
        found.phones.insert(phone.to_string());
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        if found.live.is_none() && LIVE.contains(&status) {
            found.live = Some(v.order_id.clone());
        }
    }
}

/// The ids the card was filed under before §3.1's re-key: `sha256_hex` of the
/// raw number, one per spelling.
pub fn legacy_ids(phones: &BTreeSet<String>) -> Vec<String> {
    phones.iter().map(|p| crate::auth::sha256_hex(p)).collect()
}

/// PURE. Remove the card and every id it was filed under. Returns how many.
pub fn forget_people(t: &mut dowiz_hub::table::Table, key: &str, legacy: &[String]) -> usize {
    let kind = super::record::KIND;
    std::iter::once(key).chain(legacy.iter().map(String::as_str)).filter(|id| t.remove(kind, id)).count()
}

/// Empty a string field when it holds something; true when it did.
fn blank(v: &mut Value, path: &str, to: Value) -> bool {
    match v.pointer_mut(path) {
        Some(f) if !f.is_null() && *f != to && *f != json!("") => {
            *f = to;
            true
        }
        _ => false,
    }
}

/// PURE. The event's JSON with the PERSON taken out, or `None` when it holds
/// nothing of theirs (a status delta). The ORDER stays: money, lines, status,
/// times, the ledger (§2.2 rule 2). The home's coordinates go with its line.
pub fn redact_order_json(json_text: &str) -> Option<String> {
    let mut v: Value = serde_json::from_str(json_text).ok()?;
    let s = || json!("");
    let changed = [
        blank(&mut v, "/contact/phone", s()),
        blank(&mut v, "/contact/name", s()),
        blank(&mut v, "/fulfilment/note", s()),
        blank(&mut v, "/fulfilment/address/line", s()),
        blank(&mut v, "/fulfilment/address/note", s()),
        blank(&mut v, "/fulfilment/address/parts", Value::Null),
        blank(&mut v, "/fulfilment/address/lat_udeg", Value::Null),
        blank(&mut v, "/fulfilment/address/lon_udeg", Value::Null),
    ];
    changed.contains(&true).then(|| v.to_string())
}

/// The orders and archives earlier declarations for `cust:<key>` named.
pub fn declared_scope(hot: &Hub, key: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let subject = format!("cust:{key}");
    let (mut orders, mut archives) = (BTreeSet::new(), BTreeSet::new());
    for e in hot.events().iter().filter(|e| e.kind == EventKind::Forgotten && e.order_id == subject) {
        let Ok(d) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        let names = |k: &str| d.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
        orders.extend(names("orders").iter().filter_map(Value::as_str).map(str::to_string));
        archives.extend(names("archives").iter().filter_map(|a| a.get("id")?.as_str()).map(str::to_string));
    }
    (orders, archives)
}

/// What [`erase`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Erased {
    pub hot: usize,
    pub archived: Vec<(String, usize)>,
    /// What THIS run's declaration names; 0 when none was appended.
    pub declared: usize,
}

/// Who, why and when, for the declaration. No contact details (`lib.rs`'s
/// `Revealed` rule).
pub struct Act<'a> {
    pub key: &'a str,
    pub by: &'a str,
    pub reason: &'a str,
    pub now_ms: i64,
}

/// PURE. Redact `orders` in the hot log and in `archives`, then declare what
/// is not yet declared. See the header for why the count is a difference.
pub fn erase(act: &Act, orders: &BTreeSet<String>, hot: &mut Hub, archives: &mut [(String, Hub)]) -> Result<Erased, String> {
    let redactor = |e: &dowiz_hub::Event| orders.contains(&e.order_id).then(|| redact_order_json(&e.order_json)).flatten();
    let mut out = Erased { hot: hot.redact(redactor).map_err(|e| format!("redact hot: {e:?}"))?, ..Erased::default() };
    let mine = |id: &str| orders.contains(id);
    let mut tombstones = hot.tombstones_where(mine);
    for (id, arch) in archives.iter_mut() {
        let n = arch.redact(redactor).map_err(|e| format!("redact {id}: {e:?}"))?;
        tombstones += arch.tombstones_where(mine);
        out.archived.push((id.clone(), n));
    }
    let subject = format!("cust:{}", act.key);
    let owed = tombstones.saturating_sub(hot.declared_for(&subject));
    if owed == 0 {
        return Ok(out);
    }
    let declaration = json!({
        "records": owed,
        "hot": out.hot,
        // NOT `records` inside: `Hub::declared` reads the FIRST `"records":` in
        // the text, and serde_json sorts `archives` ahead of it.
        "archives": out.archived.iter().map(|(id, n)| json!({ "id": id, "redacted": n })).collect::<Vec<_>>(),
        "orders": orders,
        "by": act.by,
        "at": act.now_ms,
        "reason": act.reason,
    });
    let prev = hot.events().first().map(|e| e.seq).unwrap_or(0);
    let seq = crate::command::amend::next_seq(prev, act.now_ms);
    hot.append(EventKind::Forgotten, &subject, &declaration.to_string(), seq, [0u8; 32])
        .map_err(|e| format!("append the declaration: {e:?}"))?;
    out.declared = owed;
    Ok(out)
}

/// A customer key: 16 lowercase hex, `customer_key`'s shape.
pub fn is_customer_key(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ForgetBody {
    /// Why (the audit's). Written into the declaration: say why, not who.
    pub reason: String,
}

/// `POST /api/owner/customers/:key/forget?location_id=` -- owner only, on the
/// venue the caller is authorised for.
pub async fn forget_customer(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: ForgetBody = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (who, loc) = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let reason = body.reason.trim().to_string();
    if reason.chars().count() < 3 {
        return Response::error("say why this customer is being forgotten", 400);
    }
    let Some(key) = ctx.param("key").cloned().filter(|k| is_customer_key(k)) else {
        return Response::error("not a customer key", 400);
    };

    // WHO: the hot fold and every archive's, under the secret the object lacks.
    let secret = signing_secret(&ctx.env);
    let mut found = Found::default();
    find(&crate::hubstore::orders(&place).await?, &loc, &key, &secret, &mut found);
    let mut archives = Vec::new();
    for id in crate::hubstore::archives_of(&crate::hubstore::load_settings(&place).await?.settings) {
        let before = found.orders.len();
        let views = crate::hubstore::archive_orders(&place, &id).await?.unwrap_or_default();
        find(&views, &loc, &key, &secret, &mut found);
        if found.orders.len() > before {
            archives.push(id);
        }
    }
    if let Some(order) = &found.live {
        return Response::error(format!("order {order} is still on its way; finish or cancel it first"), 409);
    }

    let input = crate::hubdo::forget::ForgetIn {
        key: key.clone(),
        reason,
        by: who,
        now_ms: ctx.data.now_ms,
        orders: found.orders.into_iter().collect(),
        archives,
        legacy: legacy_ids(&found.phones),
    };
    let out: crate::hubdo::forget::ForgetOut = match crate::command::send(&place, "forget", &input).await {
        Ok(o) => o,
        Err((status, msg)) => {
            crate::loud!(&place.ns, Some(&place.venue), "customers.forget", "cust:{key} not forgotten ({status}): {msg}");
            return Response::error(msg, status);
        }
    };
    Response::from_json(&json!({
        "key": key,
        "redacted": out.redacted,
        "archives": out.archives,
        "declared": out.declared,
        "people": out.people,
        "consent": { "redacted": out.consent_redacted, "withdrawn": out.consent_withdrawn },
        "notice": "Copies older than tonight in the venue's own backup expire within one month, and nothing reads them.",
    }))
}

#[cfg(test)]
mod tests;
