//! WhatsApp and Instagram, both through Meta's Graph API.
//!
//! Three things live here, because they share one credential set and one
//! webhook:
//!
//! 1. **Sending.** A WhatsApp text from the venue's business number, an
//!    Instagram DM from its professional account, and an Instagram photo post
//!    (a two-step publish: container, then publish).
//! 2. **The webhook.** Meta delivers a customer's message to
//!    `/api/webhooks/meta` on the venue's own host, which is how the venue is
//!    known without a token. Delivery is acknowledged with 200 always, since
//!    Meta retries anything else for days; a message that cannot be stored is
//!    logged, not bounced.
//! 3. **The inbox.** The owner's side of those conversations: threads, one
//!    thread, a reply.
//!
//! Nothing here decides anything about an order. A customer who writes "two
//! Philadelphia to Rruga 1" is a message the owner reads and answers, which is
//! what a small kitchen actually wants from WhatsApp.

use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Sha256;
use worker::wasm_bindgen::JsValue;
use worker::*;

use crate::owner::{now_ms, owner_and_venue};

/// The Graph API version every call pins. Meta retires a version two years on.
const GRAPH: &str = "https://graph.facebook.com/v21.0";
/// WhatsApp refuses a text body longer than this.
const TEXT_MAX: usize = 4096;
/// How much of Meta's error text the owner is shown.
const ERROR_SHOWN: usize = 240;
/// Threads listed in the inbox, and messages folded to build them.
const THREADS_SHOWN: usize = 60;
const RECENT_ROWS: usize = 600;
/// Messages returned for one thread.
const THREAD_ROWS: usize = 200;
/// The verify token Meta sends on subscription, and the signature header it
/// sends with every delivery.
const HUB_VERIFY_PARAM: &str = "hub.verify_token";
const HUB_CHALLENGE_PARAM: &str = "hub.challenge";
const SIGNATURE_HEADER: &str = "x-hub-signature-256";
const SIGNATURE_PREFIX: &str = "sha256=";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    WhatsApp,
    Instagram,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::WhatsApp => "whatsapp",
            Channel::Instagram => "instagram",
        }
    }
    pub fn from_str(s: &str) -> Option<Channel> {
        match s {
            "whatsapp" => Some(Channel::WhatsApp),
            "instagram" => Some(Channel::Instagram),
            _ => None,
        }
    }
}

// ── credentials ─────────────────────────────────────────────────────────────

pub struct WhatsApp {
    pub token: String,
    pub phone_id: String,
    /// Where new orders are announced; empty means "no bell, inbox only".
    pub to: String,
}

pub fn whatsapp_cfg(s: &dowiz_hub::settings::Settings) -> Option<WhatsApp> {
    let token = s.get("notify.whatsapp.token")?.trim().to_string();
    let phone_id = s.known("notify.whatsapp.phone_id").trim().to_string();
    if token.is_empty() || phone_id.is_empty() {
        return None;
    }
    Some(WhatsApp { token, phone_id, to: s.known("notify.whatsapp.to").trim().to_string() })
}

pub struct Instagram {
    pub token: String,
    pub user_id: String,
}

pub fn instagram_cfg(s: &dowiz_hub::settings::Settings) -> Option<Instagram> {
    let token = s.get("social.instagram.token")?.trim().to_string();
    let user_id = s.known("social.instagram.user_id").trim().to_string();
    if token.is_empty() || user_id.is_empty() {
        return None;
    }
    Some(Instagram { token, user_id })
}

// ── sending ─────────────────────────────────────────────────────────────────

/// One Graph API POST. `Err` is Meta's own `error.message`, which names the
/// fix ("(#131030) Recipient phone number not in allowed list" tells a venue
/// still in sandbox exactly what to do).
async fn graph_post(token: &str, path: &str, payload: Value) -> std::result::Result<Value, String> {
    let headers = Headers::new();
    headers.set("content-type", "application/json").map_err(|e| e.to_string())?;
    headers.set("authorization", &format!("Bearer {token}")).map_err(|e| e.to_string())?;
    let r = Request::new_with_init(
        &format!("{GRAPH}/{path}"),
        RequestInit::new().with_method(Method::Post).with_headers(headers).with_body(Some(payload.to_string().into())),
    )
    .map_err(|e| e.to_string())?;
    let mut res = Fetch::Request(r).send().await.map_err(|e| e.to_string())?;
    let body = res.text().await.unwrap_or_default();
    let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    if res.status_code() < 400 {
        return Ok(v);
    }
    let msg = v
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or(body);
    Err(msg.chars().take(ERROR_SHOWN).collect())
}

pub async fn whatsapp_text(wa: &WhatsApp, to: &str, text: &str) -> std::result::Result<String, String> {
    let text: String = text.chars().take(TEXT_MAX).collect();
    let v = graph_post(
        &wa.token,
        &format!("{}/messages", wa.phone_id),
        json!({ "messaging_product": "whatsapp", "recipient_type": "individual", "to": to,
                "type": "text", "text": { "preview_url": false, "body": text } }),
    )
    .await?;
    Ok(v["messages"][0]["id"].as_str().unwrap_or("").to_string())
}

pub async fn instagram_text(ig: &Instagram, to: &str, text: &str) -> std::result::Result<String, String> {
    let v = graph_post(
        &ig.token,
        &format!("{}/messages", ig.user_id),
        json!({ "recipient": { "id": to }, "message": { "text": text } }),
    )
    .await?;
    Ok(v["message_id"].as_str().unwrap_or("").to_string())
}

/// A photo post. Instagram has no text-only posts: the caller supplies a
/// PUBLIC image URL, which the venue's dish photos are.
pub async fn instagram_publish(ig: &Instagram, image_url: &str, caption: &str) -> std::result::Result<String, String> {
    let container = graph_post(
        &ig.token,
        &format!("{}/media", ig.user_id),
        json!({ "image_url": image_url, "caption": caption }),
    )
    .await?;
    let Some(creation_id) = container["id"].as_str() else {
        return Err("Instagram returned no container id".into());
    };
    let published = graph_post(
        &ig.token,
        &format!("{}/media_publish", ig.user_id),
        json!({ "creation_id": creation_id }),
    )
    .await?;
    Ok(published["id"].as_str().unwrap_or("").to_string())
}

// ── the webhook ─────────────────────────────────────────────────────────────

/// `GET /api/webhooks/meta` — Meta's subscription handshake.
pub async fn webhook_verify(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let q = |k: &str| url.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.into_owned());
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    let want = settings.known("notify.whatsapp.verify");
    match (q(HUB_VERIFY_PARAM), q(HUB_CHALLENGE_PARAM)) {
        (Some(got), Some(challenge))
            if !want.trim().is_empty() && ct_eq(got.as_bytes(), want.trim().as_bytes()) =>
        {
            Response::ok(challenge)
        }
        _ => Response::error("verify token mismatch", 403),
    }
}

fn signature_ok(secret: &str, header: Option<String>, raw: &[u8]) -> bool {
    let Some(h) = header else { return false };
    let Some(hex_sig) = h.strip_prefix(SIGNATURE_PREFIX) else { return false };
    let mut m = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
    m.update(raw);
    let want: String = m.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect();
    // CONSTANT TIME. `want == hex_sig` short-circuits on the first byte that
    // differs, so the time to answer measures how much of a forged signature
    // was right -- which is a way to find the rest of it, one byte at a time,
    // from outside. `stripe.rs` already compares this way and says why;
    // `subtle` is already a dependency of this crate.
    ct_eq(want.as_bytes(), hex_sig.as_bytes())
}

/// Equal, without telling anyone WHERE two byte strings first differ.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    // The LENGTH is not a secret (a signature's length is fixed and public);
    // the bytes are.
    if a.len() != b.len() {
        return false;
    }
    use subtle::ConstantTimeEq;
    a.ct_eq(b).into()
}

struct Inbound {
    channel: Channel,
    peer: String,
    peer_name: Option<String>,
    text: String,
    external_id: String,
    at_ms: i64,
}

/// Every message in one delivery, whatever object it came from.
fn inbound_of(body: &Value) -> Vec<Inbound> {
    let mut out = Vec::new();
    for entry in body["entry"].as_array().into_iter().flatten() {
        // WhatsApp Business Account: entry.changes[].value.messages[]
        for change in entry["changes"].as_array().into_iter().flatten() {
            let v = &change["value"];
            if v["messaging_product"].as_str() != Some("whatsapp") {
                continue;
            }
            let names: Vec<(String, String)> = v["contacts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| Some((c["wa_id"].as_str()?.to_string(), c["profile"]["name"].as_str()?.to_string())))
                .collect();
            for m in v["messages"].as_array().into_iter().flatten() {
                let Some(from) = m["from"].as_str() else { continue };
                let kind = m["type"].as_str().unwrap_or("text");
                let text = match kind {
                    "text" => m["text"]["body"].as_str().unwrap_or("").to_string(),
                    "button" => m["button"]["text"].as_str().unwrap_or("").to_string(),
                    "interactive" => m["interactive"]["button_reply"]["title"]
                        .as_str()
                        .or(m["interactive"]["list_reply"]["title"].as_str())
                        .unwrap_or("")
                        .to_string(),
                    "location" => format!(
                        "📍 {},{}",
                        m["location"]["latitude"].as_f64().unwrap_or(0.0),
                        m["location"]["longitude"].as_f64().unwrap_or(0.0)
                    ),
                    other => format!("[{other}]"),
                };
                let at_ms = m["timestamp"].as_str().and_then(|t| t.parse::<i64>().ok()).map(|s| s * 1000).unwrap_or_else(now_ms);
                out.push(Inbound {
                    channel: Channel::WhatsApp,
                    peer: from.to_string(),
                    peer_name: names.iter().find(|(id, _)| id == from).map(|(_, n)| n.clone()),
                    text,
                    external_id: m["id"].as_str().unwrap_or("").to_string(),
                    at_ms,
                });
            }
        }
        // Instagram: entry.messaging[] with sender.id and message.text
        for m in entry["messaging"].as_array().into_iter().flatten() {
            if m["message"]["is_echo"].as_bool().unwrap_or(false) {
                continue;
            }
            let Some(from) = m["sender"]["id"].as_str() else { continue };
            let Some(text) = m["message"]["text"].as_str() else { continue };
            out.push(Inbound {
                channel: Channel::Instagram,
                peer: from.to_string(),
                peer_name: None,
                text: text.to_string(),
                external_id: m["message"]["mid"].as_str().unwrap_or("").to_string(),
                at_ms: m["timestamp"].as_i64().unwrap_or_else(now_ms),
            });
        }
    }
    out
}

/// The table, guaranteed here rather than only by `migrations/0007`: the
/// migration is the record and applies on the next `d1 migrations apply`,
/// but a webhook must not lose a customer's message to a step nobody ran
/// yet. `IF NOT EXISTS` makes this a no-op once either has happened.
const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS channel_messages (id TEXT PRIMARY KEY, location_id TEXT NOT NULL, \
     channel TEXT NOT NULL CHECK (channel IN ('whatsapp','instagram')), \
     direction TEXT NOT NULL CHECK (direction IN ('in','out')), peer TEXT NOT NULL, peer_name TEXT, \
     text TEXT NOT NULL, external_id TEXT, at_ms INTEGER NOT NULL, read_ms INTEGER)",
    "CREATE INDEX IF NOT EXISTS channel_messages_thread ON channel_messages (location_id, channel, peer, at_ms)",
    "CREATE INDEX IF NOT EXISTS channel_messages_recent ON channel_messages (location_id, at_ms DESC)",
];

async fn ensure_schema(db: &D1Database) -> Result<()> {
    for sql in SCHEMA {
        db.prepare(*sql).run().await?;
    }
    Ok(())
}

async fn store(db: &D1Database, venue: &str, direction: &str, m: &Inbound) -> Result<bool> {
    let id = if m.external_id.is_empty() {
        format!("{}:{}:{}", m.channel.as_str(), m.peer, m.at_ms)
    } else {
        format!("{}:{}", m.channel.as_str(), m.external_id)
    };
    let r = db
        .prepare(
            "INSERT OR IGNORE INTO channel_messages \
             (id, location_id, channel, direction, peer, peer_name, text, external_id, at_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(&[
            id.into(),
            venue.into(),
            m.channel.as_str().into(),
            direction.into(),
            m.peer.clone().into(),
            m.peer_name.clone().map(JsValue::from).unwrap_or(JsValue::NULL),
            m.text.clone().into(),
            m.external_id.clone().into(),
            JsValue::from_f64(m.at_ms as f64),
        ])?
        .run()
        .await?;
    Ok(r.meta().ok().flatten().and_then(|m| m.changes).unwrap_or(0) > 0)
}

/// `POST /api/webhooks/meta` — a delivery. Stored, then the owner is told
/// on Telegram when that bell is set, so a WhatsApp question does not wait for
/// the next glance at the console.
pub async fn webhook(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // ── WHAT AN UNSIGNED REQUEST IS ALLOWED TO COST ──
    //
    // This URL is public and takes no credential, so its cost per request is
    // its exposure. A missing signature header is settled BEFORE the body is
    // read and before the venue's settings image is fetched: a request with no
    // signature cannot be from Meta, and answering it used to cost a Durable
    // Object read and -- for the default venue, which has no Meta secret -- a
    // D1 row in `worker_errors` per request.
    let signature = req.headers().get(SIGNATURE_HEADER).ok().flatten();
    if signature.is_none() {
        return Response::error("unsigned", 401);
    }
    let raw = req.bytes().await?;
    // A WEBHOOK MUST NOT 500 ON A URL THAT NAMES NO VENUE. Meta retries a 5xx
    // for days; this answers once, says what is wrong, and stops. The venue is
    // named by the HOST here -- `sushi-durres.dowiz.org/api/webhooks/meta` --
    // and the platform's apex names none.
    let place = match crate::hubstore::Place::of_any(&req, &ctx).await {
        Ok(p) => p,
        Err(e) => {
            console_log!("channels.webhook: no venue in this URL: {e}");
            return Response::from_json(
                &json!({ "stored": 0, "ignored": "this URL does not name a venue" }),
            );
        }
    };
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    // THE SIGNATURE IS THE ONLY AUTHORITY on this URL: without the app secret,
    // anyone could write into the venue's inbox and ring the owner's bell.
    // An unsigned hub acknowledges (Meta would retry a 4xx for days) and drops.
    let Some(secret) = settings.get("notify.meta.secret") else {
        // NOT `loud!`. This is the configuration of a venue that has never set
        // up Meta, not a failure of this delivery, and an unauthenticated
        // caller must not be able to write a row per request into the errors
        // table by pointing at such a venue.
        console_log!("channels.webhook: a delivery was dropped, {} has no Meta app secret", place.venue);
        return Response::from_json(&json!({ "stored": 0, "ignored": "no app secret is set" }));
    };
    if !signature_ok(secret.trim(), signature, &raw) {
        return Response::error("bad signature", 401);
    }
    let body: Value = serde_json::from_slice(&raw).unwrap_or(Value::Null);
    let db = ctx.d1("DB")?;
    ensure_schema(&db).await?;
    let mut stored = 0usize;
    for m in inbound_of(&body) {
        match store(&db, &place.venue, "in", &m).await {
            Ok(true) => {
                stored += 1;
                let chat = settings.known("notify.telegram.chat");
                if let (false, Some(token)) = (chat.trim().is_empty(), crate::notify::bot_token(&ctx.env, &settings)) {
                    let who = m.peer_name.clone().unwrap_or_else(|| m.peer.clone());
                    let text = format!("💬 {} · {who}\n{}", m.channel.as_str(), m.text);
                    if let Err(e) = crate::notify::telegram(&token, chat.trim(), &text).await {
                        crate::loud!(&place.ns, Some(&place.venue), "channels.telegram", "inbox relay refused: {e}");
                    }
                }
            }
            Ok(false) => {}
            Err(e) => {
                crate::loud!(
                    &place.ns,
                    Some(&place.venue),
                    "channels.inbox",
                    "could not store a {} message: {e}",
                    m.channel.as_str()
                )
            }
        }
    }
    // The moment of the last delivery, for the integrations screen: "Meta
    // reached this hub at …" is the fact an owner wants when nothing arrives.
    let stamp = now_ms().to_string();
    let _ = crate::hubstore::with_settings(&place, move |s| { s.set(crate::integrations::WEBHOOK_LAST_KEY, &stamp); Ok(()) }).await;
    Response::from_json(&json!({ "stored": stored }))
}

// ── the owner's inbox ───────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
struct Row {
    channel: String,
    direction: String,
    peer: String,
    peer_name: Option<String>,
    text: String,
    at_ms: i64,
    read_ms: Option<i64>,
}

/// `GET /api/owner/inbox` — one line per conversation, newest first.
pub async fn inbox(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    ensure_schema(&db).await?;
    let rows: Vec<Row> = db
        .prepare(
            "SELECT channel, direction, peer, peer_name, text, at_ms, read_ms FROM channel_messages \
             WHERE location_id = ?1 ORDER BY at_ms DESC LIMIT ?2",
        )
        .bind(&[loc.clone().into(), JsValue::from_f64(RECENT_ROWS as f64)])?
        .all()
        .await?
        .results()?;
    let mut threads: Vec<Value> = Vec::new();
    for r in rows {
        let key = (r.channel.clone(), r.peer.clone());
        if let Some(t) = threads.iter_mut().find(|t| t["channel"] == key.0 && t["peer"] == key.1) {
            if r.direction == "in" && r.read_ms.is_none() {
                t["unread"] = json!(t["unread"].as_u64().unwrap_or(0) + 1);
            }
            if t["name"].is_null() {
                if let Some(n) = r.peer_name.clone() {
                    t["name"] = json!(n);
                }
            }
            continue;
        }
        if threads.len() >= THREADS_SHOWN {
            continue;
        }
        threads.push(json!({
            "channel": r.channel, "peer": r.peer, "name": r.peer_name, "last": r.text,
            "atMs": r.at_ms, "fromThem": r.direction == "in",
            "unread": if r.direction == "in" && r.read_ms.is_none() { 1 } else { 0 },
        }));
    }
    let settings = crate::hubstore::load_settings(&crate::hubstore::Place::of_any(&req, &ctx).await?).await?.settings;
    Response::from_json(&json!({
        "threads": threads,
        "channels": { "whatsapp": whatsapp_cfg(&settings).is_some(), "instagram": instagram_cfg(&settings).is_some() },
        "webhook": format!("{}/api/webhooks/meta", origin_of(&req)),
    }))
}

fn origin_of(req: &Request) -> String {
    req.url().map(|u| u.origin().ascii_serialization()).unwrap_or_default()
}

/// `GET /api/owner/inbox/:peer?channel=` — the thread, and it is marked read.
pub async fn thread(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(peer) = ctx.param("peer").cloned() else { return Response::error("missing peer", 400) };
    let url = req.url()?;
    let channel = url.query_pairs().find(|(k, _)| k == "channel").map(|(_, v)| v.into_owned()).unwrap_or_default();
    if Channel::from_str(&channel).is_none() {
        return Response::error("unknown channel", 400);
    }
    ensure_schema(&db).await?;
    let rows: Vec<Row> = db
        .prepare(
            "SELECT channel, direction, peer, peer_name, text, at_ms, read_ms FROM channel_messages \
             WHERE location_id = ?1 AND channel = ?2 AND peer = ?3 ORDER BY at_ms ASC LIMIT ?4",
        )
        .bind(&[loc.clone().into(), channel.clone().into(), peer.clone().into(), JsValue::from_f64(THREAD_ROWS as f64)])?
        .all()
        .await?
        .results()?;
    db.prepare(
        "UPDATE channel_messages SET read_ms = ?4 WHERE location_id = ?1 AND channel = ?2 AND peer = ?3 \
         AND direction = 'in' AND read_ms IS NULL",
    )
    .bind(&[loc.into(), channel.into(), peer.clone().into(), JsValue::from_f64(now_ms() as f64)])?
    .run()
    .await?;
    let name = rows.iter().find_map(|r| r.peer_name.clone());
    Response::from_json(&json!({
        "peer": peer, "name": name,
        "messages": rows.iter().map(|r| json!({ "fromThem": r.direction == "in", "text": r.text, "atMs": r.at_ms })).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplyIn {
    #[allow(dead_code)]
    location_id: Option<String>,
    channel: String,
    text: String,
}

/// `POST /api/owner/inbox/:peer` — answer, through the channel the customer used.
pub async fn reply(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: ReplyIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let Some(peer) = ctx.param("peer").cloned() else { return Response::error("missing peer", 400) };
    let Some(channel) = Channel::from_str(&body.channel) else { return Response::error("unknown channel", 400) };
    let text = body.text.trim().to_string();
    if text.is_empty() {
        return Response::error("empty message", 400);
    }
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    let sent = match channel {
        Channel::WhatsApp => match whatsapp_cfg(&settings) {
            Some(wa) => whatsapp_text(&wa, &peer, &text).await,
            None => Err("WhatsApp is not set up".into()),
        },
        Channel::Instagram => match instagram_cfg(&settings) {
            Some(ig) => instagram_text(&ig, &peer, &text).await,
            None => Err("Instagram is not set up".into()),
        },
    };
    let external_id = match sent {
        Ok(id) => id,
        Err(e) => return Response::error(e, 502),
    };
    let m = Inbound { channel, peer: peer.clone(), peer_name: None, text, external_id: external_id.clone(), at_ms: now_ms() };
    ensure_schema(&db).await?;
    store(&db, &loc, "out", &m).await?;
    Response::from_json(&json!({ "id": external_id, "atMs": m.at_ms }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whatsapp_delivery_yields_the_text_and_the_name() {
        let body = json!({ "object": "whatsapp_business_account", "entry": [{ "changes": [{ "value": {
            "messaging_product": "whatsapp",
            "contacts": [{ "wa_id": "355691234567", "profile": { "name": "Ana" } }],
            "messages": [{ "from": "355691234567", "id": "wamid.1", "timestamp": "1700000000", "type": "text", "text": { "body": "hi" } }]
        } }] }] });
        let got = inbound_of(&body);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].channel, Channel::WhatsApp);
        assert_eq!(got[0].peer_name.as_deref(), Some("Ana"));
        assert_eq!(got[0].text, "hi");
        assert_eq!(got[0].at_ms, 1_700_000_000_000);
    }

    #[test]
    fn an_instagram_delivery_skips_echoes() {
        let body = json!({ "object": "instagram", "entry": [{ "messaging": [
            { "sender": { "id": "1" }, "timestamp": 5, "message": { "mid": "m1", "text": "hello" } },
            { "sender": { "id": "2" }, "timestamp": 6, "message": { "mid": "m2", "text": "echo", "is_echo": true } }
        ] }] });
        let got = inbound_of(&body);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].channel, Channel::Instagram);
        assert_eq!(got[0].external_id, "m1");
    }

    #[test]
    fn the_signature_is_hmac_sha256_over_the_raw_body() {
        assert!(signature_ok("s", Some("sha256=8f6a0e0a3f5d0a1a0b7b0e2e3e5a4f7c1b4b3c1d9b1f9b2c0c6f6c4c7b9b3d1e".into()), b"x") == false);
        let mut m = Hmac::<Sha256>::new_from_slice(b"s").unwrap();
        m.update(b"body");
        let hex: String = m.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect();
        assert!(signature_ok("s", Some(format!("sha256={hex}")), b"body"));
        assert!(!signature_ok("s", None, b"body"));
    }
}
