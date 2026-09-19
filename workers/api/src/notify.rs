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
    out.push_str(if kind == "pickup" { "🥡 pickup\n" } else { "🛵 delivery\n" });
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

/// Tell the owner. Never fails the caller: the order is already placed.
pub async fn order_placed(
    env: &Env,
    place: &crate::hubstore::Place,
    envelope: &Value,
    lines: &[LineOut],
    currency: &str,
    venue: &str,
) {
    let settings = match crate::hubstore::load_settings(place).await {
        Ok(l) => l.settings,
        Err(e) => {
            console_error!("notify: settings unreadable: {e}");
            return;
        }
    };
    let text = order_text(envelope, lines, currency, venue);
    let chat = settings.known("notify.telegram.chat");
    let chat = chat.trim();
    if !chat.is_empty() {
        match bot_token(env, &settings) {
            Some(token) => {
                if let Err(e) = telegram(&token, chat, &text).await {
                    console_error!("notify: telegram refused for {}: {e}", place.venue);
                }
            }
            None => console_error!("notify: chat is set but no bot token exists for {}", place.venue),
        }
    }
    if let Some(wa) = crate::channels::whatsapp_cfg(&settings) {
        if !wa.to.is_empty() {
            if let Err(e) = crate::channels::whatsapp_text(&wa, &wa.to, &text).await {
                console_error!("notify: whatsapp refused for {}: {e}", place.venue);
            }
        }
    }
}

/// `POST /api/owner/notify/test` — send one line to the configured chat and
/// report Telegram's verdict, so the owner learns on the spot whether the
/// bell works rather than at the first missed order.
pub async fn test(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // `owner_and_venue` yields the venue's id; the message names the venue by
    // it, which is what the owner sees in the console's footer too.
    let venue = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
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
    let any_ok = telegram_v == json!("ok") || whatsapp_v == json!("ok");
    let mut res = Response::from_json(&json!({ "ok": any_ok, "telegram": telegram_v, "whatsapp": whatsapp_v }))?;
    if !any_ok {
        res = res.with_status(502);
    }
    Ok(res)
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
}
