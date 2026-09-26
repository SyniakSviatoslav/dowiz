//! A MESSAGE IN THE GROUP'S LANGUAGE, WITH THE GROUP'S AMOUNT OF THE CUSTOMER.
//!
//! LANGUAGES ARE THIS TABLE: a language is spoken when it has a row, and the
//! list the console offers is read from here -- a fifth language is one row,
//! never a `match` on three names somewhere else.
//!
//! THE ORDER TICKET arrives rendered (`notify::order_text`, English, the lines
//! anchored by emoji). A group gets it re-worded line by line -- the anchors
//! are fixed, the dish names are never touched -- and, below `Pii::Fulfil`,
//! without the person: the `👤` (name, phone), `📍` (address) and `📝`
//! (note) lines are dropped.

use serde_json::Value;

use super::groups::Pii;

/// The fixed words of every message, per language.
pub struct Words {
    pub lang: &'static str,
    pub pickup: &'static str,
    pub delivery: &'static str,
    pub table: &'static str,
    pub in_venue: &'static str,
    pub fee: &'static str,
    pub discount: &'static str,
    pub tip: &'static str,
    pub message: &'static str,
    pub low: &'static str,
    pub received: &'static str,
    pub expiring: &'static str,
    pub wasted: &'static str,
    pub variance: &'static str,
    pub status: &'static str,
    pub late: &'static str,
    pub daily: &'static str,
    pub weekly: &'static str,
    pub orders: &'static str,
    pub revenue: &'static str,
    pub top: &'static str,
    pub also: &'static str,
    pub test: &'static str,
    pub gone: &'static str,
}

pub const WORDS: &[Words] = &[
    Words { lang: "en", pickup: "pickup", delivery: "delivery", table: "table", in_venue: "in the venue (no table recorded)",
        fee: "delivery", discount: "discount", tip: "tip", message: "new message - open the console to read it",
        low: "running low", received: "delivery received", expiring: "expiring soon", wasted: "written off",
        variance: "stocktake difference", status: "order", late: "waiting for", daily: "Daily summary", weekly: "Weekly summary",
        orders: "orders", revenue: "revenue", top: "Top dishes", also: "Also", test: "test message: this group is linked",
        gone: "The bot was removed from a group; nothing more goes there until it is linked again:" },
    Words { lang: "sq", pickup: "merr vetë", delivery: "dërgesë", table: "tavolina", in_venue: "në lokal (pa tavolinë)",
        fee: "dërgesa", discount: "zbritje", tip: "bakshish", message: "mesazh i ri - hapeni panelin për ta lexuar",
        low: "po mbaron", received: "mall i ardhur", expiring: "skadon së shpejti", wasted: "u hodh",
        variance: "diferencë numërimi", status: "porosia", late: "pret prej", daily: "Përmbledhja e ditës", weekly: "Përmbledhja e javës",
        orders: "porosi", revenue: "të ardhura", top: "Pjatat kryesore", also: "Gjithashtu", test: "mesazh prove: ky grup është lidhur",
        gone: "Boti u hoq nga një grup; asgjë nuk shkon më atje derisa të lidhet përsëri:" },
    Words { lang: "uk", pickup: "самовивіз", delivery: "доставка", table: "стіл", in_venue: "у закладі (стіл не вказано)",
        fee: "доставка", discount: "знижка", tip: "чайові", message: "нове повідомлення - відкрийте консоль, щоб прочитати",
        low: "закінчується", received: "надійшла партія", expiring: "скоро спливає термін", wasted: "списано",
        variance: "розбіжність інвентаризації", status: "замовлення", late: "чекає вже", daily: "Підсумок дня", weekly: "Підсумок тижня",
        orders: "замовлень", revenue: "виручка", top: "Топ страв", also: "Також", test: "тестове повідомлення: групу підключено",
        gone: "Бота видалили з групи; туди більше нічого не надходить, доки її не підключать знову:" },
    Words { lang: "ru", pickup: "самовывоз", delivery: "доставка", table: "стол", in_venue: "в заведении (стол не указан)",
        fee: "доставка", discount: "скидка", tip: "чаевые", message: "новое сообщение - откройте консоль, чтобы прочитать",
        low: "заканчивается", received: "поступила партия", expiring: "скоро истекает срок", wasted: "списано",
        variance: "расхождение инвентаризации", status: "заказ", late: "ждёт уже", daily: "Итог дня", weekly: "Итог недели",
        orders: "заказов", revenue: "выручка", top: "Топ блюд", also: "Также", test: "тестовое сообщение: группа подключена",
        gone: "Бота удалили из группы; туда больше ничего не приходит, пока её не подключат снова:" },
];

/// The words of `lang`, English when nobody wrote that language.
pub fn words(lang: &str) -> &'static Words {
    WORDS.iter().find(|w| w.lang == lang).unwrap_or(&WORDS[0])
}

pub fn speaks(lang: &str) -> bool {
    WORDS.iter().any(|w| w.lang == lang)
}

pub fn langs() -> Vec<&'static str> {
    WORDS.iter().map(|w| w.lang).collect()
}

/// Lines a group below `Fulfil` never sees: the person.
const PERSON: [&str; 3] = ["👤 ", "📍 ", "📝 "];

/// The ticket, re-worded for `lang` and cut to `pii`. PURE.
pub fn ticket(text: &str, lang: &str, pii: Pii) -> String {
    let en = &WORDS[0];
    let w = words(lang);
    let mut out: Vec<String> = Vec::new();
    for line in text.split('\n') {
        if pii == Pii::None && PERSON.iter().any(|p| line.starts_with(p)) {
            continue;
        }
        let swapped = if line == format!("🥡 {}", en.pickup) {
            format!("🥡 {}", w.pickup)
        } else if line == format!("🛵 {}", en.delivery) {
            format!("🛵 {}", w.delivery)
        } else if line == format!("🍽 {}", en.in_venue) {
            format!("🍽 {}", w.in_venue)
        } else if let Some(t) = line.strip_prefix("🍽 table ") {
            format!("🍽 {} {t}", w.table)
        } else if let Some(rest) = line.strip_prefix("+ #").filter(|r| r.contains(" — table ")) {
            format!("+ #{}", rest.replacen(" — table ", &format!(" — {} ", w.table), 1))
        } else if let Some(v) = line.strip_prefix("delivery ").filter(|v| starts_digit(v)) {
            format!("{} {v}", w.fee)
        } else if let Some(v) = line.strip_prefix("discount −") {
            format!("{} −{v}", w.discount)
        } else if let Some(v) = line.strip_prefix("tip ").filter(|v| starts_digit(v)) {
            format!("{} {v}", w.tip)
        } else {
            line.to_string()
        };
        out.push(swapped);
    }
    out.join("\n")
}

fn starts_digit(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '-')
}

fn s<'a>(d: &'a Value, k: &str) -> &'a str {
    d.get(k).and_then(Value::as_str).unwrap_or("")
}
fn n(d: &Value, k: &str) -> i64 {
    d.get(k).and_then(Value::as_i64).unwrap_or(0)
}

/// One quantity as the kitchen reads it: `1500 g`.
fn qty(d: &Value, k: &str) -> String {
    format!("{} {}", n(d, k), s(d, "unit")).trim().to_string()
}

/// An event with structured data (`{"data": {...}}`), in `lang`. The shapes
/// are the hand-back contract for the stock producers (W-INV):
///   stock.low          {items: [{name, on_hand, low_at, unit}]}
///   stock.received     {name, qty, unit, lot?, expiry?}
///   stock.expiring     {items: [{name, qty, unit, expiry}]}
///   stock.wasted       {name, qty, unit, reason}
///   stocktake.variance {items: [{name, expected, observed, unit}]}
///   order.status       {order, status}
///   order.late         {order, minutes}
/// None of them carries a customer.
pub fn event(ev: &str, d: &Value, lang: &str) -> String {
    let w = words(lang);
    let items = || d.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
    match ev {
        "stock.low" => {
            let rows: Vec<String> = items().iter().map(|i| format!("- {}: {} (≤ {})", s(i, "name"), qty(i, "on_hand"), n(i, "low_at"))).collect();
            format!("📉 {}\n{}", w.low, rows.join("\n"))
        }
        "stock.received" => {
            let mut t = format!("📦 {}: {} {}", w.received, s(d, "name"), qty(d, "qty"));
            for k in ["lot", "expiry"] {
                if !s(d, k).is_empty() {
                    t.push_str(&format!(" · {}", s(d, k)));
                }
            }
            t
        }
        "stock.expiring" => {
            let rows: Vec<String> = items().iter().map(|i| format!("- {}: {} · {}", s(i, "name"), qty(i, "qty"), s(i, "expiry"))).collect();
            format!("⏳ {}\n{}", w.expiring, rows.join("\n"))
        }
        "stock.wasted" => format!("🗑 {}: {} {} · {}", w.wasted, s(d, "name"), qty(d, "qty"), s(d, "reason")),
        "stocktake.variance" => {
            let rows: Vec<String> = items()
                .iter()
                .map(|i| format!("- {}: {} → {} {}", s(i, "name"), n(i, "expected"), n(i, "observed"), s(i, "unit")))
                .collect();
            format!("🧮 {}\n{}", w.variance, rows.join("\n"))
        }
        "order.status" => format!("🔔 {} #{} → {}", w.status, s(d, "order").chars().take(8).collect::<String>(), s(d, "status")),
        "order.late" => format!("⏰ #{} {} {} min", s(d, "order").chars().take(8).collect::<String>(), w.late, n(d, "minutes")),
        _ => s(d, "text").to_string(),
    }
}

/// A customer's message relayed from WhatsApp/Instagram: the words only at
/// `Full`; below it, that one arrived and where.
pub fn inbox(d: &Value, lang: &str, pii: Pii) -> String {
    match pii {
        Pii::Full => format!("💬 {} · {}\n{}", s(d, "channel"), s(d, "peer"), s(d, "text")),
        _ => format!("💬 {} · {}", s(d, "channel"), words(lang).message),
    }
}

/// The numbers of a summary.
pub struct Summary {
    pub venue: String,
    pub orders: i64,
    pub revenue: String,
    pub top: Vec<(String, i64)>,
}

/// A daily or weekly summary: the numbers, then what the group chose to
/// receive "in the summary" since the last one.
pub fn digest(lang: &str, weekly: bool, sum: &Summary, lines: &[String]) -> String {
    let w = words(lang);
    let head = if weekly { w.weekly } else { w.daily };
    let mut t = format!("📊 {head} · {}\n{} {} · {} {}\n", sum.venue, sum.orders, w.orders, w.revenue, sum.revenue);
    if !sum.top.is_empty() {
        t.push_str(&format!("\n{}:\n", w.top));
        for (i, (name, q)) in sum.top.iter().take(5).enumerate() {
            t.push_str(&format!("{}. {name} × {q}\n", i + 1));
        }
    }
    if !lines.is_empty() {
        t.push_str(&format!("\n{}:\n", w.also));
        for l in lines.iter().take(40) {
            t.push_str(&format!("• {}\n", l.replace('\n', " ")));
        }
    }
    t
}

#[cfg(test)]
mod tests;
