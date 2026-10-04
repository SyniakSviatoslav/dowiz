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

/// Every language the storefront speaks (`crate::lang::LANGS`). N languages
/// are N wordings: a shared id could not say which was read.
pub use crate::lang::LANGS;

/// One sentence per language, in `LANGS` order; the array's length IS the
/// language count, so a new language does not compile until it has its words.
pub const WORDINGS: [(&str, &str); LANGS.len()] = [
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
    // DRAFT, not legally reviewed (lane W-RU, 2026-09-27); the operator is the
    // reviewer, as for sq/en/uk (docs/privacy/LEGAL-DECISIONS-2026-09-24.md).
    (
        "ru",
        "{venue} может отправлять мне предложения в WhatsApp на этот номер. \
         Я могу остановить это в любой момент, ответив STOP.",
    ),
];

/// THE SECOND PURPOSE'S SENTENCES (W-MR0 row MR8, operator 2026-10-04: guest taste is scored on the
/// server too, under its OWN consent). Separate from the offers box: one tick, one purpose. Same
/// content addressing, so these ids can never collide with an offers id (the bytes differ).
/// DRAFT, not legally reviewed; the operator is the reviewer, as for the offers sentences.
pub const PERSONALISATION_WORDINGS: [(&str, &str); LANGS.len()] = [
    (
        "sq",
        "{venue} mund të mbajë mend çfarë porosis dhe çfarë më pëlqen në meny, për të më sugjeruar pjata. \
         Kurrë për çmimet. Mund ta tërheq dhe ta fshij në çdo kohë nga faqja e porosisë.",
    ),
    (
        "en",
        "{venue} may remember what I order and what I like on the menu, to suggest dishes to me. \
         Never for prices. I can withdraw and delete it at any time from the order page.",
    ),
    (
        "uk",
        "{venue} може запам\u{2bc}ятовувати, що я замовляю і що мені смакує в меню, щоб пропонувати мені страви. \
         Ніколи для цін. Я можу відкликати це й видалити будь-коли зі сторінки замовлення.",
    ),
    (
        "ru",
        "{venue} может запоминать, что я заказываю и что мне нравится в меню, чтобы предлагать мне блюда. \
         Никогда для цен. Я могу отозвать это и удалить в любой момент со страницы заказа.",
    ),
];

fn id_of(lang: &str, text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(lang.as_bytes());
    h.update(b"\n");
    h.update(text.as_bytes());
    crate::crypto::hex(&h.finalize()[..8])
}

/// The personalisation sentence for a language, and its id.
pub fn personalisation_wording_of(lang: &str) -> Option<(&'static str, &'static str)> {
    PERSONALISATION_WORDINGS.iter().find(|(l, _)| *l == lang).copied()
}
pub fn personalisation_wording_id(lang: &str) -> String {
    personalisation_wording_of(lang).map(|(l, t)| id_of(l, t)).unwrap_or_default()
}
/// The language of a PERSONALISATION sentence id, or `None` (an offers id is not one).
pub fn lang_of_personalisation(id: &str) -> Option<&'static str> {
    PERSONALISATION_WORDINGS.iter().find(|(l, t)| id_of(l, t) == id).map(|(l, _)| *l)
}

/// The `w` record for ANY sentence this build knows, by its id (both purposes).
pub fn wording_json_of_id(id: &str) -> Option<String> {
    WORDINGS
        .iter()
        .chain(PERSONALISATION_WORDINGS.iter())
        .find(|(l, t)| id_of(l, t) == id)
        .map(|(l, t)| format!("{{\"lang\":\"{}\",\"text\":\"{}\",\"id\":\"{}\"}}", esc(l), esc(t), id))
}

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
    id_of(l, text)
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
