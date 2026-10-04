//! THE SMS BOX'S SENTENCE (W-SMS) -- what the customer read beside the
//! unticked "text me about this order" box, content-addressed exactly as the
//! offers sentence is (`wordings.rs`), so the act names the bytes shown.
//!
//! A DIFFERENT PROMISE, A DIFFERENT ID: the hash is taken over `sms\n` + the
//! language + the sentence, so no SMS id can ever equal an offers id, and an
//! act cannot borrow the other box's proof (`log::write` checks the purpose).
//!
//! `{venue}` is substituted where it is shown, never stored.

use super::LANGS;
use crate::minijson::esc;

/// One sentence per language, in `LANGS` order; a new language does not
/// compile until it has its words.
pub const SMS_WORDINGS: [(&str, &str); LANGS.len()] = [
    (
        "sq",
        "{venue} mund të më dërgojë SMS për gjendjen e kësaj porosie në këtë numër. \
         Vetëm numri i porosisë dhe emri i lokalit. Mund ta ndal duke i thënë lokalit STOP.",
    ),
    (
        "en",
        "{venue} may text me about the status of this order at this number. \
         Only the order number and the venue's name. I can stop it by telling the venue STOP.",
    ),
    (
        "uk",
        "{venue} може надсилати мені SMS про стан цього замовлення на цей номер. \
         Лише номер замовлення і назва закладу. Я можу зупинити це, сказавши закладу STOP.",
    ),
    // DRAFT, not legally reviewed, as the offers sentence in Russian is.
    (
        "ru",
        "{venue} может отправлять мне SMS о статусе этого заказа на этот номер. \
         Только номер заказа и название заведения. Я могу остановить это, сказав заведению STOP.",
    ),
];

/// The sentence for a language, or nothing.
pub fn sms_wording_of(lang: &str) -> Option<(&'static str, &'static str)> {
    SMS_WORDINGS.iter().find(|(l, _)| *l == lang).copied()
}

/// The id of a language's SMS sentence: the hash of the exact bytes shown,
/// under its own domain (`sms\n`).
pub fn sms_wording_id(lang: &str) -> String {
    let Some((l, text)) = sms_wording_of(lang) else { return String::new() };
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"sms\n");
    h.update(l.as_bytes());
    h.update(b"\n");
    h.update(text.as_bytes());
    crate::crypto::hex(&h.finalize()[..8])
}

/// The `w` record for an SMS sentence.
pub fn sms_wording_json(lang: &str) -> String {
    match sms_wording_of(lang) {
        Some((l, text)) => format!(
            "{{\"lang\":\"{}\",\"text\":\"{}\",\"id\":\"{}\",\"purpose\":\"order_status\"}}",
            esc(l),
            esc(text),
            sms_wording_id(lang)
        ),
        None => String::new(),
    }
}

/// The language whose SMS sentence has this id.
pub fn lang_of_sms_wording(id: &str) -> Option<&'static str> {
    LANGS.iter().copied().find(|l| sms_wording_id(l) == id)
}

#[cfg(test)]
mod tests;
