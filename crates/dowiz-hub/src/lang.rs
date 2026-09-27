//! THE LANGUAGE SET, ONCE (lane W-RU, 2026-09-27; research 2026-09-26 Part B, row B0).
//!
//! Every Rust site that speaks a language — the privacy notice, the DPA, the
//! consent wordings, the erasure promise, the STOP line, the alert and
//! Telegram words, the voice lines, the learn endpoint, the MCP schema, the
//! waitlist mail — takes its set from here. The JS twin is
//! `workers/api/public/lib/langs.js`; `tools/gates/langs.sh` refuses a list of
//! language codes anywhere else and checks the two sets are the same.

/// The UI languages, Albanian first: the venues are in Albania.
pub const LANGS: [&str; 4] = ["sq", "en", "uk", "ru"];

/// `LANGS` as a JSON array literal, for a `concat!` that builds a schema at
/// compile time (the MCP `menu` tool's `lang` enum). A test holds it equal to
/// `LANGS`, so the two cannot drift.
#[macro_export]
macro_rules! langs_json {
    () => {
        r#"["sq","en","uk","ru"]"#
    };
}

/// True for a code of ours ("ru"), false for anything else (incl. "ru-RU").
pub fn is_lang(l: &str) -> bool {
    LANGS.contains(&l)
}

/// A language the films are recorded and captioned in: every UI language but
/// Russian (operator 2026-09-26: the videos stay out of Russian). The JS twin
/// is `MEDIA_LANGS` in lib/langs.js.
pub fn is_media_lang(l: &str) -> bool {
    is_lang(l) && l != "ru"
}

/// The two-letter code a BCP-47 tag starts with, when it is one of ours:
/// "ru-RU" -> Some("ru"), "de" -> None.
pub fn norm(tag: &str) -> Option<&'static str> {
    let head = tag.trim().get(..2)?.to_ascii_lowercase();
    LANGS.iter().copied().find(|l| *l == head)
}

/// `lang` when it is ours (after `norm`), else `fallback`.
pub fn or(lang: &str, fallback: &'static str) -> &'static str {
    norm(lang).unwrap_or(fallback)
}

/// The best of an `Accept-Language` header that is one of ours, honouring
/// q-values (`q=0` means "not this one"); ties keep the header's order.
pub fn from_accept(header: &str) -> Option<&'static str> {
    let mut best: Option<(&'static str, u32)> = None;
    for part in header.split(',') {
        let mut it = part.split(';');
        let Some(l) = norm(it.next().unwrap_or("")) else { continue };
        let q = it
            .filter_map(|p| p.trim().strip_prefix("q="))
            .next()
            .map(q_permille)
            .unwrap_or(1000);
        if q > 0 && best.map_or(true, |(_, b)| q > b) {
            best = Some((l, q));
        }
    }
    best.map(|(l, _)| l)
}

/// A q-value as an integer permille ("0.8" -> 800); junk reads as 0.
fn q_permille(s: &str) -> u32 {
    let s = s.trim();
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let Ok(i) = int.parse::<u32>() else { return 0 };
    let mut f = 0u32;
    for (n, c) in frac.chars().take(3).enumerate() {
        let Some(d) = c.to_digit(10) else { return 0 };
        f += d * 10u32.pow(2 - n as u32);
    }
    (i * 1000 + f).min(1000)
}

/// The page's language: `?lang=` when it is ours, else the best of the
/// browser's `Accept-Language` that is ours, else `fallback`.
pub fn choose(query: Option<&str>, accept: Option<&str>, fallback: &'static str) -> &'static str {
    query
        .and_then(norm)
        .or_else(|| accept.and_then(from_accept))
        .unwrap_or(fallback)
}

/// The second language dish content is read in when `want` has no
/// translation: English, before the venue's own words (research 2026-09-26
/// B2.4, `ru -> en -> venue`). None when English is the one asked for or the
/// venue's own, where the venue's words already are the next step.
pub fn content_fallback(want: &str, venue: &str) -> Option<&'static str> {
    (want != "en" && venue != "en").then_some("en")
}

/// The language's name in English, for a prompt to a model.
pub fn english_name(l: &str) -> &'static str {
    match norm(l) {
        Some("sq") => "Albanian",
        Some("uk") => "Ukrainian",
        Some("ru") => "Russian",
        _ => "English",
    }
}

/// East Slavic plural: forms[0] for 1 (21, 31...), forms[1] for 2-4, forms[2]
/// for the rest. Callers pick this only for uk and ru.
pub fn slavic_plural<'a>(n: u64, forms: [&'a str; 3]) -> &'a str {
    let (m10, m100) = (n % 10, n % 100);
    if m10 == 1 && m100 != 11 {
        forms[0]
    } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
        forms[1]
    } else {
        forms[2]
    }
}

#[cfg(test)]
mod tests;
