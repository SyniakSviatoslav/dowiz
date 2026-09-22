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

/// Telegram refuses messages past this many characters; an order with many
/// lines is cut at the lines, not mid-word, and the total always survives.
const TELEGRAM_TEXT_MAX: usize = 4096;
/// A message longer than this is trimmed to its first lines plus the totals.
const LINES_SHOWN_MAX: usize = 30;
/// Currencies whose minor unit does not exist. Mirrors `DECIMALS` in
/// `public/lib/money.js`; every other code renders two decimals.
const ZERO_DECIMAL_CURRENCIES: &[&str] = &["ALL", "JPY"];
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
pub fn money_text(amount: i64, code: &str) -> String {
    if ZERO_DECIMAL_CURRENCIES.contains(&code) {
        format!("{amount} {code}")
    } else {
        let sign = if amount < 0 { "-" } else { "" };
        let a = amount.unsigned_abs();
        format!("{sign}{}.{:02} {code}", a / 100, a % 100)
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
    if !name.is_empty() || !phone.is_empty() {
        out.push_str(&format!("👤 {name} {phone}\n"));
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
mod tests {
    use super::*;

    #[test]
    fn lek_has_no_minor_unit_and_euro_has_two() {
        assert_eq!(money_text(1500, "ALL"), "1500 ALL");
        assert_eq!(money_text(981, "EUR"), "9.81 EUR");
        assert_eq!(money_text(-5, "EUR"), "-0.05 EUR");
    }

    #[test]
    fn order_text_carries_the_total_and_every_line() {
        let env = json!({ "id": "abcdefghijkl", "contact": { "name": "Ana", "phone": "+355" },
            "fulfilment": { "kind": "delivery", "address": { "line": "Rruga 1" } },
            "payment": "cash", "total": 1800, "delivery_fee": 300 });
        let lines = [LineOut { name: "Sake".into(), quantity: 2, unit_price: 750 }];
        let t = order_text(&env, &lines, "ALL", "Dubin & Sushi");
        assert!(t.contains("#abcdefgh"));
        assert!(t.contains("2 × Sake — 1500 ALL"));
        assert!(t.contains("delivery 300 ALL"));
        assert!(t.ends_with("1800 ALL · cash"));
    }

    /// THE KITCHEN'S FIRST LINE IS WHERE IT GOES, and this is the one that
    /// used to be wrong for a whole kind. The branch was `if kind == "pickup"
    /// { 🥡 } else { 🛵 }`, so an order placed at a table printed "delivery"
    /// and sent somebody looking for an address that does not exist.
    #[test]
    fn a_table_order_says_which_table_and_never_says_delivery() {
        let env = json!({ "id": "abcdefghijkl", "contact": { "name": "Ana", "phone": "+355" },
            "fulfilment": { "kind": "dine_in", "table": "7" },
            "payment": "cash", "total": 1500 });
        let lines = [LineOut { name: "Sake".into(), quantity: 2, unit_price: 750 }];
        let t = order_text(&env, &lines, "ALL", "Dubin & Sushi");
        assert!(t.contains("table 7"), "the ticket must name the table: {t}");
        assert!(!t.contains("delivery"), "and must not call it a delivery: {t}");
    }

    /// A pickup is still a pickup. Asserted beside the above because the fix
    /// for one kind is exactly how the other two get broken.
    #[test]
    fn a_pickup_is_unchanged_by_the_third_kind_arriving() {
        let env = json!({ "id": "abcdefghijkl", "contact": { "name": "Ana", "phone": "+355" },
            "fulfilment": { "kind": "pickup" }, "payment": "cash", "total": 1500 });
        let t = order_text(&env, &[], "ALL", "Dubin & Sushi");
        assert!(t.contains("pickup"), "{t}");
        assert!(!t.contains("table"), "{t}");
    }
}
