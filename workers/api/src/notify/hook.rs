//! THE BOT'S WEBHOOK, AND THE CONSOLE'S TELEGRAM ROUTES (W-TG T3).
//!
//! `POST /api/webhooks/telegram` mirrors the Meta webhook (`channels::webhook`):
//! the venue is named by the HOST; the `X-Telegram-Bot-Api-Secret-Token`
//! header is required BEFORE anything is read, and compared with the secret
//! this hub gave `setWebhook`; a venue that never connected acknowledges and
//! drops (Telegram retries a non-2xx), and every answer after the secret
//! check is 200 -- a 5xx would make Telegram re-deliver the same update for
//! hours.

use serde_json::{json, Value};
use worker::*;

use crate::notify::route::groups::{self, Group};

pub mod inbound;
pub mod owner;

/// The header Telegram sets from `setWebhook(secret_token)`.
pub const SECRET_HEADER: &str = "X-Telegram-Bot-Api-Secret-Token";

/// The bot's name and when its webhook was set (`notify.tg.bot`).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Bot {
    pub username: String,
    pub hook_ms: i64,
}

pub fn bot_of(s: &dowiz_hub::settings::Settings) -> Bot {
    s.get(groups::KEY_BOT).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default()
}

/// Constant-time, so the secret cannot be guessed a byte at a time.
pub fn secret_ok(want: &str, given: &str) -> bool {
    use subtle::ConstantTimeEq;
    let (w, g) = (want.trim().as_bytes(), given.trim().as_bytes());
    !w.is_empty() && w.len() == g.len() && bool::from(w.ct_eq(g))
}

/// The groups to CHANGE: a legacy venue's derived groups become stored ones
/// on its first change (the migration is on write).
pub fn editable(s: &dowiz_hub::settings::Settings, lang: &str) -> std::result::Result<Vec<Group>, String> {
    crate::notify::route::groups_of(s, lang).map(|g| g.list)
}

pub fn store(s: &mut dowiz_hub::settings::Settings, list: &[Group]) {
    s.set(groups::KEY_GROUPS, &serde_json::to_string(list).unwrap_or_else(|_| "[]".into()));
}

/// The venue's language, from its record: a new group starts in it.
pub fn venue_lang(record: Option<&Value>) -> String {
    record.and_then(|r| r.get("default_locale")).and_then(Value::as_str).unwrap_or("en").to_string()
}

/// `POST /api/webhooks/telegram`
pub async fn webhook(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(given) = req.headers().get(SECRET_HEADER).ok().flatten() else {
        return Response::error("unsigned", 401);
    };
    let place = match crate::hubstore::Place::of_any(&req, &ctx).await {
        Ok(p) => p,
        Err(e) => {
            console_log!("telegram.webhook: no venue in this URL: {e}");
            return Response::from_json(&json!({ "ignored": "this URL does not name a venue" }));
        }
    };
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    let Some(secret) = settings.get(groups::KEY_SECRET) else {
        // NOT `loud!`: a venue that never connected is configuration, and an
        // unauthenticated caller must not write a row per request.
        return Response::from_json(&json!({ "ignored": "not connected" }));
    };
    if !secret_ok(&secret, &given) {
        return Response::error("bad secret", 401);
    }
    let body: Value = req.json().await.unwrap_or(Value::Null);
    let update = inbound::parse(&body, &bot_of(&settings).username);
    if update == inbound::Update::Ignore {
        return Response::from_json(&json!({ "ok": true }));
    }
    let now = ctx.data.now_ms;
    let record = crate::hubstore::venue_record(&place).await.ok().flatten();
    let lang = venue_lang(record.as_ref());
    let venue = record.as_ref().and_then(|r| r.get("name")).and_then(Value::as_str).unwrap_or(&place.venue).to_string();
    let u = update.clone();
    let effect = crate::hubstore::with_settings(&place, move |s| {
        let mut list = editable(s, &lang).map_err(Error::RustError)?;
        let pending: Option<inbound::Pending> = s.get(groups::KEY_LINK).and_then(|j| serde_json::from_str(&j).ok());
        let e = inbound::apply(&mut list, pending.as_ref(), &u, &lang, &venue, now);
        if e.changed {
            store(s, &list);
        }
        if e.consumed {
            s.clear(groups::KEY_LINK);
        }
        Ok(e)
    })
    .await;
    match effect {
        Ok(e) => {
            if let (Some((to, text)), Some(token)) = (e.reply, crate::notify::bot_token(&ctx.env, &settings)) {
                // One line back, best effort: the link is already written.
                if let Err(f) = crate::notify::tg::send(&token, &to, &text).await {
                    console_log!("telegram.webhook: reply refused: {}", f.words());
                }
            }
        }
        Err(e) => crate::loud!(&place.ns, Some(&place.venue), "telegram.webhook", "update not applied: {e}"),
    }
    Response::from_json(&json!({ "ok": true }))
}

#[cfg(test)]
mod tests;
