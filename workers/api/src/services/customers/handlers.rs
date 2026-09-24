//! The owner's customer list, and the deliberate act of un-redacting one row.
//!
//! ORCHESTRATION ONLY. The fold is `roll`, the mask is `dowiz_hub::redact`,
//! and both are tested where they live.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::{owner_and_venue};
use crate::services::orders::mine::of_venue as orders_of;
use crate::services::venue::currency_of;


/// A stable, non-reversible handle for a phone. The audit entry must not carry
/// the number it is about, and a URL holding a phone number puts it in every
/// proxy log between here and the browser.
pub(crate) fn customer_key(secret: &[u8], phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    // ── `00` IS NOT PART OF THE NUMBER ──
    //
    // FOUND BY THE TEST THAT WAS SUPPOSED TO PROVE THIS RULE and until now
    // could not: the fold's "two spellings" test handed `roll` a pass-through
    // closure and the same spelling twice, so the normalisation here was
    // exercised by nothing. Keeping only the digits turns `+355691234567` into
    // `355691234567` and `00355691234567` into `00355691234567` — one person
    // who once dialled the international-access form is a SECOND customer,
    // with their own row, their own totals and their own reveal audit trail.
    //
    // A LEADING `00` IS THE INTERNATIONAL ACCESS CODE, the written form of the
    // `+`, and stripping it is the one normalisation that needs no country to
    // be known.
    //
    // WHAT IS DELIBERATELY NOT DONE: the NATIONAL form. `069 123 4567` and
    // `+355 69 123 4567` are the same Albanian phone and remain two handles
    // here, because collapsing them means knowing the venue's country and
    // guessing one is how a Kosovan number becomes an Albanian customer. It
    // needs the venue's dialling code passed in; that is a change to every
    // caller and it is written down rather than half-done.
    let digits = digits.strip_prefix("00").unwrap_or(&digits).to_string();
    let mac = dowiz_hub::crypto::hmac_sha256(secret, digits.as_bytes());
    dowiz_hub::crypto::hex(&mac[..8])
}

pub(crate) fn signing_secret(env: &Env) -> Vec<u8> {
    env.secret("AUTH_SIGNING_KEY")
        .map(|v| v.to_string().into_bytes())
        // A hub with no configured key still masks consistently WITHIN itself:
        // the key only has to be stable and secret, and an unconfigured Worker
        // has bigger problems than a predictable mask.
        .unwrap_or_else(|_| b"dowiz-unconfigured".to_vec())
}

/// `GET /api/owner/customers?location_id=&sort=spent|orders`
pub async fn customers(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and the image read do not depend on each other, so
    // `owner_beside` runs them together. Measured against this very handler
    // before it was converted: 293 ms of server time against the dashboard's
    // 235 for the same 655 KB, on the same deployment at the same minute.
    let (_, loc, (listed, loaded_cat, people, consent)) = match crate::owner::owner_beside(
        &req,
        &ctx,
        &place,
        futures_util::future::try_join4(
            crate::hubstore::orders(&place),
            crate::hubstore::load_catalog(&place),
            // THE CARD, joined on by key (§3.1). Read beside the other two,
            // so the row gaining its note costs no extra round trip in series.
            crate::hubstore::load_table(
                &place,
                crate::hubstore::IMAGE_PEOPLE,
                crate::hubstore::PEOPLE_BYTES,
            ),
            crate::hubstore::load_log(&place, crate::services::customers::consent_log::IMAGE_CONSENT),
        ),
    )
    .await
    {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let sort = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "sort").map(|(_, v)| v.to_string()));
    let secret = signing_secret(&ctx.env);

    // The fold and both masks are `services::customers`, where they have tests:
    // the mask is the WHOLE protection on this screen, and it had none.
    use crate::services::customers::roll;
    use dowiz_hub::redact;
    // LINK, NEVER MERGE (§3.4): every key is shown under the key it is linked
    // to, read from the `alias` kind of the same `people` image.
    let aliases = crate::services::customers::alias::Aliases::of(&people.table);
    let rows = roll::roll(
        &orders_of(listed, &loc),
        |phone| customer_key(&secret, phone),
        |key| aliases.resolve(key),
        redact::name,
        redact::phone,
        roll::Sort::of(sort.as_deref()),
    );
    let cat = loaded_cat.catalog;
    let acts = consent.log.entries();
    use dowiz_hub::consent::{CHANNEL_WHATSAPP, PURPOSE_MARKETING};
    use crate::services::customers::consent_log::circle_state;
    Response::from_json(&json!({
        "customers": rows.iter().map(|r| {
            // READ ON THE CIRCLE (D26): the row's consent is its person's.
            let members = aliases.members(&r.key);
            let circle: Vec<String> = std::iter::once(r.key.clone()).chain(members.iter().cloned()).collect();
            crate::services::customers::view::row_json(
                r,
                people.table.get(crate::services::customers::record::KIND, &r.key).as_deref(),
                circle_state(&acts, &circle, PURPOSE_MARKETING, CHANNEL_WHATSAPP).is_some(),
                &members,
            )
        }).collect::<Vec<_>>(),
        "currency": currency_of(&cat),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevealIn {
    reason: String,
}

/// `POST /api/owner/customers/:key/reveal?location_id=`
///
/// The audit entry is appended BEFORE the answer is returned. An un-auditable
/// reveal is the one thing this route must not do, and answering first would
/// make the log best-effort.
pub async fn reveal_customer(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: RevealIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let (who, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let reason = body.reason.trim().to_string();
    if reason.len() < 3 {
        return Response::error("say why you are looking", 400);
    }
    let Some(key) = ctx.param("key").cloned() else {
        return Response::error("missing customer", 400);
    };

    let secret = signing_secret(&ctx.env);
    // THE ROW'S KEY IS A CANONICAL ONE (§3.4): the orders of every spelling
    // linked into it are the row's orders, so they are the reveal's too.
    let (listed, people) = futures_util::future::try_join(
        crate::hubstore::orders(&place),
        crate::hubstore::load_table(&place, crate::hubstore::IMAGE_PEOPLE, crate::hubstore::PEOPLE_BYTES),
    )
    .await?;
    let aliases = crate::services::customers::alias::Aliases::of(&people.table);
    let mut found: Option<(String, String, Vec<Value>)> = None;
    for o in orders_of(listed, &loc) {
        let Some(phone) = o.get("contact").and_then(|c| c.get("phone")).and_then(Value::as_str)
            .filter(|p| super::roll::names_a_person(p))
        else {
            continue;
        };
        if aliases.resolve(&customer_key(&secret, phone)) != key {
            continue;
        }
        let name = o
            .get("contact")
            .and_then(|c| c.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let row = json!({
            "id": o.get("id").cloned().unwrap_or(Value::Null),
            "at": o.get("created_at_ms").cloned().unwrap_or(json!(0)),
            "status": o.get("status").cloned().unwrap_or(Value::Null),
            "total": o.get("total").cloned().unwrap_or(json!(0)),
            "address": o.get("fulfilment").and_then(|f| f.get("address"))
                .and_then(|a| a.get("line")).cloned().unwrap_or(Value::Null),
        });
        match &mut found {
            Some((_, _, rows)) => rows.push(row),
            None => found = Some((name, phone.to_string(), vec![row])),
        }
    }
    let Some((name, phone, orders)) = found else {
        return Response::error("not found", 404);
    };

    let at = ctx.data.now_ms;
    let entry = json!({ "by": who, "at": at, "reason": reason }).to_string();
    let subject = format!("cust:{key}");
    // AN AUDIT RECORD DEPENDS ON NOTHING THE LOG ALREADY SAYS, so it is
    // written without reading anything first.
    crate::hubstore::append_blind(
        &place,
        dowiz_hub::EventKind::Revealed,
        &subject,
        &entry,
        ctx.data.now_ms,
    )
        .await?;

    Response::from_json(&json!({ "name": name, "phone": phone, "orders": orders }))
}

/// `GET /api/owner/customers/reveals?location_id=` — who has been looking.
pub async fn reveals(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and this read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    // THE AUDIT TRAIL IS NOT A PROJECTION. `reveals()` reads the events the
    // fold deliberately skips, so this route still reads the image.
    let (_, _loc, loaded) =
        match crate::owner::owner_beside(&req, &ctx, &place, crate::hubstore::load(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
    let out: Vec<Value> = loaded
        .hub
        .reveals()
        .into_iter()
        .take(200)
        .filter_map(|e| {
            let v: Value = serde_json::from_str(&e.order_json).ok()?;
            Some(json!({
                "customer": e.order_id.strip_prefix("cust:").unwrap_or(&e.order_id),
                "by": v.get("by").cloned().unwrap_or(Value::Null),
                "at": v.get("at").cloned().unwrap_or(json!(0)),
                "reason": v.get("reason").cloned().unwrap_or(Value::Null),
                // A LINK IS A REVEAL OF BOTH (§3.4): `alias_routes` files it
                // here with `act` "linked"/"unlinked"; a plain reveal has none.
                "act": v.get("act").cloned().unwrap_or(Value::Null),
            }))
        })
        .collect();
    Response::from_json(&json!({ "reveals": out }))
}

