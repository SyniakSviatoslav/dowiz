//! The owner's phone buzzes when an order lands.
//!
//! One channel today: a Telegram bot the VENUE owns (see `notify.telegram.*`
//! in `dowiz_hub::settings::KNOWN`), with the platform's bot as a fallback
//! when the operator has set one. A message is sent once, right after the
//! order is in the log, and a failure is written to the console and never to
//! the customer: the order survived, the bell did not, and the console still
//! rings on its next poll.
//!
//! WhatsApp has no free bot API, so it is not here, and the console says so
//! rather than showing a switch that does nothing.

use serde_json::{json, Value};
use worker::*;

use crate::owner::owner_and_venue;
use dowiz_core::money::Currency;

/// Telegram refuses messages past this many characters; an order with many
/// lines is cut at the lines, not mid-word, and the total always survives.
const TELEGRAM_TEXT_MAX: usize = 4096;
/// A message longer than this is trimmed to its first lines plus the totals.
const LINES_SHOWN_MAX: usize = 30;
/// The order id is long; the owner sees the same first characters the console
/// shows (`ORDER_ID_SHOWN` in `admin/core.js`).
const ORDER_ID_SHOWN: usize = 8;

/// A line as the owner reads it: what, how many, for how much.
pub struct LineOut {
    pub name: String,
    pub quantity: i64,
    pub unit_price: i64,
}

/// An integer minor-unit amount as text, e.g. 1500 ALL → "1500 ALL", 981 EUR → "9.81 EUR".
///
/// Uses `Currency::minor_units()` to determine the number of decimal places,
/// so the format is derived from the kernel rather than hardcoded.
pub fn money_text(amount: i64, code: &str) -> String {
    let decimals = Currency::from_code(code)
        .map(|c| c.minor_units() as usize)
        .unwrap_or(if code == "JPY" { 0 } else { 2 }); // outside the kernel's set: the yen had no minor unit before this moved, and still has none

    if decimals == 0 {
        format!("{amount} {code}")
    } else {
        let sign = if amount < 0 { "-" } else { "" };
        let a = amount.unsigned_abs();
        let divisor = 10_u64.pow(decimals as u32);
        let major = a / divisor;
        let minor = a % divisor;
        format!("{sign}{}.{:0width$} {code}", major, minor, width = decimals)
    }
}

/// The venue's own bot token, else the platform's, else nothing.
pub fn bot_token(env: &Env, settings: &dowiz_hub::settings::Settings) -> Option<String> {
    settings
        .get("notify.telegram.token")
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .or_else(|| env.secret("TELEGRAM_BOT_TOKEN").ok().map(|v| v.to_string()))
}

/// One Telegram message. `Err` carries Telegram's own words, which name the
/// fix ("chat not found" means the owner never wrote to the bot).
pub async fn telegram(token: &str, chat: &str, text: &str) -> std::result::Result<(), String> {
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let headers = Headers::new();
    headers.set("content-type", "application/json").map_err(|e| e.to_string())?;
    let text: String = text.chars().take(TELEGRAM_TEXT_MAX).collect();
    let payload = json!({ "chat_id": chat, "text": text, "disable_web_page_preview": true });
    let r = Request::new_with_init(
        &url,
        RequestInit::new()
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(payload.to_string().into())),
    )
    .map_err(|e| e.to_string())?;
    let mut res = Fetch::Request(r).send().await.map_err(|e| e.to_string())?;
    if res.status_code() < 400 {
        return Ok(());
    }
    let body = res.text().await.unwrap_or_default();
    // Telegram answers `{"ok":false,"description":"..."}`; the description is
    // the useful part and the rest is noise on a phone.
    let desc = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| v.get("description").and_then(Value::as_str).map(str::to_string))
        .unwrap_or(body);
    Err(desc.chars().take(200).collect())
}

/// The order as a message. Plain text on purpose: Markdown escaping of dish
/// names and street names is where a notification silently stops arriving.
pub fn order_text(envelope: &Value, lines: &[LineOut], currency: &str, venue: &str) -> String {
    let id: String = envelope
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .chars()
        .take(ORDER_ID_SHOWN)
        .collect();
    let contact = envelope.get("contact").cloned().unwrap_or(Value::Null);
    let name = contact.get("name").and_then(Value::as_str).unwrap_or("").trim().to_string();
    let phone = contact.get("phone").and_then(Value::as_str).unwrap_or("").trim().to_string();
    let ful = envelope.get("fulfilment").cloned().unwrap_or(Value::Null);
    let kind = ful.get("kind").and_then(Value::as_str).unwrap_or("delivery");
    let addr = ful
        .get("address")
        .and_then(|a| a.get("line"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let note = ful.get("note").and_then(Value::as_str).unwrap_or("").trim().to_string();
    let payment = envelope.get("payment").and_then(Value::as_str).unwrap_or("cash");
    let total = envelope.get("total").and_then(Value::as_i64).unwrap_or(0);
    let fee = envelope.get("delivery_fee").and_then(Value::as_i64).unwrap_or(0);
    let discount = envelope.get("discount").and_then(Value::as_i64).unwrap_or(0);
    let tip = envelope.get("tip").and_then(Value::as_i64).unwrap_or(0);

    let mut out = format!("🍣 {venue} — #{id}\n");
    if let Some(who) = ticket_contact(&name, &phone, kind) {
        out.push_str(&format!("👤 {who}\n"));
    }
    // THE KITCHEN'S FIRST LINE IS WHERE IT GOES, and a table order says which
    // table: a ticket reading "🛵 delivery" for somebody sitting in the room
    // sends a courier to look for an address that does not exist.
    out.push_str(match kind {
        "pickup" => "🥡 pickup\n".to_string(),
        "dine_in" => match ful.get("table").and_then(Value::as_str) {
            Some(t) if !t.trim().is_empty() => format!("🍽 table {}\n", t.trim()),
            // A `dine_in` order cannot be PLACED without a table, so this is
            // reachable only by a record older than that rule. It says so
            // rather than printing an empty line.
            _ => "🍽 in the venue (no table recorded)\n".to_string(),
        },
        _ => "🛵 delivery\n".to_string(),
    }.as_str());
    if !addr.is_empty() {
        out.push_str(&format!("📍 {addr}\n"));
    }
    if !note.is_empty() {
        out.push_str(&format!("📝 {note}\n"));
    }
    out.push('\n');
    for l in lines.iter().take(LINES_SHOWN_MAX) {
        out.push_str(&format!(
            "{} × {} — {}\n",
            l.quantity,
            l.name,
            money_text(l.unit_price * l.quantity, currency)
        ));
    }
    if lines.len() > LINES_SHOWN_MAX {
        out.push_str(&format!("… +{}\n", lines.len() - LINES_SHOWN_MAX));
    }
    out.push('\n');
    if fee > 0 {
        out.push_str(&format!("delivery {}\n", money_text(fee, currency)));
    }
    if discount > 0 {
        out.push_str(&format!("discount −{}\n", money_text(discount, currency)));
    }
    if tip > 0 {
        out.push_str(&format!("tip {}\n", money_text(tip, currency)));
    }
    out.push_str(&format!("💰 {} · {payment}", money_text(total, currency)));
    out
}

/// THE PERSON ON A TICKET, MINIMISED (P11, GDPR Art. 5(1)(c)). A ticket goes
/// to the venue's Telegram chat, which keeps it on Telegram's servers for as
/// long as the chat lives -- no erasure reaches it -- so it carries only what
/// the kitchen and the pass need:
///
/// - the NAME the order is called out by: the first name and the initial of
///   the last ("Arben H."), never the whole of what was typed;
/// - the PHONE only for a delivery, where the courier or the kitchen may have
///   to ring the door. A pickup is collected at the counter by the name, and a
///   table order is at a table; neither needs a number in a chat log.
///
/// `None` when nothing is left to print.
pub fn ticket_contact(name: &str, phone: &str, kind: &str) -> Option<String> {
    let words: Vec<&str> = name.split_whitespace().collect();
    let short = match words.as_slice() {
        [] => String::new(),
        [one] => one.to_string(),
        [first, .., last] => {
            let initial: String = last.chars().take(1).collect();
            format!("{first} {initial}.")
        }
    };
    let phone = if kind == "delivery" { phone.trim() } else { "" };
    let out = format!("{short} {phone}").trim().to_string();
    (!out.is_empty()).then_some(out)
}

// `order_placed` WAS HERE, and its going unused is the proof rather than a
// side effect.
//
// It loaded the settings, rendered the text and AWAITED Telegram and WhatsApp
// inline, after the order was already in the log. One attempt, no memory of it:
// a refusal, or an isolate cut off at the end of the response, and the kitchen
// was never told about an order that exists and is paid for. Nothing retried
// and nothing recorded that anything was missed.
//
// The effect is now WRITTEN by the object turn that writes the order
// (`hubdo::enqueue_bell`), so "the order landed" and "the message is owed" are
// one fact, and `outbox::sweep` delivers it with a backoff. `order_text` below
// is what survived: the Worker renders, the object queues, the cron sends.

/// `POST /api/owner/notify/test` — send one line to the configured chat and
/// report Telegram's verdict, so the owner learns on the spot whether the
/// bell works rather than at the first missed order.
pub async fn test(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    // `owner_and_venue` yields the venue's id; the message names the venue by
    // it, which is what the owner sees in the console's footer too.
    let venue = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &venue)?;
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    let text = format!("✅ dowiz · {venue} — notifications work");
    // One verdict per channel, so the owner sees which bell rang. "unset" is
    // not a failure; a channel with a chat but no token is.
    let chat = settings.known("notify.telegram.chat");
    let telegram_v = if chat.trim().is_empty() {
        json!("unset")
    } else {
        match bot_token(&ctx.env, &settings) {
            None => json!({ "error": "no bot token is set" }),
            Some(token) => match telegram(&token, chat.trim(), &text).await {
                Ok(()) => json!("ok"),
                Err(e) => json!({ "error": e }),
            },
        }
    };
    let whatsapp_v = match crate::channels::whatsapp_cfg(&settings) {
        None => json!("unset"),
        Some(wa) if wa.to.is_empty() => json!({ "error": "no number to notify is set" }),
        Some(wa) => match crate::channels::whatsapp_text(&wa, &wa.to, &text).await {
            Ok(_) => json!("ok"),
            Err(e) => json!({ "error": e }),
        },
    };
    // 200 whatever happened: the per-channel verdict IS the answer, and a 502
    // would hide it behind the console's generic "HTTP 502".
    let any_ok = telegram_v == json!("ok") || whatsapp_v == json!("ok");
    Response::from_json(&json!({ "ok": any_ok, "telegram": telegram_v, "whatsapp": whatsapp_v }))
}

#[cfg(test)]
mod tests;
