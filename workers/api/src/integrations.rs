//! One screen's worth of truth: is each integration set up, and does it work.
//!
//! `GET /api/owner/integrations` reads what is configured. `POST
//! /api/owner/integrations/check {which}` PROVES one of them, without sending
//! anything to a customer: Telegram's `getMe`, the WhatsApp number's own
//! record, the Instagram account's username, the webhook handshake run
//! against this very Worker, a one-line object put in the bucket, the MCP
//! tool list, the Stripe secrets' presence. Each answer is the provider's
//! own words on failure, so the owner fixes the right thing.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::{owner_and_venue};

/// The Graph API version, the same one `channels.rs` pins.
const GRAPH: &str = "https://graph.facebook.com/v21.0";
/// The challenge the self-check sends through the webhook handshake.
const CHALLENGE: &str = "dowiz-check";
/// How much of a provider's refusal is shown.
const ERROR_SHOWN: usize = 240;
/// The settings key `channels::webhook` stamps on every delivery.
pub const WEBHOOK_LAST_KEY: &str = "notify.meta.last";
/// The probe object's content; the object is tiny and named by the moment.
const PROBE_BODY: &str = "dowiz cloud check\n";

fn settings_flag(s: &dowiz_hub::settings::Settings, key: &str) -> bool {
    !s.known(key).trim().is_empty() || s.get(key).is_some_and(|v| !v.trim().is_empty())
}

/// `GET /api/owner/integrations`
pub async fn status(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let s = crate::hubstore::load_settings(&place).await?.settings;
    let origin = req.url()?.origin().ascii_serialization();
    // The venue's raw record lives in the catalogue image, wallets included.
    let loc_json: Value = crate::hubstore::load_catalog(&place)
        .await
        .ok()
        .and_then(|c| c.catalog.location())
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or(Value::Null);
    let _ = loc;
    let wallets = crate::storefront::payment_wallets(&loc_json).len();
    let last_webhook = s.get(WEBHOOK_LAST_KEY).and_then(|v| v.trim().parse::<i64>().ok());
    Response::from_json(&json!({
        "telegram": { "configured": crate::notify::bot_token(&ctx.env, &s).is_some() && settings_flag(&s, "notify.telegram.chat") },
        "whatsapp": { "configured": crate::channels::whatsapp_cfg(&s).is_some(), "notifies": settings_flag(&s, "notify.whatsapp.to") },
        "instagram": { "configured": crate::channels::instagram_cfg(&s).is_some() },
        "webhook": { "url": format!("{origin}/api/webhooks/meta"), "verifySet": settings_flag(&s, "notify.whatsapp.verify"),
                     "secretSet": s.get("notify.meta.secret").is_some(), "lastMs": last_webhook },
        "cloud": { "configured": crate::cloud::cfg(&s).is_some(), "bucket": s.known("cloud.s3.bucket") },
        "mcp": { "url": format!("{origin}/api/mcp"), "tools": crate::mcp::tool_count() },
        "stripe": { "configured": ctx.env.secret("STRIPE_SECRET_KEY").is_ok() && ctx.env.secret("STRIPE_PUBLISHABLE_KEY").is_ok() },
        "crypto": { "wallets": wallets },
        // ENABLED IS NOT THE SAME AS USABLE, and reporting only the flag is
        // what let this screen say the assistant was on while every question
        // answered 400. A Worker can only call https, so an endpoint that is
        // not https is a switch with nothing behind it.
        "ai": {
            "enabled": s.flag("ai.enabled"),
            "endpoint": s.known("ai.endpoint"),
            "usable": s.flag("ai.enabled") && s.known("ai.endpoint").starts_with("https://"),
        },
    }))
}

async fn graph_get(token: &str, path: &str) -> std::result::Result<Value, String> {
    let headers = Headers::new();
    headers.set("authorization", &format!("Bearer {token}")).map_err(|e| e.to_string())?;
    let r = Request::new_with_init(&format!("{GRAPH}/{path}"), RequestInit::new().with_method(Method::Get).with_headers(headers))
        .map_err(|e| e.to_string())?;
    let mut res = Fetch::Request(r).send().await.map_err(|e| e.to_string())?;
    let body = res.text().await.unwrap_or_default();
    let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    if res.status_code() < 400 {
        return Ok(v);
    }
    Err(v["error"]["message"].as_str().map(str::to_string).unwrap_or(body).chars().take(ERROR_SHOWN).collect())
}

async fn telegram_get_me(token: &str) -> std::result::Result<Value, String> {
    let r = Request::new(&format!("https://api.telegram.org/bot{token}/getMe"), Method::Get).map_err(|e| e.to_string())?;
    let mut res = Fetch::Request(r).send().await.map_err(|e| e.to_string())?;
    let body = res.text().await.unwrap_or_default();
    let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    if v["ok"].as_bool().unwrap_or(false) {
        return Ok(v["result"].clone());
    }
    Err(v["description"].as_str().map(str::to_string).unwrap_or(body).chars().take(ERROR_SHOWN).collect())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckIn {
    #[allow(dead_code)]
    location_id: Option<String>,
    which: String,
}

/// `POST /api/owner/integrations/check` — one proof, the provider's words.
pub async fn check(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: CheckIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let s = crate::hubstore::load_settings(&place).await?.settings;
    let origin = req.url()?.origin().ascii_serialization();
    let verdict: std::result::Result<Value, (&str, String)> = match body.which.as_str() {
        "telegram" => match crate::notify::bot_token(&ctx.env, &s) {
            None => Err(("no_token", "no bot token is set".to_string())),
            Some(token) => telegram_get_me(&token).await.map_err(|e| ("provider", e)).map(|me| json!({
                "bot": me["username"], "chatSet": settings_flag(&s, "notify.telegram.chat") })),
        },
        "whatsapp" => match crate::channels::whatsapp_cfg(&s) {
            None => Err(("no_phone", "token or phone number id missing".to_string())),
            Some(wa) => graph_get(&wa.token, &format!("{}?fields=display_phone_number,verified_name,quality_rating", wa.phone_id))
                .await
                .map_err(|e| ("provider", e))
                .map(|v| json!({ "number": v["display_phone_number"], "name": v["verified_name"], "quality": v["quality_rating"], "notifies": wa.to })),
        },
        "instagram" => match crate::channels::instagram_cfg(&s) {
            None => Err(("no_account", "token or account id missing".to_string())),
            Some(ig) => graph_get(&ig.token, &format!("{}?fields=username,followers_count,media_count", ig.user_id))
                .await
                .map_err(|e| ("provider", e))
                .map(|v| json!({ "username": v["username"], "followers": v["followers_count"], "posts": v["media_count"] })),
        },
        "webhook" => {
            // The handshake Meta will perform, performed here against this
            // Worker's own router, with the stored verify token.
            let want = s.known("notify.whatsapp.verify");
            if want.trim().is_empty() {
                Err(("no_verify", "no verify token is set".to_string()))
            } else {
                let url = format!(
                    "{origin}/api/webhooks/meta?hub.mode=subscribe&hub.verify_token={}&hub.challenge={CHALLENGE}",
                    crate::mcp::enc(want.trim())
                );
                let headers = Headers::new();
                if let Ok(Some(h)) = req.headers().get("host") {
                    headers.set("host", &h)?;
                }
                let probe = Request::new_with_init(&url, RequestInit::new().with_method(Method::Get).with_headers(headers))?;
                match crate::route(probe, ctx.env.clone()).await {
                    Ok(mut r) => {
                        let status = r.status_code();
                        let echoed = r.text().await.unwrap_or_default();
                        if status == 200 && echoed == CHALLENGE {
                            Ok(json!({
                                "url": format!("{origin}/api/webhooks/meta"), "signatureChecked": s.get("notify.meta.secret").is_some(),
                                "lastMs": s.get(WEBHOOK_LAST_KEY).and_then(|v| v.trim().parse::<i64>().ok()) }))
                        } else {
                            Err(("handshake", format!("handshake answered {status}")))
                        }
                    }
                    Err(e) => Err(("provider", e.to_string())),
                }
            }
        },
        "cloud" => match crate::cloud::cfg(&s) {
            None => Err(("no_bucket", "endpoint, bucket, key or secret missing".to_string())),
            Some(s3) => {
                let now = ctx.data.now_ms;
                let key = if s3.prefix.is_empty() { format!("{}/probe-{now}.txt", place.venue) } else { format!("{}/{}/probe-{now}.txt", s3.prefix, place.venue) };
                crate::cloud::put(&s3, &key, PROBE_BODY.as_bytes().to_vec(), "text/plain", now)
                    .await
                    .map_err(|e| ("provider", e))
                    .map(|etag| json!({ "bucket": s3.bucket, "key": key, "etag": etag }))
            }
        },
        "mcp" => Ok(json!({ "url": format!("{origin}/api/mcp"), "tools": crate::mcp::tool_count(), "names": crate::mcp::tool_names() })),
        "stripe" => {
            let (pk, sk) = (ctx.env.secret("STRIPE_PUBLISHABLE_KEY").is_ok(), ctx.env.secret("STRIPE_SECRET_KEY").is_ok());
            if pk && sk { Ok(json!({ "configured": true })) } else { Err(("no_stripe", format!("missing: {}{}", if pk { "" } else { "STRIPE_PUBLISHABLE_KEY " }, if sk { "" } else { "STRIPE_SECRET_KEY" }))) }
        },
        "ai" => {
            if !s.flag("ai.enabled") { Err(("ai_off", "the assistant is off".to_string())) } else {
                // The endpoint's model list is the cheapest question an
                // OpenAI-compatible server answers.
                let base = s.known("ai.endpoint");
                if base.trim().is_empty() {
                    return Response::from_json(&json!({ "ok": false, "which": "ai", "code": "no_endpoint", "error": "no endpoint is set" })).map(|r| r.with_status(502));
                }
                let headers = Headers::new();
                if let Some(tok) = s.get("ai.token") { headers.set("authorization", &format!("Bearer {}", tok.trim()))?; }
                let r = Request::new_with_init(&format!("{}/models", base.trim_end_matches('/')), RequestInit::new().with_method(Method::Get).with_headers(headers))?;
                match Fetch::Request(r).send().await {
                    Ok(mut res) if res.status_code() < 400 => {
                        let v: Value = res.json().await.unwrap_or(Value::Null);
                        let n = v["data"].as_array().map(|a| a.len()).unwrap_or(0);
                        Ok(json!({ "endpoint": base, "models": n, "model": s.known("ai.model") }))
                    }
                    Ok(mut res) => Err(("provider", format!("{} {}", res.status_code(), res.text().await.unwrap_or_default().chars().take(ERROR_SHOWN).collect::<String>()))),
                    Err(e) => Err(("provider", e.to_string())),
                }
            }
        },
        other => return Response::error(format!("unknown integration {other:?}"), 400),
    };
    match verdict {
        Ok(v) => Response::from_json(&json!({ "ok": true, "which": body.which, "detail": v })),
        Err((code, e)) => {
            let mut r = Response::from_json(&json!({ "ok": false, "which": body.which, "code": code, "error": e }))?;
            r = r.with_status(502);
            Ok(r)
        }
    }
}
