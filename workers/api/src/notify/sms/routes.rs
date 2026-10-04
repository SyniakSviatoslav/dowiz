//! THE SMS ROUTES (W-SMS). The owner's are authenticated as every owner route
//! is (`owner_and_venue`) and act on the venue the caller was authorised for
//! and no other (`Place::of_authorised`).
//!
//!   GET  /api/owner/sms                     the card: configuration (masked), health, waiting
//!   POST /api/owner/sms                     the owner's edit (`config::CfgIn`)
//!   POST /api/owner/sms/test                {phone}: one test text NOW, the provider's verdict
//!   POST /api/owner/sms/stop                {phone}: the customer said STOP; filed, the drain obeys
//!   GET  /api/public/locations/:slug/sms    the checkout's box: on? and the sentences
//!
//! THE TEST GOES TO THE OWNER'S OWN NUMBER, typed by the owner, and is not an
//! order-status text: it goes through `gateway::post`, not the customer door
//! `gateway::sms_text` that the consent gate counts.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::{config, gateway, rail, words};
use crate::hubstore::Place;
use crate::owner::owner_and_venue;

/// `{phone}`, for test and stop. A CLOSED shape.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhoneIn {
    pub phone: String,
}

async fn owner_place(req: &Request, ctx: &RouteContext<crate::Req>) -> Result<std::result::Result<(String, Place), Response>> {
    match owner_and_venue(req, ctx).await {
        Ok((who, loc)) => Ok(Ok((who, Place::of_authorised(ctx, &loc)?))),
        Err(r) => Ok(Err(r)),
    }
}

/// The card's state, from the settings and the outbox image. PURE.
pub fn card(settings: &impl config::Read, health: Option<&str>, waiting: usize, now_ms: i64) -> Value {
    let h = rail::Health::of(health, now_ms);
    json!({
        "config": config::view(settings),
        "health": h,
        "waiting": waiting,
        "providers": ["smsgate", "textbee", "twilio"],
    })
}

/// THE HEALTH PANE'S LINE (`GET /api/owner/health` -> `sms`): `{on:false}` for
/// a venue without texts; otherwise what is missing, today's count, the last
/// success and the last failure with its cause, and how many wait.
pub async fn health(place: &Place, settings: Option<&dowiz_hub::settings::Settings>, now_ms: i64) -> Value {
    let Some(s) = settings else { return json!({ "error": "settings unreadable" }) };
    if !config::is_on(s) {
        return json!({ "on": false });
    }
    match crate::hubstore::load_table(place, crate::outbox::IMAGE_OUTBOX, crate::outbox::OUTBOX_BYTES).await {
        Ok(l) => {
            let waiting = l.table.all(crate::outbox::KIND).into_iter().filter(|(_, j)| serde_json::from_str::<crate::outbox::Entry>(j).is_ok_and(|e| e.kind == super::plan::KIND)).count();
            let mut v = card(s, l.table.get(rail::HEALTH_KIND, rail::HEALTH_ID).as_deref(), waiting, now_ms);
            v["on"] = json!(true);
            v["missing"] = v["config"]["missing"].clone();
            v.as_object_mut().map(|m| m.remove("config"));
            v.as_object_mut().map(|m| m.remove("providers"));
            v
        }
        // An unreadable queue is NOT an empty one.
        Err(e) => json!({ "on": true, "error": e.to_string() }),
    }
}

/// `GET /api/owner/sms`
pub async fn status(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = match owner_place(&req, &ctx).await? {
        Ok((_, p)) => p,
        Err(r) => return Ok(r),
    };
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    let image = crate::hubstore::load_table(&place, crate::outbox::IMAGE_OUTBOX, crate::outbox::OUTBOX_BYTES).await?.table;
    let waiting = image
        .all(crate::outbox::KIND)
        .into_iter()
        .filter(|(_, j)| serde_json::from_str::<crate::outbox::Entry>(j).is_ok_and(|e| e.kind == super::plan::KIND))
        .count();
    Response::from_json(&card(&settings, image.get(rail::HEALTH_KIND, rail::HEALTH_ID).as_deref(), waiting, ctx.data.now_ms))
}

/// `POST /api/owner/sms`
pub async fn set(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: config::CfgIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let place = match owner_place(&req, &ctx).await? {
        Ok((_, p)) => p,
        Err(r) => return Ok(r),
    };
    let stored = !crate::hubstore::load_settings(&place).await?.settings.get(config::KEY_SECRET).unwrap_or_default().trim().is_empty();
    let writes = match config::edit(&body, stored) {
        Ok(w) => w,
        Err(why) => return Response::error(why, 400),
    };
    crate::hubstore::with_settings(&place, move |s| {
        for (k, v) in &writes {
            if v.is_empty() {
                s.clear(k);
            } else {
                s.set(k, v);
            }
        }
        Ok(())
    })
    .await?;
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    Response::from_json(&json!({ "ok": true, "config": config::view(&settings) }))
}

/// `POST /api/owner/sms/test` — 200 whatever happened: the verdict IS the answer.
pub async fn test(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: PhoneIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(to) = super::phone::e164(&body.phone, "") else {
        return Response::error("type the number in full, like +355 69 123 4567", 400);
    };
    let place = match owner_place(&req, &ctx).await? {
        Ok((_, p)) => p,
        Err(r) => return Ok(r),
    };
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    let cfg = match config::of_any(&settings) {
        Ok(c) => c,
        Err(missing) => return Response::from_json(&json!({ "ok": false, "why": missing })),
    };
    let venue = crate::hubstore::venue_record(&place).await.ok().flatten().and_then(|v| v.get("name").and_then(Value::as_str).map(str::to_string)).unwrap_or(place.venue.clone());
    let id = format!("test/{}", ctx.data.now_ms);
    let (code, words_back) = gateway::post(&gateway::request(&cfg, &to, &words::test_text(&venue), &id)).await;
    let ok = code != 0 && gateway::classify(cfg.provider, code) == gateway::Outcome::Sent;
    Response::from_json(&json!({
        "ok": ok,
        "status": code,
        "provider": cfg.provider.as_str(),
        // SMSGate's own id for this text: `GET /3rdparty/v1/messages/{id}` on the
        // gateway says whether the phone sent it (the live probe reads it).
        "message_id": if cfg.provider == config::Provider::SmsGate { json!(gateway::message_id(&id)) } else { Value::Null },
        "why": if ok { Value::Null } else { json!(gateway::why(code)) },
        "detail": if ok { Value::Null } else { json!(words_back) },
    }))
}

/// `POST /api/owner/sms/stop` — the customer said STOP (at the counter, on the
/// phone, by a reply the gateway saw). Filed under the customer's key; the
/// drain's fold drops every text still queued for them.
pub async fn stop(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: PhoneIn = match crate::body::parse(&mut req).await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let numbers = super::phone::candidates(&body.phone);
    if numbers.is_empty() {
        return Response::error("type the customer's number", 400);
    }
    let (who, place) = match owner_place(&req, &ctx).await? {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let secret = crate::services::customers::handlers::signing_secret(&ctx.env);
    // The SAME key the checkout filed the tick under: `customer_key` of the E.164 number.
    for n in &numbers {
        let key = crate::services::customers::handlers::customer_key(&secret, n);
        if let Err(why) = crate::services::customers::consent_log::file(&place, &super::checkout::stop_act(&key, &who, ctx.data.now_ms)).await {
            return Response::error(why, 500);
        }
    }
    Response::from_json(&json!({ "ok": true, "numbers": numbers.len() }))
}

/// `GET /api/public/locations/:slug/sms` — is the box shown here, and its words.
pub async fn box_for(_req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else { return Response::error("no venue named", 400) };
    let place = Place::of_slug(&ctx, &slug).await?;
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    use dowiz_hub::consent::sms_wordings::{sms_wording_id, SMS_WORDINGS};
    let out: Vec<Value> = SMS_WORDINGS.iter().map(|(l, text)| json!({ "lang": l, "id": sms_wording_id(l), "text": text })).collect();
    let mut res = Response::from_json(&json!({ "on": config::of(&settings).is_ok(), "wordings": out }))?;
    res.headers_mut().set("cache-control", "public, max-age=60")?;
    Ok(res)
}

#[cfg(test)]
mod tests;
