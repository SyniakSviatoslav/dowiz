//! THE CONSOLE'S TELEGRAM ROUTES (W-TG T3/T5). Owner only, one venue each.
//!
//!   GET  /api/owner/telegram          the bot, the groups with their health, the event catalogue
//!   POST /api/owner/telegram/connect  {token?}  check the bot, set its webhook to this venue
//!   POST /api/owner/telegram/link     mint the one-time code a group is linked with
//!   POST /api/owner/telegram/group    {id, patch}  language, customer data, matrix, quiet hours
//!   POST /api/owner/telegram/test     {id}  one line to that group, Telegram's verdict back
//!   POST /api/owner/telegram/unlink   {id}
//!
//! THE TOKEN IS NEVER RETURNED OR LOGGED. `connect` uses the VENUE'S OWN bot
//! only -- never the platform's fallback, whose webhook is not one venue's to set.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use super::{bot_of, editable, inbound, store, venue_lang, Bot};
use crate::notify::route::{events, groups, health::Health, render, HEALTH_KIND};
use crate::owner::owner_and_venue;

const TOKEN_KEY: &str = "notify.telegram.token";

fn fail(msg: impl Into<String>) -> Result<Response> {
    Response::error(msg.into(), 400)
}

fn random(n: usize) -> Result<Vec<u8>> {
    let mut b = vec![0u8; n];
    getrandom::getrandom(&mut b).map_err(|e| Error::RustError(format!("no entropy: {e}")))?;
    Ok(b)
}

/// `GET /api/owner/telegram`
pub async fn state(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let s = crate::hubstore::load_settings(&place).await?.settings;
    let record = crate::hubstore::venue_record(&place).await.ok().flatten();
    let gs = match crate::notify::route::groups_of(&s, &venue_lang(record.as_ref())) {
        Ok(g) => g,
        Err(e) => return fail(e),
    };
    let outbox = crate::hubstore::load_table(&place, crate::outbox::IMAGE_OUTBOX, crate::outbox::OUTBOX_BYTES).await?.table;
    let waiting: Vec<crate::outbox::Entry> =
        outbox.all(crate::outbox::KIND).into_iter().filter_map(|(_, j)| serde_json::from_str(&j).ok()).collect();
    let now = ctx.data.now_ms;
    let pending: Option<inbound::Pending> =
        s.get(groups::KEY_LINK).and_then(|j| serde_json::from_str(&j).ok()).filter(|p: &inbound::Pending| p.exp_ms >= now);
    let bot = bot_of(&s);
    let list: Vec<Value> = gs
        .list
        .iter()
        .map(|g| {
            let target = g.target();
            let h = Health::read(outbox.get(HEALTH_KIND, &target).as_deref());
            let mut v = serde_json::to_value(g).unwrap_or(Value::Null);
            // The linker's Telegram id stays in the image; the console needs only when.
            v["linked"] = json!(g.linked.as_ref().map(|l| l.at_ms));
            v["health"] = json!({ "ok_ms": h.ok_ms, "err_ms": h.err_ms, "err": h.err, "failing": h.failing() });
            v["waiting"] = json!(waiting.iter().filter(|e| e.to == target).count());
            v
        })
        .collect();
    Response::from_json(&json!({
        "tokenSet": s.get(TOKEN_KEY).is_some(),
        "bot": if bot.username.is_empty() { Value::Null } else { json!({ "username": bot.username, "hook_ms": bot.hook_ms }) },
        "legacy": gs.legacy,
        "groups": list,
        "pending": pending.map(|p| json!({ "code": p.code, "exp_ms": p.exp_ms })),
        "events": events::EVENTS.iter().map(|e| json!({
            "key": e.key, "area": e.area.as_str(), "live": e.live,
            "scheduled": e.class == events::Class::Scheduled, "urgent": e.class == events::Class::Urgent,
        })).collect::<Vec<_>>(),
        "langs": render::langs(),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectIn {
    #[serde(default)]
    token: String,
}

/// `POST /api/owner/telegram/connect`
pub async fn connect(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: ConnectIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return fail(format!("bad request body: {e}")),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    // THE WEBHOOK NAMES THIS VENUE BY ITS HOST, so the console must be open on it.
    let Some(slug) = crate::hubstore::Place::slug_of_host(&req, &ctx) else {
        return fail("open the console on your venue's own address to connect the bot");
    };
    if crate::hubstore::Place::of_slug(&ctx, &slug).await.map(|p| p.venue).ok().as_deref() != Some(loc.as_str()) {
        return fail("this address belongs to another venue");
    }
    let s = crate::hubstore::load_settings(&place).await?.settings;
    let typed = body.token.trim().to_string();
    let token = if typed.is_empty() { s.get(TOKEN_KEY).map(|t| t.trim().to_string()) } else { Some(typed.clone()) };
    let Some(token) = token.filter(|t| !t.is_empty()) else {
        return fail("paste your bot's token from @BotFather first");
    };
    let me = match crate::notify::tg::call(&token, "getMe", &json!({})).await {
        Ok(v) => v,
        Err(f) => return fail(format!("Telegram refused the token: {}", f.words())),
    };
    let username = me.get("username").and_then(Value::as_str).unwrap_or("").to_string();
    let secret = match s.get(groups::KEY_SECRET) {
        Some(x) => x,
        None => random(24)?.iter().map(|b| format!("{b:02x}")).collect(),
    };
    let url = format!("https://{}/api/webhooks/telegram", req.url()?.host_str().unwrap_or(""));
    let hook = json!({ "url": url, "secret_token": secret, "allowed_updates": ["message", "my_chat_member"] });
    if let Err(f) = crate::notify::tg::call(&token, "setWebhook", &hook).await {
        return fail(format!("Telegram refused the webhook: {}", f.words()));
    }
    let bot = Bot { username: username.clone(), hook_ms: ctx.data.now_ms };
    crate::hubstore::with_settings(&place, move |s| {
        if !typed.is_empty() {
            s.set(TOKEN_KEY, &typed);
        }
        s.set(groups::KEY_SECRET, &secret);
        s.set(groups::KEY_BOT, &serde_json::to_string(&bot).unwrap_or_default());
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true, "bot": username, "webhook": url }))
}

/// `POST /api/owner/telegram/link`
pub async fn link(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let (user, loc) = match owner_and_venue(&req, &ctx).await {
        Ok(x) => x,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let bot = bot_of(&crate::hubstore::load_settings(&place).await?.settings);
    if bot.username.is_empty() {
        return fail("connect the bot first");
    }
    let bytes: [u8; 8] = random(8)?.try_into().unwrap_or([0; 8]);
    let p = inbound::mint(bytes, &user, ctx.data.now_ms);
    let wire = serde_json::to_string(&p).unwrap_or_default();
    crate::hubstore::with_settings(&place, move |s| {
        s.set(groups::KEY_LINK, &wire);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({
        "code": p.code, "exp_ms": p.exp_ms, "bot": bot.username,
        "command": format!("/link@{} {}", bot.username, p.code),
        "deepLink": format!("https://t.me/{}?startgroup={}", bot.username, p.code),
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupIn {
    id: String,
    #[serde(default)]
    patch: groups::Patch,
}

/// `POST /api/owner/telegram/group`
pub async fn group(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: GroupIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return fail(format!("bad request body: {e}")),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let lang = venue_lang(crate::hubstore::venue_record(&place).await.ok().flatten().as_ref());
    let out = crate::hubstore::with_settings(&place, move |s| {
        let mut list = editable(s, &lang).map_err(Error::RustError)?;
        if let Err(e) = groups::apply(&mut list, &body.id, &body.patch) {
            return Ok(Err(e));
        }
        store(s, &list);
        Ok(Ok(list.into_iter().find(|g| g.id == body.id)))
    })
    .await?;
    match out {
        Ok(g) => Response::from_json(&json!({ "ok": true, "group": g })),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdIn {
    id: String,
}

/// `POST /api/owner/telegram/test`
pub async fn test(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: IdIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return fail(format!("bad request body: {e}")),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let s = crate::hubstore::load_settings(&place).await?.settings;
    let list = match editable(&s, "en") {
        Ok(l) => l,
        Err(e) => return fail(e),
    };
    let Some(g) = list.into_iter().find(|g| g.id == body.id) else { return fail(format!("no group {:?}", body.id)) };
    let Some(token) = crate::notify::bot_token(&ctx.env, &s) else { return fail("no bot token is set") };
    let text = format!("✅ dowiz · {loc} — {}", render::words(&g.lang).test);
    // 200 whatever happened: Telegram's words ARE the answer (`notify::test`).
    match crate::notify::tg::send(&token, &g.target(), &text).await {
        Ok(()) => Response::from_json(&json!({ "ok": true })),
        Err(f) => Response::from_json(&json!({ "ok": false, "error": f.words() })),
    }
}

/// `POST /api/owner/telegram/unlink`
pub async fn unlink(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: IdIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return fail(format!("bad request body: {e}")),
    };
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let id = body.id.clone();
    let gone = crate::hubstore::with_settings(&place, move |s| {
        let mut list = editable(s, "en").map_err(Error::RustError)?;
        let Some(i) = list.iter().position(|g| g.id == id) else { return Ok(false) };
        let g = list.remove(i);
        // The old keys are the same chat under another name: cleared with it,
        // so no older path (`notify::test`, the inbox relay) keeps writing there.
        for k in [groups::LEGACY_CHAT, groups::LEGACY_BAR] {
            if s.known(k).trim() == g.chat {
                s.clear(k);
            }
        }
        store(s, &list);
        Ok(true)
    })
    .await?;
    if !gone {
        return fail(format!("no group {:?}", body.id));
    }
    Response::from_json(&json!({ "ok": true }))
}
