//! THE INVITE IS DELIVERED, NOT ONLY SHOWN (W-INVITE, R-GAPS A6/F5, operator
//! 2026-10-10).
//!
//! THE GAP. `invite_courier` answered a link (`invite_link.rs`) and the owner
//! carried it to the courier by hand; `hiring.rs` had 0 outbox hits. Now the
//! owner names a rail and the link goes out on the VENUE'S OWN rail -- its
//! WhatsApp number or its Telegram bot (`notify::bot_token`, the venue's
//! setting first) -- as ONE OUTBOX ENTRY of an existing kind. There is no
//! second sender: `outbox/rails.rs` drains `whatsapp` and `telegram` already,
//! with its retries, its parking, its pacing and its abandon lines.
//!
//! NO RAIL, NO ENTRY. A venue with neither set up queues nothing: the answer
//! says `noRail` and the link stays, so the owner's copy/share path is what is
//! left. A rail that disappears after the entry was queued PARKS it (the
//! drain's own rule), and the list says so.
//!
//! THE STATE, READ BACK. The invite record keeps a MARK (`via`, the outbox id,
//! when) and the list turns it into a state: in the queue = `queued` /
//! `retrying` / `parked`; gone from it with an `outbox.abandoned` line in the
//! venue's error log naming it after the mark = `failed`; gone without one =
//! `sent` (HANDED TO the provider -- WhatsApp's own delivered/read webhook is
//! keyed by a message id the generic drain does not keep); the log unreadable
//! = `unknown`, never `sent`.
//!
//! ONE ENTRY PER PHONE (`courier-invite/<16 hex of the phone digest>`): a
//! re-invite replaces the pending send -- the replaced code is revoked anyway --
//! and a withdrawn invite's pending send is removed. The entry holds the link
//! and so the code, in the venue's own outbox, until it is sent; that is the
//! price of delivering it, and it is why nothing is queued without a rail.

use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use crate::identity_store as ids;
use crate::outbox::Entry;

/// Every invite's outbox id starts with this.
pub const ID_PREFIX: &str = "courier-invite/";
/// The two rails: the outbox kinds `outbox/rails.rs` already drains.
pub const WHATSAPP: &str = "whatsapp";
pub const TELEGRAM: &str = "telegram";
/// Where the drain says what it gave up on (`outbox::rails::drain_venue`).
const ABANDONED: &str = "outbox.abandoned";
/// How many abandon lines are read back: more than a week of a busy venue's.
const LINES: usize = 200;

/// The outbox id for one phone.
pub fn outbox_id(phone_hash: &str) -> String {
    format!("{ID_PREFIX}{}", phone_hash.get(..16).unwrap_or(phone_hash))
}

/// PURE. What the owner asked for, as `(kind, to)`, or the refusal -- checked
/// BEFORE a code is minted. `None` = no send asked. WhatsApp goes to the phone
/// typed for the invite, as the digits the Cloud API takes; Telegram goes to a
/// CHAT ID the owner names (a bot cannot write to a phone number, and only to
/// a person who has pressed Start on it).
pub fn ask(via: Option<&str>, phone: &str, telegram: Option<&str>) -> std::result::Result<Option<(&'static str, String)>, &'static str> {
    match via.map(str::trim) {
        None | Some("") => Ok(None),
        Some(WHATSAPP) => Ok(Some((WHATSAPP, phone.chars().filter(char::is_ascii_digit).collect()))),
        Some(TELEGRAM) => {
            let chat = telegram.unwrap_or("").trim();
            let digits = chat.strip_prefix('-').unwrap_or(chat);
            if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
                return Err("a Telegram chat id is digits: the courier presses Start on your bot and sends you the number it shows");
            }
            Ok(Some((TELEGRAM, chat.to_string())))
        }
        Some(_) => Err("an invite is sent by whatsapp or telegram"),
    }
}

/// PURE. The message, in the owner's console language (English for any other),
/// the link last. The same sentence the console's share sheet uses.
pub fn text(lang: Option<&str>, url: &str) -> String {
    let words = match lang.unwrap_or("sq") {
        "sq" => "Ftesë për të dërguar porosi për ne. Hapni lidhjen dhe zgjidhni një fjalëkalim.",
        "uk" => "Запрошення доставляти замовлення для нас. Відкрийте посилання та придумайте пароль.",
        "ru" => "Приглашение доставлять заказы для нас. Откройте ссылку и придумайте пароль.",
        _ => "An invitation to deliver for us. Open the link and choose a password.",
    };
    format!("{words} {url}")
}

/// PURE. The state the owner reads, from the invite's mark, the venue's queue
/// and its abandon lines `(atMs, message)`; `failed = None` = unreadable.
pub fn state(mark: Option<&Value>, waiting: &[Entry], failed: Option<&[(i64, String)]>) -> Value {
    let Some(m) = mark.filter(|m| m.is_object()) else { return Value::Null };
    let (via, id, at) = (ids::s_of(m, "via"), ids::s_of(m, "outbox"), ids::i_of(m, "atMs"));
    let mut out = json!({ "via": via, "atMs": at });
    if let Some(e) = waiting.iter().find(|e| e.id == id) {
        out["state"] = json!(if e.parked_ms.is_some() { "parked" } else if e.tries > 0 { "retrying" } else { "queued" });
        out["tries"] = json!(e.tries);
        return out;
    }
    // THIS entry's line: the id and then the separator the drain writes, so
    // `courier-invite/abc` never reads `courier-invite/abcd`'s failure.
    let named = |msg: &str| msg.strip_prefix(&id).is_some_and(|rest| rest.starts_with(' ') || rest.starts_with(':'));
    let line = failed.map(|lines| lines.iter().find(|(when, msg)| *when >= at && named(msg)));
    out["state"] = json!(match line {
        None => "unknown",
        Some(None) => "sent",
        Some(Some(_)) => "failed",
    });
    if let Some(Some((_, msg))) = line {
        out["why"] = json!(msg);
    }
    out
}

/// Which rails this venue can send on: the drain's own tests of them.
async fn rails(env: &Env, place: &crate::hubstore::Place) -> Result<(bool, bool)> {
    let settings = crate::hubstore::load_settings(place).await?.settings;
    Ok((crate::channels::whatsapp_cfg(&settings).is_some(), crate::notify::bot_token(env, &settings).is_some()))
}

/// Queue the invite on the asked rail, and mark the invite. Never fails the
/// invite itself: the code is minted and answered; the delivery says how it went.
#[allow(clippy::too_many_arguments)]
pub async fn send(ctx: &RouteContext<crate::Req>, loc: &str, invite: &str, phone_hash: &str, url: Option<&str>, to: Option<(&'static str, String)>, lang: Option<&str>, now: i64) -> Value {
    let Some((via, to)) = to else { return Value::Null };
    let Some(url) = url else { return json!({ "via": via, "state": "noLink" }) };
    let place = match crate::hubstore::Place::of_authorised(ctx, loc) {
        Ok(p) => p,
        Err(e) => return json!({ "via": via, "state": "error", "why": e.to_string() }),
    };
    match rails(&ctx.env, &place).await {
        Ok((wa, tg)) if (via == WHATSAPP && wa) || (via == TELEGRAM && tg) => {}
        Ok(_) => return json!({ "via": via, "state": "noRail" }),
        Err(e) => return json!({ "via": via, "state": "error", "why": e.to_string() }),
    }
    let id = outbox_id(phone_hash);
    // THE ENTRY FIRST, THE MARK SECOND: a mark without an entry would read as
    // `sent`; an entry without a mark only under-claims.
    if let Err(e) = crate::outbox::enqueue(&place, &[Entry::new(id.clone(), via, to, text(lang, url), now)]).await {
        return json!({ "via": via, "state": "error", "why": e.to_string() });
    }
    let mark = json!({ "via": via, "outbox": id, "atMs": now });
    let (iid, l2, ph, m) = (invite.to_string(), loc.to_string(), phone_hash.to_string(), mark.clone());
    let marked = ids::with_couriers(&ctx.env, move |t| {
        let Some(mut i) = ids::rec(t, ids::K_INVITE, &iid) else { return Ok(()) };
        i["delivery"] = m.clone();
        let index = [(ids::invite_by_phone(&ph), iid.clone()), (ids::invite_at(&l2, &iid), iid.clone())];
        t.put(ids::K_INVITE, &iid, &i.to_string(), &index, &[]).map_err(|e| Error::RustError(format!("invite: {e}")))
    })
    .await;
    if let Err(e) = marked {
        crate::loud!(&place.ns, Some(loc), "courier.invite", "queued, but the invite was not marked: {e}");
    }
    json!({ "via": via, "state": "queued", "atMs": now })
}

/// A withdrawn invite's pending send leaves the queue. Best effort: the code is
/// already revoked, so a send that slipped through carries a dead link.
pub async fn withdraw(ctx: &RouteContext<crate::Req>, loc: &str, invite: &str) {
    let Ok(crew) = ids::couriers(&ctx.env).await else { return };
    let Some(mark) = ids::rec(&crew, ids::K_INVITE, invite).and_then(|i| i.get("delivery").cloned()) else { return };
    let id = ids::s_of(&mark, "outbox");
    let Ok(place) = crate::hubstore::Place::of_authorised(ctx, loc) else { return };
    let gone = crate::hubstore::with_table(&place, crate::outbox::IMAGE_OUTBOX, crate::outbox::OUTBOX_BYTES, move |t| {
        t.remove(crate::outbox::KIND, &id);
        Ok(())
    })
    .await;
    if let Err(e) = gone {
        crate::loud!(&place.ns, Some(loc), "courier.invite", "withdrawn, but its pending send was not removed: {e}");
    }
}

/// The courier list's half: turn each row's `delivery` MARK into a state, and
/// answer which rails the venue has. Reads the queue and the abandon lines only
/// when some invite was sent.
pub async fn decorate(ctx: &RouteContext<crate::Req>, loc: &str, rows: &mut [(i64, Value)]) -> Value {
    let Ok(place) = crate::hubstore::Place::of_authorised(ctx, loc) else { return Value::Null };
    let rails = match rails(&ctx.env, &place).await {
        Ok((wa, tg)) => json!({ "whatsapp": wa, "telegram": tg }),
        Err(e) => json!({ "error": e.to_string() }),
    };
    if rows.iter().all(|(_, r)| r["delivery"].is_null()) {
        return rails;
    }
    // AN UNREADABLE QUEUE IS NOT AN EMPTY ONE: then every state reads `unknown`.
    let (waiting, failed) = match crate::outbox::waiting(&place).await {
        Ok(w) => (w, abandoned(&place).await),
        Err(_) => (Vec::new(), None),
    };
    for (_, row) in rows.iter_mut() {
        let mark = row["delivery"].take();
        row["delivery"] = state(Some(&mark), &waiting, failed.as_deref());
    }
    rails
}

/// The venue's `outbox.abandoned` lines, `(atMs, message)`; `None` = unreadable.
async fn abandoned(place: &crate::hubstore::Place) -> Option<Vec<(i64, String)>> {
    let stub = place.stub().ok()?;
    let loaded = crate::platform_store::load_log_at(&stub, crate::errlog::image_of(Some(&place.venue))).await.ok()?;
    Some(
        loaded
            .log
            .about(crate::errlog::KIND, Some(ABANDONED), LINES)
            .into_iter()
            .filter_map(|e| serde_json::from_str::<Value>(&e.json).ok())
            .map(|v| (ids::i_of(&v, "atMs"), ids::s_of(&v, "message")))
            .collect(),
    )
}

#[cfg(test)]
#[path = "invite_send/tests.rs"]
mod tests;
