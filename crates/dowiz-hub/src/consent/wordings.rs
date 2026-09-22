//! WHAT THE PERSON WAS TOLD — the fourth of the ICO's four questions, and the
//! one a boolean can never answer.
//!
//! The sentence beside the unticked box is part of the consent: a record that
//! cannot produce the exact words is not proof of an INFORMED agreement. So
//! the sentences live here, each is content-addressed by its own hash, and the
//! act names that id.
//!
//! `{venue}` IS SUBSTITUTED WHERE IT IS SHOWN, never stored. Meta requires the
//! business to be named in the opt-in; baking the name into the stored text
//! would give every venue a different id for the same promise, and then no two
//! venues could be held to the same wording.
//!
//! A LANGUAGE WITH NO WORDING HAS NO BOX. Showing an English sentence to a
//! reader who asked for Ukrainian and filing the tick as informed consent is
//! precisely the failure `wording_id`'s empty answer, and `check`'s refusal of
//! a grant without one, exist to stop.

use crate::minijson::esc;

/// The three the storefront speaks (`public/store/i18n.js`). Three languages
/// are THREE wordings: a shared id could not say which was read.
pub const LANGS: [&str; 3] = ["sq", "en", "uk"];

pub const WORDINGS: [(&str, &str); 3] = [
    (
        "sq",
        "{venue} mund të më dërgojë oferta në WhatsApp në këtë numër. \
         Mund ta ndal në çdo kohë duke u përgjigjur me STOP.",
    ),
    (
        "en",
        "{venue} may send me offers on WhatsApp to this number. \
         I can stop at any time by replying STOP.",
    ),
    (
        "uk",
        "{venue} може надсилати мені пропозиції у WhatsApp на цей номер. \
         Я можу зупинити це будь-коли, відповівши STOP.",
    ),
];

/// The sentence for a language, or nothing.
pub fn wording_of(lang: &str) -> Option<(&'static str, &'static str)> {
    WORDINGS.iter().find(|(l, _)| *l == lang).copied()
}

/// The id of a language's wording: the hash of the exact bytes shown.
///
/// CONTENT-ADDRESSED, so editing a sentence mints a NEW wording rather than
/// silently changing what every past consent claims to have said.
pub fn wording_id(lang: &str) -> String {
    let Some((l, text)) = wording_of(lang) else { return String::new() };
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(l.as_bytes());
    h.update(b"\n");
    h.update(text.as_bytes());
    crate::crypto::hex(&h.finalize()[..8])
}

/// The `w` record: the sentence, written once under its own id, so the act
/// that names it can always be shown what it said.
pub fn wording_json(lang: &str) -> String {
    match wording_of(lang) {
        Some((l, text)) => format!(
            "{{\"lang\":\"{}\",\"text\":\"{}\",\"id\":\"{}\"}}",
            esc(l),
            esc(text),
            wording_id(lang)
        ),
        None => String::new(),
    }
}
