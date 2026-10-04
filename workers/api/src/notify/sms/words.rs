//! WHAT AN SMS SAYS, in the customer's language (sq, en, uk, ru). PURE.
//!
//! THE ORDER NUMBER, THE VENUE'S NAME, THE STATE AND HOW TO STOP -- nothing
//! else. No name, no address, no dish, no amount: an SMS sits in a phone's
//! inbox for years, passes the gateway's relay, and is read by whoever holds
//! the phone. The status words are the push words (`push::words`), which are
//! the storefront's own, so a customer reads one vocabulary everywhere.

use crate::notify::push::words;

/// The statuses a customer is texted about. Fewer than push on purpose: every
/// SMS spends the venue's SIM. `READY` only for a pickup (a delivery's next
/// word is "on the way"); `PENDING` is the placement the customer is looking at.
pub fn texts(status: &str, fulfilment: &str) -> bool {
    match status {
        "CONFIRMED" | "IN_DELIVERY" | "REJECTED" | "CANCELLED" => true,
        "READY" => fulfilment == "pickup",
        _ => false,
    }
}

/// How to stop, in the customer's language.
pub fn stop(lang: &str) -> &'static str {
    match lang {
        "sq" => "STOP: thuajini lokalit",
        "uk" => "STOP: скажіть закладу",
        "ru" => "STOP: скажите заведению",
        _ => "STOP: tell the venue",
    }
}

/// The venue's name as the text carries it: one line, at most 30 characters.
pub fn venue_short(name: &str) -> String {
    let one: String = name.split_whitespace().collect::<Vec<_>>().join(" ");
    one.chars().take(30).collect()
}

/// The text, or `None` for a status nobody is texted about.
pub fn text(lang: &str, venue: &str, order_id: &str, status: &str) -> Option<String> {
    let st = words::status(lang, status)?;
    Some(gsm(&format!("{}: {} - {}. {}", venue_short(venue), words::title(lang, order_id), st, stop(lang))))
}

/// ONE SEGMENT, NOT TWO. `ë` and `ç` are not in the GSM-7 alphabet, so one of
/// them turns a 160-character Albanian SMS into a 70-character UCS-2 one --
/// three segments, three SMS off the venue's SIM. Written as Albanian SMS
/// are commonly written: `e`, `c`. Cyrillic stays Cyrillic (UCS-2).
pub fn gsm(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'ë' => 'e',
            'Ë' => 'E',
            'ç' => 'c',
            _ => c,
        })
        .collect()
}

/// How many SMS a text costs: GSM-7 160 (153 per part), else UCS-2 70 (67 per part).
pub fn segments(text: &str) -> usize {
    let n = text.chars().count();
    let (one, part) = if text.is_ascii() { (160, 153) } else { (70, 67) };
    if n <= one { 1 } else { n.div_ceil(part) }
}

/// The test message an owner sends from the console.
pub fn test_text(venue: &str) -> String {
    gsm(&format!("{}: dowiz SMS test OK", venue_short(venue)))
}

#[cfg(test)]
mod tests;
