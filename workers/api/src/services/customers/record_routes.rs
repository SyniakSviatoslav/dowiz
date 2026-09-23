//! The owner's writes to the customer CARD (§3.1). ORCHESTRATION ONLY: the
//! rules are `record::merge` and `record::rekey`, tested natively.

use serde_json::json;
use worker::*;

use super::handlers::{customer_key, signing_secret};
use super::record::{self, Card, KIND};
use crate::owner::owner_and_venue;
use crate::services::orders::mine::of_venue as orders_of;

/// A `customer_key` is 16 lowercase hex characters (8 bytes of an HMAC). A
/// path that is not one names nobody, and writing a card under it would make
/// a record no row can ever show.
pub(crate) fn is_key(k: &str) -> bool {
    k.len() == 16 && k.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// `PUT /api/owner/customers/:key/record?location_id=` — the paper card.
///
/// The body is `record::Card`, a CLOSED shape: `spent` or `tier` is a 400.
/// Every refusal of `merge` is answered 400 with its reason and nothing is
/// written.
pub async fn put_record(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let card: Card = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (_who, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let Some(key) = ctx.param("key").cloned().filter(|k| is_key(k)) else {
        return Response::error("not a customer key", 400);
    };
    let now = ctx.data.now_ms;
    let out = crate::hubstore::with_table(
        &place,
        crate::hubstore::IMAGE_PEOPLE,
        crate::hubstore::PEOPLE_BYTES,
        |t| {
            let existing = t.get(KIND, &key).unwrap_or_else(|| "{}".into());
            let merged = match record::merge(&existing, &card, now) {
                Ok(m) => m,
                Err(why) => return Ok(Err(why)),
            };
            t.put(KIND, &key, &merged, &[], &[])
                .map_err(|e| Error::RustError(format!("customer card: {e:?}")))?;
            Ok(Ok(merged))
        },
    )
    .await?;
    match out {
        Ok(rec) => Response::from_json(&json!({
            "key": key,
            "record": serde_json::from_str::<serde_json::Value>(&rec).unwrap_or(json!({})),
        })),
        Err(why) => Response::error(why, 400),
    }
}

/// `POST /api/owner/customers/rekey?location_id=` — the ONE-SHOT re-key.
///
/// Cards filed under the old `sha256_hex(raw phone)` move to `customer_key`;
/// the phones come from this venue's own orders, the only place they are.
/// IDEMPOTENT: a second call answers `changed: 0`.
pub async fn rekey(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (_who, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let secret = signing_secret(&ctx.env);
    let listed = crate::hubstore::orders(&place).await?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    for o in orders_of(listed, &loc) {
        let Some(phone) = o.get("contact").and_then(|c| c.get("phone")).and_then(|p| p.as_str())
        else {
            continue;
        };
        let pair = (crate::auth::sha256_hex(phone), customer_key(&secret, phone));
        if !pairs.contains(&pair) {
            pairs.push(pair);
        }
    }
    let changed = crate::hubstore::with_table(
        &place,
        crate::hubstore::IMAGE_PEOPLE,
        crate::hubstore::PEOPLE_BYTES,
        |t| Ok(record::rekey(t, &pairs)),
    )
    .await?;
    Response::from_json(&json!({ "changed": changed }))
}
